//! Sync-aware meal log repository that reads/writes Automerge documents.
//!
//! This module provides a repository layer that uses the user's personal
//! meallogs document. Identity must be initialized first.

use std::path::PathBuf;

use automerge::AutoCommit;
use chrono::NaiveDate;
use uuid::Uuid;

use todu_fit_core::{DocumentId, MultiDocStorage};

use crate::models::MealLog;
use crate::sync::group_context::{resolve_user_context, GroupContextError};
use crate::sync::reader::{
    list_meallogs_by_date_range, read_all_meallogs, read_meallog_by_id, ReaderError,
};
use crate::sync::writer;

/// Error type for sync meal log operations.
#[derive(Debug)]
pub enum SyncMealLogError {
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
            SyncMealLogError::Reader(e) => write!(f, "Reader error: {}", e),
            SyncMealLogError::NotFound(id) => write!(f, "MealLog not found: {}", id),
            SyncMealLogError::UserContext(e) => write!(f, "{}", e),
            SyncMealLogError::MultiStorage(e) => write!(f, "Storage error: {}", e),
        }
    }
}

impl std::error::Error for SyncMealLogError {}

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
/// Uses the user's personal meallogs document.
pub struct SyncMealLogRepository {
    storage: MultiDocStorage,
    data_dir: PathBuf,
}

impl SyncMealLogRepository {
    /// Creates a new sync meal log repository.
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            storage: MultiDocStorage::new(data_dir.clone()),
            data_dir,
        }
    }

    /// Resolves the meallogs document ID from user context.
    fn resolve_doc_id(&self) -> Result<DocumentId, SyncMealLogError> {
        let ctx = resolve_user_context(&self.data_dir)?;
        Ok(ctx.meallogs_doc_id)
    }

    /// Loads the meallogs Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<(AutoCommit, DocumentId), SyncMealLogError> {
        let doc_id = self.resolve_doc_id()?;
        let doc = match self.storage.load(&doc_id)? {
            Some(bytes) => AutoCommit::load(&bytes).map_err(|e| {
                SyncMealLogError::Reader(ReaderError::AutomergeError(e.to_string()))
            })?,
            None => AutoCommit::new(),
        };
        Ok((doc, doc_id))
    }

    /// Saves the document to storage.
    fn save_doc(&self, doc: &mut AutoCommit, doc_id: &DocumentId) -> Result<(), SyncMealLogError> {
        let bytes = doc.save();
        self.storage.save(doc_id, &bytes)?;
        Ok(())
    }

    /// Creates a new meal log.
    pub fn create(&self, log: &MealLog) -> Result<MealLog, SyncMealLogError> {
        let (mut doc, doc_id) = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_meallog(&mut doc, log);

        // Save
        self.save_doc(&mut doc, &doc_id)?;

        // Return the log (read back to confirm)
        self.get_by_id(log.id)?
            .ok_or_else(|| SyncMealLogError::NotFound(log.id.to_string()))
    }

    /// Updates an existing meal log.
    #[allow(dead_code)]
    pub fn update(&self, log: &MealLog) -> Result<MealLog, SyncMealLogError> {
        let (mut doc, doc_id) = self.load_or_create_doc()?;

        // Write updated log to Automerge (overwrites existing)
        writer::write_meallog(&mut doc, log);

        // Save
        self.save_doc(&mut doc, &doc_id)?;

        // Return the updated log
        self.get_by_id(log.id)?
            .ok_or_else(|| SyncMealLogError::NotFound(log.id.to_string()))
    }

    /// Deletes a meal log by ID.
    #[allow(dead_code)]
    pub fn delete(&self, id: Uuid) -> Result<(), SyncMealLogError> {
        let (mut doc, doc_id) = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_meallog(&mut doc, id);

        // Save
        self.save_doc(&mut doc, &doc_id)?;

        Ok(())
    }

    /// Gets a meal log by ID.
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<MealLog>, SyncMealLogError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(read_meallog_by_id(&doc, id)?)
    }

    /// Lists all meal logs.
    #[allow(dead_code)]
    pub fn list(&self) -> Result<Vec<MealLog>, SyncMealLogError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(read_all_meallogs(&doc)?)
    }

    /// Lists meal logs within a date range.
    pub fn list_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealLog>, SyncMealLogError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(list_meallogs_by_date_range(&doc, from, to)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Dish, MealType};
    use crate::sync::writer;
    use tempfile::TempDir;

    /// Test helper that bypasses identity requirements.
    struct TestMealLogRepo {
        storage: MultiDocStorage,
        doc_id: DocumentId,
    }

    impl TestMealLogRepo {
        fn new(temp_dir: &TempDir) -> Self {
            let storage = MultiDocStorage::new(temp_dir.path().to_path_buf());
            let doc_id = DocumentId::new();
            Self { storage, doc_id }
        }

        fn load_or_create_doc(&self) -> AutoCommit {
            match self.storage.load(&self.doc_id).unwrap() {
                Some(bytes) => AutoCommit::load(&bytes).unwrap(),
                None => AutoCommit::new(),
            }
        }

        fn save_doc(&self, doc: &mut AutoCommit) {
            let bytes = doc.save();
            self.storage.save(&self.doc_id, &bytes).unwrap();
        }

        fn create(&self, log: &MealLog) -> MealLog {
            let mut doc = self.load_or_create_doc();
            writer::write_meallog(&mut doc, log);
            self.save_doc(&mut doc);
            self.get_by_id(log.id).unwrap()
        }

        fn update(&self, log: &MealLog) -> MealLog {
            let mut doc = self.load_or_create_doc();
            writer::write_meallog(&mut doc, log);
            self.save_doc(&mut doc);
            self.get_by_id(log.id).unwrap()
        }

        fn get_by_id(&self, id: Uuid) -> Option<MealLog> {
            let doc = self.load_or_create_doc();
            read_meallog_by_id(&doc, id).unwrap()
        }

        fn list_range(&self, from: NaiveDate, to: NaiveDate) -> Vec<MealLog> {
            let doc = self.load_or_create_doc();
            list_meallogs_by_date_range(&doc, from, to).unwrap()
        }
    }

    #[test]
    fn test_create_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealLogRepo::new(&temp_dir);

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let log = MealLog::new(date, MealType::Dinner, "chef");
        let created = repo.create(&log);

        assert_eq!(created.date, date);
        assert_eq!(created.meal_type, MealType::Dinner);

        let fetched = repo.get_by_id(log.id).unwrap();
        assert_eq!(fetched.id, log.id);
    }

    #[test]
    fn test_create_with_dishes() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealLogRepo::new(&temp_dir);

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let dish1 = Dish::new("Pasta", "chef");
        let dish2 = Dish::new("Salad", "chef");

        let log = MealLog::new(date, MealType::Lunch, "chef")
            .with_dishes(vec![dish1.clone(), dish2.clone()]);
        let created = repo.create(&log);

        assert_eq!(created.dishes.len(), 2);
        assert_eq!(created.dishes[0].name, "Pasta");
        assert_eq!(created.dishes[1].name, "Salad");
    }

    #[test]
    fn test_list_range() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealLogRepo::new(&temp_dir);

        let date1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let date3 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        repo.create(&MealLog::new(date1, MealType::Dinner, "chef"));
        repo.create(&MealLog::new(date2, MealType::Dinner, "chef"));
        repo.create(&MealLog::new(date3, MealType::Dinner, "chef"));

        let from = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let to = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap();
        let logs = repo.list_range(from, to);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].date, date2);
    }

    #[test]
    fn test_update() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealLogRepo::new(&temp_dir);

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut log = MealLog::new(date, MealType::Dinner, "chef");
        repo.create(&log);

        log.notes = Some("Updated notes".to_string());
        let updated = repo.update(&log);

        assert_eq!(updated.notes, Some("Updated notes".to_string()));
    }
}
