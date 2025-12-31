//! Sync-aware meal plan repository that writes to Automerge and projects to SQLite.
//!
//! This module provides a repository layer that:
//! 1. Writes changes to Automerge documents (source of truth)
//! 2. Projects changes to SQLite (for fast queries)
//! 3. Reads from SQLite for queries

use automerge::AutoCommit;
use chrono::NaiveDate;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db::{DishRepository, MealPlanRepository};
use crate::models::{MealPlan, MealType};
use crate::sync::projection::MealPlanProjection;
use crate::sync::storage::{DocType, DocumentStorage, StorageError};
use crate::sync::writer;

/// Error type for sync meal plan operations.
#[derive(Debug)]
pub enum SyncMealPlanError {
    /// Storage error (loading/saving Automerge docs).
    Storage(StorageError),
    /// Projection error (syncing to SQLite).
    Projection(crate::sync::projection::ProjectionError),
    /// SQLite error (queries).
    Sqlite(sqlx::Error),
    /// MealPlan not found.
    NotFound(String),
    /// Dish not found.
    DishNotFound(String),
    /// Dish already in plan.
    DishAlreadyInPlan(String),
    /// Dish not in plan.
    DishNotInPlan(String),
}

impl std::fmt::Display for SyncMealPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncMealPlanError::Storage(e) => write!(f, "Storage error: {}", e),
            SyncMealPlanError::Projection(e) => write!(f, "Projection error: {}", e),
            SyncMealPlanError::Sqlite(e) => write!(f, "SQLite error: {}", e),
            SyncMealPlanError::NotFound(id) => write!(f, "MealPlan not found: {}", id),
            SyncMealPlanError::DishNotFound(id) => write!(f, "Dish not found: {}", id),
            SyncMealPlanError::DishAlreadyInPlan(name) => {
                write!(f, "Dish '{}' is already in this meal plan", name)
            }
            SyncMealPlanError::DishNotInPlan(name) => {
                write!(f, "Dish '{}' is not in this meal plan", name)
            }
        }
    }
}

impl std::error::Error for SyncMealPlanError {}

impl From<StorageError> for SyncMealPlanError {
    fn from(e: StorageError) -> Self {
        SyncMealPlanError::Storage(e)
    }
}

impl From<crate::sync::projection::ProjectionError> for SyncMealPlanError {
    fn from(e: crate::sync::projection::ProjectionError) -> Self {
        SyncMealPlanError::Projection(e)
    }
}

impl From<sqlx::Error> for SyncMealPlanError {
    fn from(e: sqlx::Error) -> Self {
        SyncMealPlanError::Sqlite(e)
    }
}

/// Sync-aware meal plan repository.
///
/// Writes go to Automerge first, then project to SQLite.
/// Reads come from SQLite for fast queries.
pub struct SyncMealPlanRepository {
    storage: DocumentStorage,
    pool: SqlitePool,
}

impl SyncMealPlanRepository {
    /// Creates a new sync meal plan repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            storage: DocumentStorage::new(),
            pool,
        }
    }

    /// Creates a new sync meal plan repository with custom storage.
    pub fn with_storage(storage: DocumentStorage, pool: SqlitePool) -> Self {
        Self { storage, pool }
    }

    /// Loads the mealplans Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<AutoCommit, SyncMealPlanError> {
        match self.storage.load(DocType::MealPlans)? {
            Some(doc) => Ok(doc),
            None => Ok(AutoCommit::new()),
        }
    }

    /// Saves the document and projects to SQLite.
    async fn save_and_project(&self, doc: &mut AutoCommit) -> Result<(), SyncMealPlanError> {
        // Save to Automerge storage
        self.storage.save(DocType::MealPlans, doc)?;

        // Project to SQLite
        MealPlanProjection::project_all(doc, &self.pool).await?;

        Ok(())
    }

    /// Creates a new meal plan.
    pub async fn create(&self, mealplan: &MealPlan) -> Result<MealPlan, SyncMealPlanError> {
        let mut doc = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_mealplan(&mut doc, mealplan);

        // Save and project
        self.save_and_project(&mut doc).await?;

        // Return the meal plan (read from SQLite to confirm)
        self.get_by_id(mealplan.id)
            .await?
            .ok_or_else(|| SyncMealPlanError::NotFound(mealplan.id.to_string()))
    }

    /// Updates an existing meal plan.
    pub async fn update(&self, mealplan: &MealPlan) -> Result<MealPlan, SyncMealPlanError> {
        let mut doc = self.load_or_create_doc()?;

        // Write updated meal plan to Automerge (overwrites existing)
        writer::write_mealplan(&mut doc, mealplan);

        // Save and project
        self.save_and_project(&mut doc).await?;

        // Return the updated meal plan
        self.get_by_id(mealplan.id)
            .await?
            .ok_or_else(|| SyncMealPlanError::NotFound(mealplan.id.to_string()))
    }

    /// Deletes a meal plan.
    pub async fn delete(&self, id: Uuid) -> Result<(), SyncMealPlanError> {
        let mut doc = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_mealplan(&mut doc, id);

        // Save and project
        self.save_and_project(&mut doc).await?;

        Ok(())
    }

    /// Adds a dish to a meal plan.
    pub async fn add_dish(
        &self,
        mealplan_id: Uuid,
        dish_id: Uuid,
    ) -> Result<(), SyncMealPlanError> {
        // Get the current meal plan
        let mut mealplan = self
            .get_by_id(mealplan_id)
            .await?
            .ok_or_else(|| SyncMealPlanError::NotFound(mealplan_id.to_string()))?;

        // Get the dish from SQLite
        let dish_repo = DishRepository::new(self.pool.clone());
        let dish = dish_repo
            .get_by_id(dish_id)
            .await?
            .ok_or_else(|| SyncMealPlanError::DishNotFound(dish_id.to_string()))?;

        // Check if already in plan
        if mealplan.dishes.iter().any(|d| d.id == dish_id) {
            return Err(SyncMealPlanError::DishAlreadyInPlan(dish.name));
        }

        // Add the dish
        mealplan.dishes.push(dish);
        mealplan.updated_at = chrono::Utc::now();

        // Update via Automerge
        self.update(&mealplan).await?;

        Ok(())
    }

    /// Removes a dish from a meal plan.
    pub async fn remove_dish(
        &self,
        mealplan_id: Uuid,
        dish_id: Uuid,
    ) -> Result<(), SyncMealPlanError> {
        // Get the current meal plan
        let mut mealplan = self
            .get_by_id(mealplan_id)
            .await?
            .ok_or_else(|| SyncMealPlanError::NotFound(mealplan_id.to_string()))?;

        // Check if dish is in plan
        let dish_name = mealplan
            .dishes
            .iter()
            .find(|d| d.id == dish_id)
            .map(|d| d.name.clone());

        match dish_name {
            Some(_) => {
                // Remove the dish
                mealplan.dishes.retain(|d| d.id != dish_id);
                mealplan.updated_at = chrono::Utc::now();

                // Update via Automerge
                self.update(&mealplan).await?;

                Ok(())
            }
            None => {
                // Get dish name for error message
                let dish_repo = DishRepository::new(self.pool.clone());
                let name = dish_repo
                    .get_by_id(dish_id)
                    .await?
                    .map(|d| d.name)
                    .unwrap_or_else(|| dish_id.to_string());
                Err(SyncMealPlanError::DishNotInPlan(name))
            }
        }
    }

    // ========== Read operations (from SQLite) ==========

    /// Gets a meal plan by ID.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MealPlan>, SyncMealPlanError> {
        let repo = MealPlanRepository::new(self.pool.clone());
        Ok(repo.get_by_id(id).await?)
    }

    /// Gets meal plans by date.
    pub async fn get_by_date(&self, date: NaiveDate) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let repo = MealPlanRepository::new(self.pool.clone());
        Ok(repo.get_by_date(date).await?)
    }

    /// Gets a meal plan by date and type.
    pub async fn get_by_date_and_type(
        &self,
        date: NaiveDate,
        meal_type: MealType,
    ) -> Result<Option<MealPlan>, SyncMealPlanError> {
        let repo = MealPlanRepository::new(self.pool.clone());
        Ok(repo.get_by_date_and_type(date, meal_type).await?)
    }

    /// Lists all meal plans.
    pub async fn list(&self) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let repo = MealPlanRepository::new(self.pool.clone());
        Ok(repo.list().await?)
    }

    /// Lists meal plans in a date range.
    pub async fn list_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let repo = MealPlanRepository::new(self.pool.clone());
        Ok(repo.list_range(from, to).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use crate::models::Dish;
    use crate::sync::SyncDishRepository;
    use tempfile::TempDir;

    async fn setup() -> (SyncMealPlanRepository, SyncDishRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        let mealplan_repo = SyncMealPlanRepository::with_storage(storage.clone(), pool.clone());
        let dish_repo = SyncDishRepository::with_storage(storage, pool);
        (mealplan_repo, dish_repo, temp_dir)
    }

    #[tokio::test]
    async fn test_create_mealplan() {
        let (repo, _, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Test Dinner", "chef");
        let created = repo.create(&plan).await.unwrap();

        assert_eq!(created.title, "Test Dinner");
        assert_eq!(created.id, plan.id);
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let (repo, _, _temp) = setup().await;
        let date1 = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2025, 1, 16).unwrap();

        repo.create(&MealPlan::new(date1, MealType::Dinner, "Dinner 1", "chef"))
            .await
            .unwrap();
        repo.create(&MealPlan::new(date2, MealType::Lunch, "Lunch", "chef"))
            .await
            .unwrap();

        let plans = repo.list().await.unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[tokio::test]
    async fn test_update_mealplan() {
        let (repo, _, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Original", "chef");
        repo.create(&plan).await.unwrap();

        let mut updated = plan.clone();
        updated.title = "Updated".to_string();
        let result = repo.update(&updated).await.unwrap();

        assert_eq!(result.title, "Updated");
    }

    #[tokio::test]
    async fn test_delete_mealplan() {
        let (repo, _, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "To Delete", "chef");
        let id = plan.id;
        repo.create(&plan).await.unwrap();

        repo.delete(id).await.unwrap();

        let result = repo.get_by_id(id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_add_dish() {
        let (mealplan_repo, dish_repo, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        // Create a dish
        let dish = Dish::new("Pasta", "chef");
        dish_repo.create(&dish).await.unwrap();

        // Create a meal plan
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner", "chef");
        mealplan_repo.create(&plan).await.unwrap();

        // Add dish
        mealplan_repo.add_dish(plan.id, dish.id).await.unwrap();

        let updated = mealplan_repo.get_by_id(plan.id).await.unwrap().unwrap();
        assert_eq!(updated.dishes.len(), 1);
        assert_eq!(updated.dishes[0].name, "Pasta");
    }

    #[tokio::test]
    async fn test_add_dish_already_in_plan() {
        let (mealplan_repo, dish_repo, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let dish = Dish::new("Pasta", "chef");
        dish_repo.create(&dish).await.unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Dinner", "chef");
        mealplan_repo.create(&plan).await.unwrap();

        // Add dish first time
        mealplan_repo.add_dish(plan.id, dish.id).await.unwrap();

        // Try to add again
        let result = mealplan_repo.add_dish(plan.id, dish.id).await;
        assert!(matches!(
            result,
            Err(SyncMealPlanError::DishAlreadyInPlan(_))
        ));
    }

    #[tokio::test]
    async fn test_remove_dish() {
        let (mealplan_repo, dish_repo, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let dish = Dish::new("Pasta", "chef");
        dish_repo.create(&dish).await.unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Dinner", "chef");
        mealplan_repo.create(&plan).await.unwrap();
        mealplan_repo.add_dish(plan.id, dish.id).await.unwrap();

        // Remove dish
        mealplan_repo.remove_dish(plan.id, dish.id).await.unwrap();

        let updated = mealplan_repo.get_by_id(plan.id).await.unwrap().unwrap();
        assert!(updated.dishes.is_empty());
    }

    #[tokio::test]
    async fn test_remove_dish_not_in_plan() {
        let (mealplan_repo, dish_repo, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let dish = Dish::new("Pasta", "chef");
        dish_repo.create(&dish).await.unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Dinner", "chef");
        mealplan_repo.create(&plan).await.unwrap();

        // Try to remove dish that's not in plan
        let result = mealplan_repo.remove_dish(plan.id, dish.id).await;
        assert!(matches!(result, Err(SyncMealPlanError::DishNotInPlan(_))));
    }

    #[tokio::test]
    async fn test_get_by_date() {
        let (repo, _, _temp) = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        repo.create(&MealPlan::new(
            date,
            MealType::Breakfast,
            "Breakfast",
            "chef",
        ))
        .await
        .unwrap();
        repo.create(&MealPlan::new(date, MealType::Dinner, "Dinner", "chef"))
            .await
            .unwrap();

        let plans = repo.get_by_date(date).await.unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[tokio::test]
    async fn test_list_range() {
        let (repo, _, _temp) = setup().await;

        let jan1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let jan5 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let jan10 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        repo.create(&MealPlan::new(jan1, MealType::Dinner, "Jan 1", "chef"))
            .await
            .unwrap();
        repo.create(&MealPlan::new(jan5, MealType::Dinner, "Jan 5", "chef"))
            .await
            .unwrap();
        repo.create(&MealPlan::new(jan10, MealType::Dinner, "Jan 10", "chef"))
            .await
            .unwrap();

        let plans = repo.list_range(jan1, jan5).await.unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[tokio::test]
    async fn test_automerge_doc_persists() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());

        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Persistent Plan", "chef");
        let plan_id = plan.id;

        {
            let repo = SyncMealPlanRepository::with_storage(storage.clone(), pool.clone());
            repo.create(&plan).await.unwrap();
        }

        // Verify Automerge doc exists and contains the mealplan
        let doc = storage.load(DocType::MealPlans).unwrap().unwrap();
        use automerge::ReadDoc;
        assert!(doc
            .get(automerge::ROOT, &plan_id.to_string())
            .unwrap()
            .is_some());
    }
}
