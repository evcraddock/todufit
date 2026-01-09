//! Sync-aware meal log repository that reads/writes Automerge documents.
//!
//! This module provides a repository layer that:
//! 1. Uses the user's personal meallogs document when identity is configured
//! 2. Falls back to legacy DocType-based storage otherwise
//! 3. Reads directly from Automerge documents (in-memory queries)

use automerge::AutoCommit;
use chrono::NaiveDate;
use uuid::Uuid;

use todu_fit_core::{DocumentId, MultiDocStorage};

use crate::config::Config;
use crate::models::MealLog;
use crate::sync::group_context::{is_identity_ready, resolve_user_context, GroupContextError};
use crate::sync::reader::{
    list_meallogs_by_date_range, read_all_meallogs, read_meallog_by_id, ReaderError,
};
use crate::sync::storage::{DocType, DocumentStorage, StorageError};
use crate::sync::writer;

/// Error type for sync meal log operations.
#[derive(Debug)]
pub enum SyncMealLogError {
    /// Storage error (loading/saving Automerge docs).
    Storage(StorageError),
    /// Reader error (parsing Automerge data).
    Reader(ReaderError),
    /// MealLog not found.
    NotFound(String),
    /// User context error.
    UserContext(GroupContextError),
    /// Multi-storage error.
    MultiStorage(todu_fit_core::MultiStorageError),
}

impl std::fmt::Display for SyncMealLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncMealLogError::Storage(e) => write!(f, "Storage error: {}", e),
            SyncMealLogError::Reader(e) => write!(f, "Reader error: {}", e),
            SyncMealLogError::NotFound(id) => write!(f, "MealLog not found: {}", id),
            SyncMealLogError::UserContext(e) => write!(f, "{}", e),
            SyncMealLogError::MultiStorage(e) => write!(f, "Storage error: {}", e),
        }
    }
}

impl std::error::Error for SyncMealLogError {}

impl From<StorageError> for SyncMealLogError {
    fn from(e: StorageError) -> Self {
        SyncMealLogError::Storage(e)
    }
}

impl From<ReaderError> for SyncMealLogError {
    fn from(e: ReaderError) -> Self {
        SyncMealLogError::Reader(e)
    }
}

impl From<GroupContextError> for SyncMealLogError {
    fn from(e: GroupContextError) -> Self {
        SyncMealLogError::UserContext(e)
    }
}

impl From<todu_fit_core::MultiStorageError> for SyncMealLogError {
    fn from(e: todu_fit_core::MultiStorageError) -> Self {
        SyncMealLogError::MultiStorage(e)
    }
}

/// Sync-aware meal log repository.
///
/// All operations work directly with Automerge documents.
/// Uses the user's personal meallogs document when identity is configured,
/// otherwise falls back to legacy DocType-based storage.
pub struct SyncMealLogRepository {
    legacy_storage: DocumentStorage,
    multi_storage: MultiDocStorage,
}

#[allow(dead_code)]
impl SyncMealLogRepository {
    /// Creates a new sync meal log repository.
    pub fn new() -> Self {
        Self {
            legacy_storage: DocumentStorage::new(),
            multi_storage: MultiDocStorage::new(Config::default_data_dir()),
        }
    }

    /// Creates a new sync meal log repository with custom storage (for testing).
    #[cfg(test)]
    pub fn with_storage(storage: DocumentStorage) -> Self {
        Self {
            legacy_storage: storage,
            multi_storage: MultiDocStorage::new(Config::default_data_dir()),
        }
    }

    /// Resolves the meallogs document ID.
    /// Returns None if using legacy mode, Some(doc_id) if using identity mode.
    fn resolve_doc_id(&self) -> Option<DocumentId> {
        if !is_identity_ready() {
            return None;
        }

        resolve_user_context().ok().map(|ctx| ctx.meallogs_doc_id)
    }

    /// Loads the meallogs Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<AutoCommit, SyncMealLogError> {
        if let Some(doc_id) = self.resolve_doc_id() {
            // Identity mode: use multi-storage with document ID
            match self.multi_storage.load(&doc_id)? {
                Some(bytes) => AutoCommit::load(&bytes).map_err(|e| {
                    SyncMealLogError::Reader(ReaderError::AutomergeError(e.to_string()))
                }),
                None => Ok(AutoCommit::new()),
            }
        } else {
            // Legacy mode: use DocType-based storage
            match self.legacy_storage.load(DocType::MealLogs)? {
                Some(doc) => Ok(doc),
                None => Ok(AutoCommit::new()),
            }
        }
    }

    /// Saves the document to storage.
    fn save_doc(&self, doc: &mut AutoCommit) -> Result<(), SyncMealLogError> {
        if let Some(doc_id) = self.resolve_doc_id() {
            // Identity mode: use multi-storage
            let bytes = doc.save();
            self.multi_storage.save(&doc_id, &bytes)?;
        } else {
            // Legacy mode
            self.legacy_storage.save(DocType::MealLogs, doc)?;
        }
        Ok(())
    }

    /// Creates a new meal log.
    pub fn create(&self, log: &MealLog) -> Result<MealLog, SyncMealLogError> {
        let mut doc = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_meallog(&mut doc, log);

        // Save
        self.save_doc(&mut doc)?;

        // Return the log (read back to confirm)
        self.get_by_id(log.id)?
            .ok_or_else(|| SyncMealLogError::NotFound(log.id.to_string()))
    }

    /// Updates an existing meal log.
    pub fn update(&self, log: &MealLog) -> Result<MealLog, SyncMealLogError> {
        let mut doc = self.load_or_create_doc()?;

        // Write updated log to Automerge (overwrites existing)
        writer::write_meallog(&mut doc, log);

        // Save
        self.save_doc(&mut doc)?;

        // Return the updated log
        self.get_by_id(log.id)?
            .ok_or_else(|| SyncMealLogError::NotFound(log.id.to_string()))
    }

    /// Deletes a meal log by ID.
    pub fn delete(&self, id: Uuid) -> Result<(), SyncMealLogError> {
        let mut doc = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_meallog(&mut doc, id);

        // Save
        self.save_doc(&mut doc)?;

        Ok(())
    }

    /// Gets a meal log by ID.
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<MealLog>, SyncMealLogError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_meallog_by_id(&doc, id)?)
    }

    /// Lists all meal logs.
    pub fn list(&self) -> Result<Vec<MealLog>, SyncMealLogError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_all_meallogs(&doc)?)
    }

    /// Lists meal logs within a date range.
    pub fn list_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealLog>, SyncMealLogError> {
        let doc = self.load_or_create_doc()?;
        Ok(list_meallogs_by_date_range(&doc, from, to)?)
    }
}

impl Default for SyncMealLogRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Dish, MealType};
    use tempfile::TempDir;

    fn test_repo() -> (SyncMealLogRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        let repo = SyncMealLogRepository::with_storage(storage);
        (repo, temp_dir)
    }

    #[test]
    fn test_create_and_get() {
        let (repo, _temp) = test_repo();

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let log = MealLog::new(date, MealType::Dinner, "chef");
        let created = repo.create(&log).unwrap();

        assert_eq!(created.date, date);
        assert_eq!(created.meal_type, MealType::Dinner);

        let fetched = repo.get_by_id(log.id).unwrap().unwrap();
        assert_eq!(fetched.id, log.id);
    }

    #[test]
    fn test_create_with_dishes() {
        let (repo, _temp) = test_repo();

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let dish1 = Dish::new("Pasta", "chef");
        let dish2 = Dish::new("Salad", "chef");

        let log = MealLog::new(date, MealType::Lunch, "chef")
            .with_dishes(vec![dish1.clone(), dish2.clone()]);
        let created = repo.create(&log).unwrap();

        assert_eq!(created.dishes.len(), 2);
        assert_eq!(created.dishes[0].name, "Pasta");
        assert_eq!(created.dishes[1].name, "Salad");
    }

    #[test]
    fn test_list_range() {
        let (repo, _temp) = test_repo();

        let date1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let date3 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        repo.create(&MealLog::new(date1, MealType::Dinner, "chef"))
            .unwrap();
        repo.create(&MealLog::new(date2, MealType::Dinner, "chef"))
            .unwrap();
        repo.create(&MealLog::new(date3, MealType::Dinner, "chef"))
            .unwrap();

        let from = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let to = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap();
        let logs = repo.list_range(from, to).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].date, date2);
    }

    #[test]
    fn test_update() {
        let (repo, _temp) = test_repo();

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut log = MealLog::new(date, MealType::Dinner, "chef");
        repo.create(&log).unwrap();

        log.notes = Some("Updated notes".to_string());
        let updated = repo.update(&log).unwrap();

        assert_eq!(updated.notes, Some("Updated notes".to_string()));
    }
}
