//! Sync-aware meal plan repository that reads/writes Automerge documents.
//!
//! This module provides a repository layer that:
//! 1. Writes changes to Automerge documents (source of truth)
//! 2. Reads directly from Automerge documents (in-memory queries)

use automerge::AutoCommit;
use chrono::NaiveDate;
use uuid::Uuid;

use crate::models::{MealPlan, MealType};
use crate::sync::reader::{
    get_mealplan_by_date_and_type, get_mealplans_by_date, list_mealplans_by_date_range,
    read_all_mealplans, read_mealplan_by_id, ReaderError,
};
use crate::sync::storage::{DocType, DocumentStorage, StorageError};
use crate::sync::writer;

/// Error type for sync meal plan operations.
#[allow(dead_code)]
#[derive(Debug)]
pub enum SyncMealPlanError {
    /// Storage error (loading/saving Automerge docs).
    Storage(StorageError),
    /// Reader error (parsing Automerge data).
    Reader(ReaderError),
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
            SyncMealPlanError::Reader(e) => write!(f, "Reader error: {}", e),
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

impl From<ReaderError> for SyncMealPlanError {
    fn from(e: ReaderError) -> Self {
        SyncMealPlanError::Reader(e)
    }
}

/// Sync-aware meal plan repository.
///
/// All operations work directly with Automerge documents.
pub struct SyncMealPlanRepository {
    storage: DocumentStorage,
}
#[allow(dead_code)]
impl SyncMealPlanRepository {
    /// Creates a new sync meal plan repository.
    pub fn new() -> Self {
        Self {
            storage: DocumentStorage::new(),
        }
    }

    /// Creates a new sync meal plan repository with custom storage.
    #[cfg(test)]
    pub fn with_storage(storage: DocumentStorage) -> Self {
        Self { storage }
    }

    /// Loads the mealplans Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<AutoCommit, SyncMealPlanError> {
        match self.storage.load(DocType::MealPlans)? {
            Some(doc) => Ok(doc),
            None => Ok(AutoCommit::new()),
        }
    }

    /// Saves the document to storage.
    fn save_doc(&self, doc: &mut AutoCommit) -> Result<(), SyncMealPlanError> {
        self.storage.save(DocType::MealPlans, doc)?;
        Ok(())
    }

    /// Creates a new meal plan.
    pub fn create(&self, mealplan: &MealPlan) -> Result<MealPlan, SyncMealPlanError> {
        let mut doc = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_mealplan(&mut doc, mealplan);

        // Save
        self.save_doc(&mut doc)?;

        // Return the meal plan (read back to confirm)
        self.get_by_id(mealplan.id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(mealplan.id.to_string()))
    }

    /// Updates an existing meal plan.
    pub fn update(&self, mealplan: &MealPlan) -> Result<MealPlan, SyncMealPlanError> {
        let mut doc = self.load_or_create_doc()?;

        // Write updated meal plan to Automerge (overwrites existing)
        writer::write_mealplan(&mut doc, mealplan);

        // Save
        self.save_doc(&mut doc)?;

        // Return the updated meal plan
        self.get_by_id(mealplan.id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(mealplan.id.to_string()))
    }

    /// Deletes a meal plan.
    pub fn delete(&self, id: Uuid) -> Result<(), SyncMealPlanError> {
        let mut doc = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_mealplan(&mut doc, id);

        // Save
        self.save_doc(&mut doc)?;

        Ok(())
    }

    /// Adds a dish to a meal plan by ID.
    pub fn add_dish(&self, mealplan_id: Uuid, dish_id: Uuid) -> Result<(), SyncMealPlanError> {
        // Get the current meal plan
        let mut mealplan = self
            .get_by_id(mealplan_id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(mealplan_id.to_string()))?;

        // Check if already in plan
        if mealplan.dish_ids.contains(&dish_id) {
            return Err(SyncMealPlanError::DishAlreadyInPlan(dish_id.to_string()));
        }

        // Add the dish ID
        mealplan.add_dish(dish_id);

        // Update via Automerge
        self.update(&mealplan)?;

        Ok(())
    }

    /// Removes a dish from a meal plan by ID.
    pub fn remove_dish(&self, mealplan_id: Uuid, dish_id: Uuid) -> Result<(), SyncMealPlanError> {
        // Get the current meal plan
        let mut mealplan = self
            .get_by_id(mealplan_id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(mealplan_id.to_string()))?;

        // Check if dish is in plan
        if !mealplan.dish_ids.contains(&dish_id) {
            return Err(SyncMealPlanError::DishNotInPlan(dish_id.to_string()));
        }

        // Remove the dish ID
        mealplan.remove_dish(&dish_id);

        // Update via Automerge
        self.update(&mealplan)?;

        Ok(())
    }

    // ========== Read operations (from Automerge) ==========

    /// Gets a meal plan by ID.
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_mealplan_by_id(&doc, id)?)
    }

    /// Gets meal plans by date.
    pub fn get_by_date(&self, date: NaiveDate) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(get_mealplans_by_date(&doc, date)?)
    }

    /// Gets a meal plan by date and type.
    pub fn get_by_date_and_type(
        &self,
        date: NaiveDate,
        meal_type: MealType,
    ) -> Result<Option<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(get_mealplan_by_date_and_type(&doc, date, meal_type)?)
    }

    /// Lists all meal plans.
    pub fn list(&self) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_all_mealplans(&doc)?)
    }

    /// Lists meal plans in a date range.
    pub fn list_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(list_mealplans_by_date_range(&doc, from, to)?)
    }
}

impl Default for SyncMealPlanRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Dish;
    use crate::sync::SyncDishRepository;
    use tempfile::TempDir;

    fn setup() -> (SyncMealPlanRepository, SyncDishRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        let mealplan_repo = SyncMealPlanRepository::with_storage(storage.clone());
        let dish_repo = SyncDishRepository::with_storage(storage);
        (mealplan_repo, dish_repo, temp_dir)
    }

    #[test]
    fn test_create_mealplan() {
        let (repo, _, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Test Dinner", "chef");
        let created = repo.create(&plan).unwrap();

        assert_eq!(created.title, "Test Dinner");
        assert_eq!(created.id, plan.id);
    }

    #[test]
    fn test_create_and_list() {
        let (repo, _, _temp) = setup();
        let date1 = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2025, 1, 16).unwrap();

        repo.create(&MealPlan::new(date1, MealType::Dinner, "Dinner 1", "chef"))
            .unwrap();
        repo.create(&MealPlan::new(date2, MealType::Lunch, "Lunch", "chef"))
            .unwrap();

        let plans = repo.list_range(date1, date2).unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn test_update_mealplan() {
        let (repo, _, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Original", "chef");
        repo.create(&plan).unwrap();

        let mut updated = plan.clone();
        updated.title = "Updated".to_string();
        let result = repo.update(&updated).unwrap();

        assert_eq!(result.title, "Updated");
    }

    #[test]
    fn test_delete_mealplan() {
        let (repo, _, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "To Delete", "chef");
        let id = plan.id;
        repo.create(&plan).unwrap();

        repo.delete(id).unwrap();

        let result = repo.get_by_id(id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_add_dish() {
        let (mealplan_repo, dish_repo, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        // Create a dish
        let dish = Dish::new("Pasta", "chef");
        dish_repo.create(&dish).unwrap();

        // Create a meal plan
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner", "chef");
        mealplan_repo.create(&plan).unwrap();

        // Add dish
        mealplan_repo.add_dish(plan.id, dish.id).unwrap();

        let updated = mealplan_repo.get_by_id(plan.id).unwrap().unwrap();
        assert_eq!(updated.dish_ids.len(), 1);
        assert_eq!(updated.dish_ids[0], dish.id);
    }

    #[test]
    fn test_add_dish_already_in_plan() {
        let (mealplan_repo, dish_repo, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let dish = Dish::new("Pasta", "chef");
        dish_repo.create(&dish).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Dinner", "chef");
        mealplan_repo.create(&plan).unwrap();

        // Add dish first time
        mealplan_repo.add_dish(plan.id, dish.id).unwrap();

        // Try to add again
        let result = mealplan_repo.add_dish(plan.id, dish.id);
        assert!(matches!(
            result,
            Err(SyncMealPlanError::DishAlreadyInPlan(_))
        ));
    }

    #[test]
    fn test_remove_dish() {
        let (mealplan_repo, dish_repo, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let dish = Dish::new("Pasta", "chef");
        dish_repo.create(&dish).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Dinner", "chef");
        mealplan_repo.create(&plan).unwrap();
        mealplan_repo.add_dish(plan.id, dish.id).unwrap();

        // Remove dish
        mealplan_repo.remove_dish(plan.id, dish.id).unwrap();

        let updated = mealplan_repo.get_by_id(plan.id).unwrap().unwrap();
        assert!(updated.dish_ids.is_empty());
    }

    #[test]
    fn test_remove_dish_not_in_plan() {
        let (mealplan_repo, dish_repo, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let dish = Dish::new("Pasta", "chef");
        dish_repo.create(&dish).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Dinner", "chef");
        mealplan_repo.create(&plan).unwrap();

        // Try to remove dish that's not in plan
        let result = mealplan_repo.remove_dish(plan.id, dish.id);
        assert!(matches!(result, Err(SyncMealPlanError::DishNotInPlan(_))));
    }

    #[test]
    fn test_get_by_date() {
        let (repo, _, _temp) = setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        repo.create(&MealPlan::new(
            date,
            MealType::Breakfast,
            "Breakfast",
            "chef",
        ))
        .unwrap();
        repo.create(&MealPlan::new(date, MealType::Dinner, "Dinner", "chef"))
            .unwrap();

        let plans = repo.get_by_date(date).unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn test_list_range() {
        let (repo, _, _temp) = setup();

        let jan1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let jan5 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let jan10 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        repo.create(&MealPlan::new(jan1, MealType::Dinner, "Jan 1", "chef"))
            .unwrap();
        repo.create(&MealPlan::new(jan5, MealType::Dinner, "Jan 5", "chef"))
            .unwrap();
        repo.create(&MealPlan::new(jan10, MealType::Dinner, "Jan 10", "chef"))
            .unwrap();

        let plans = repo.list_range(jan1, jan5).unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn test_automerge_doc_persists() {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());

        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Persistent Plan", "chef");
        let plan_id = plan.id;

        {
            let repo = SyncMealPlanRepository::with_storage(storage.clone());
            repo.create(&plan).unwrap();
        }

        // Create new repo instance and verify plan is still there
        let repo = SyncMealPlanRepository::with_storage(storage);
        let loaded = repo.get_by_id(plan_id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().title, "Persistent Plan");
    }
}
