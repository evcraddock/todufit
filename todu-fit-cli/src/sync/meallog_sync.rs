//! Sync-aware meal log repository that writes to Automerge and projects to SQLite.
//!
//! This module provides a repository layer that:
//! 1. Writes changes to Automerge documents (source of truth)
//! 2. Projects changes to SQLite (for fast queries)
//! 3. Reads from SQLite for queries

use automerge::AutoCommit;
use chrono::NaiveDate;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::MealLogRepository;
use crate::models::MealLog;
use crate::sync::projection::MealLogProjection;
use crate::sync::storage::{DocType, DocumentStorage, StorageError};
use crate::sync::writer;

/// Error type for sync meal log operations.
#[derive(Debug)]
pub enum SyncMealLogError {
    /// Storage error (loading/saving Automerge docs).
    Storage(StorageError),
    /// Projection error (syncing to SQLite).
    Projection(crate::sync::projection::ProjectionError),
    /// SQLite error (queries).
    Sqlite(sqlx::Error),
    /// MealLog not found.
    NotFound(String),
}

impl std::fmt::Display for SyncMealLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncMealLogError::Storage(e) => write!(f, "Storage error: {}", e),
            SyncMealLogError::Projection(e) => write!(f, "Projection error: {}", e),
            SyncMealLogError::Sqlite(e) => write!(f, "SQLite error: {}", e),
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

impl From<crate::sync::projection::ProjectionError> for SyncMealLogError {
    fn from(e: crate::sync::projection::ProjectionError) -> Self {
        SyncMealLogError::Projection(e)
    }
}

impl From<sqlx::Error> for SyncMealLogError {
    fn from(e: sqlx::Error) -> Self {
        SyncMealLogError::Sqlite(e)
    }
}

/// Sync-aware meal log repository.
///
/// Writes go to Automerge first, then project to SQLite.
/// Reads come from SQLite for fast queries.
pub struct SyncMealLogRepository {
    storage: DocumentStorage,
    pool: SqlitePool,
}

impl SyncMealLogRepository {
    /// Creates a new sync meal log repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            storage: DocumentStorage::new(),
            pool,
        }
    }

    /// Creates a new sync meal log repository with custom storage.
    #[cfg(test)]
    pub fn with_storage(storage: DocumentStorage, pool: SqlitePool) -> Self {
        Self { storage, pool }
    }

    /// Loads the meallogs Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<AutoCommit, SyncMealLogError> {
        match self.storage.load(DocType::MealLogs)? {
            Some(doc) => Ok(doc),
            None => Ok(AutoCommit::new()),
        }
    }

    /// Saves the document and projects to SQLite.
    async fn save_and_project(&self, doc: &mut AutoCommit) -> Result<(), SyncMealLogError> {
        // Save to Automerge storage
        self.storage.save(DocType::MealLogs, doc)?;

        // Project to SQLite
        MealLogProjection::project_all(doc, &self.pool).await?;

        Ok(())
    }

    /// Creates a new meal log.
    pub async fn create(&self, meallog: &MealLog) -> Result<MealLog, SyncMealLogError> {
        let mut doc = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_meallog(&mut doc, meallog);

        // Save and project
        self.save_and_project(&mut doc).await?;

        // Return the meal log (read from SQLite to confirm)
        self.get_by_id(meallog.id)
            .await?
            .ok_or_else(|| SyncMealLogError::NotFound(meallog.id.to_string()))
    }

    /// Deletes a meal log.
    #[cfg(test)]
    pub async fn delete(&self, id: Uuid) -> Result<(), SyncMealLogError> {
        let mut doc = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_meallog(&mut doc, id);

        // Save and project
        self.save_and_project(&mut doc).await?;

        Ok(())
    }

    // ========== Read operations (from SQLite) ==========

    /// Gets a meal log by ID.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MealLog>, SyncMealLogError> {
        let repo = MealLogRepository::new(self.pool.clone());
        Ok(repo.get_by_id(id).await?)
    }

    /// Lists meal logs in a date range.
    pub async fn list_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealLog>, SyncMealLogError> {
        let repo = MealLogRepository::new(self.pool.clone());
        Ok(repo.list_range(from, to).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use crate::models::{Dish, MealPlan, MealType};
    use crate::sync::{SyncDishRepository, SyncMealPlanRepository};
    use tempfile::TempDir;

    async fn setup() -> (
        SyncMealLogRepository,
        SyncMealPlanRepository,
        SyncDishRepository,
        TempDir,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        let meallog_repo = SyncMealLogRepository::with_storage(storage.clone(), pool.clone());
        let mealplan_repo = SyncMealPlanRepository::with_storage(storage.clone(), pool.clone());
        let dish_repo = SyncDishRepository::with_storage(storage, pool);
        (meallog_repo, mealplan_repo, dish_repo, temp_dir)
    }

    #[tokio::test]
    async fn test_create_meallog() {
        let (repo, _, _, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let log = MealLog::new(date, MealType::Dinner, "chef").with_notes("Great meal!");
        let created = repo.create(&log).await.unwrap();

        assert_eq!(created.date, date);
        assert_eq!(created.meal_type, MealType::Dinner);
        assert_eq!(created.notes, Some("Great meal!".to_string()));
        assert_eq!(created.id, log.id);
    }

    #[tokio::test]
    async fn test_create_with_mealplan_id() {
        let (meallog_repo, mealplan_repo, _, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        // Create a meal plan
        let plan = MealPlan::new(date, MealType::Lunch, "Lunch Plan", "chef");
        mealplan_repo.create(&plan).await.unwrap();

        // Create a meal log referencing the plan
        let log = MealLog::new(date, MealType::Lunch, "chef").with_mealplan_id(plan.id);
        let created = meallog_repo.create(&log).await.unwrap();

        assert_eq!(created.mealplan_id, Some(plan.id));
    }

    #[tokio::test]
    async fn test_create_with_dishes() {
        let (meallog_repo, _, dish_repo, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        // Create dishes
        let dish1 = Dish::new("Pasta", "chef");
        let dish2 = Dish::new("Salad", "chef");
        dish_repo.create(&dish1).await.unwrap();
        dish_repo.create(&dish2).await.unwrap();

        // Create meal log with dishes
        let log = MealLog::new(date, MealType::Dinner, "chef")
            .with_dishes(vec![dish1.clone(), dish2.clone()]);
        let created = meallog_repo.create(&log).await.unwrap();

        assert_eq!(created.dishes.len(), 2);
        let dish_names: Vec<&str> = created.dishes.iter().map(|d| d.name.as_str()).collect();
        assert!(dish_names.contains(&"Pasta"));
        assert!(dish_names.contains(&"Salad"));
    }

    #[tokio::test]
    async fn test_delete_meallog() {
        let (repo, _, _, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let log = MealLog::new(date, MealType::Dinner, "chef");
        let id = log.id;
        repo.create(&log).await.unwrap();

        repo.delete(id).await.unwrap();

        let result = repo.get_by_id(id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_range() {
        let (repo, _, _, _temp) = setup().await;

        let jan1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let jan5 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let jan10 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        repo.create(&MealLog::new(jan1, MealType::Dinner, "chef"))
            .await
            .unwrap();
        repo.create(&MealLog::new(jan5, MealType::Lunch, "chef"))
            .await
            .unwrap();
        repo.create(&MealLog::new(jan10, MealType::Breakfast, "chef"))
            .await
            .unwrap();

        let logs = repo.list_range(jan1, jan5).await.unwrap();
        assert_eq!(logs.len(), 2);

        let all_logs = repo.list_range(jan1, jan10).await.unwrap();
        assert_eq!(all_logs.len(), 3);
    }

    #[tokio::test]
    async fn test_automerge_doc_persists() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());

        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let log = MealLog::new(date, MealType::Dinner, "chef");
        let log_id = log.id;

        {
            let repo = SyncMealLogRepository::with_storage(storage.clone(), pool.clone());
            repo.create(&log).await.unwrap();
        }

        // Verify Automerge doc exists and contains the meallog
        let doc = storage.load(DocType::MealLogs).unwrap().unwrap();
        use automerge::ReadDoc;
        assert!(doc
            .get(automerge::ROOT, &log_id.to_string())
            .unwrap()
            .is_some());
    }
}
