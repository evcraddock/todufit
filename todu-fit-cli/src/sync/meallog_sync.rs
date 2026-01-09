//! Sync-aware meal log repository that reads/writes Automerge documents.
//!
//! This module provides a repository layer that:
//! 1. Writes changes to Automerge documents (source of truth)
//! 2. Reads directly from Automerge documents (in-memory queries)

use automerge::AutoCommit;
use chrono::NaiveDate;
use uuid::Uuid;

use crate::models::MealLog;
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
}

impl std::fmt::Display for SyncMealLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncMealLogError::Storage(e) => write!(f, "Storage error: {}", e),
            SyncMealLogError::Reader(e) => write!(f, "Reader error: {}", e),
            SyncMealLogError::NotFound(id) => write!(f, "MealLog not found: {}", id),
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

/// Sync-aware meal log repository.
///
/// All operations work directly with Automerge documents.
pub struct SyncMealLogRepository {
    storage: DocumentStorage,
}
#[allow(dead_code)]
impl SyncMealLogRepository {
    /// Creates a new sync meal log repository.
    pub fn new() -> Self {
        Self {
            storage: DocumentStorage::new(),
        }
    }

    /// Creates a new sync meal log repository with custom storage.
    #[cfg(test)]
    pub fn with_storage(storage: DocumentStorage) -> Self {
        Self { storage }
    }

    /// Loads the meallogs Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<AutoCommit, SyncMealLogError> {
        match self.storage.load(DocType::MealLogs)? {
            Some(doc) => Ok(doc),
            None => Ok(AutoCommit::new()),
        }
    }

    /// Saves the document to storage.
    fn save_doc(&self, doc: &mut AutoCommit) -> Result<(), SyncMealLogError> {
        self.storage.save(DocType::MealLogs, doc)?;
        Ok(())
    }

    /// Creates a new meal log.
    pub fn create(&self, meallog: &MealLog) -> Result<MealLog, SyncMealLogError> {
        let mut doc = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_meallog(&mut doc, meallog);

        // Save
        self.save_doc(&mut doc)?;

        // Return the meal log (read back to confirm)
        self.get_by_id(meallog.id)?
            .ok_or_else(|| SyncMealLogError::NotFound(meallog.id.to_string()))
    }

    /// Deletes a meal log.
    pub fn delete(&self, id: Uuid) -> Result<(), SyncMealLogError> {
        let mut doc = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_meallog(&mut doc, id);

        // Save
        self.save_doc(&mut doc)?;

        Ok(())
    }

    // ========== Read operations (from Automerge) ==========

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

    /// Lists meal logs in a date range.
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

    fn setup() -> (SyncMealLogRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        let repo = SyncMealLogRepository::with_storage(storage);
        (repo, temp_dir)
    }

    #[test]
    fn test_create_meallog() {
        let (repo, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let log = MealLog::new(date, MealType::Lunch, "chef");
        let created = repo.create(&log).unwrap();

        assert_eq!(created.id, log.id);
        assert_eq!(created.date, date);
    }

    #[test]
    fn test_create_with_mealplan_id() {
        let (repo, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let mealplan_id = Uuid::new_v4();

        let log = MealLog::new(date, MealType::Dinner, "chef").with_mealplan_id(mealplan_id);
        let created = repo.create(&log).unwrap();

        assert_eq!(created.mealplan_id, Some(mealplan_id));
    }

    #[test]
    fn test_create_with_dishes() {
        let (repo, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let dish1 = Dish::new("Pasta", "chef");
        let dish2 = Dish::new("Salad", "chef");

        let log = MealLog::new(date, MealType::Lunch, "chef")
            .with_dishes(vec![dish1.clone(), dish2.clone()]);
        let created = repo.create(&log).unwrap();

        assert_eq!(created.dishes.len(), 2);
        let dish_names: Vec<&str> = created.dishes.iter().map(|d| d.name.as_str()).collect();
        assert!(dish_names.contains(&"Pasta"));
        assert!(dish_names.contains(&"Salad"));
    }

    #[test]
    fn test_delete_meallog() {
        let (repo, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let log = MealLog::new(date, MealType::Lunch, "chef");
        let id = log.id;
        repo.create(&log).unwrap();

        repo.delete(id).unwrap();

        let result = repo.get_by_id(id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_range() {
        let (repo, _temp) = setup();

        let jan1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let jan5 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let jan10 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        repo.create(&MealLog::new(jan1, MealType::Lunch, "chef"))
            .unwrap();
        repo.create(&MealLog::new(jan5, MealType::Dinner, "chef"))
            .unwrap();
        repo.create(&MealLog::new(jan10, MealType::Breakfast, "chef"))
            .unwrap();

        let logs = repo.list_range(jan1, jan5).unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_automerge_doc_persists() {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());

        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let log = MealLog::new(date, MealType::Lunch, "chef").with_notes("Test notes");
        let log_id = log.id;

        {
            let repo = SyncMealLogRepository::with_storage(storage.clone());
            repo.create(&log).unwrap();
        }

        // Create new repo instance and verify log is still there
        let repo = SyncMealLogRepository::with_storage(storage);
        let loaded = repo.get_by_id(log_id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().notes, Some("Test notes".to_string()));
    }
}
