//! Sync-aware meal plan repository that reads/writes Automerge documents.
//!
//! This module provides a repository layer that:
//! 1. Uses the current group's mealplans document when identity is configured
//! 2. Falls back to legacy DocType-based storage otherwise
//! 3. Reads directly from Automerge documents (in-memory queries)

use automerge::AutoCommit;
use chrono::NaiveDate;
use uuid::Uuid;

use todu_fit_core::{DocumentId, MultiDocStorage};

use crate::config::Config;
use crate::models::{MealPlan, MealType};
use crate::sync::group_context::{is_identity_ready, resolve_group_context, GroupContextError};
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
    /// Group context error.
    GroupContext(GroupContextError),
    /// Multi-storage error.
    MultiStorage(todu_fit_core::MultiStorageError),
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
            SyncMealPlanError::GroupContext(e) => write!(f, "{}", e),
            SyncMealPlanError::MultiStorage(e) => write!(f, "Storage error: {}", e),
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

impl From<GroupContextError> for SyncMealPlanError {
    fn from(e: GroupContextError) -> Self {
        SyncMealPlanError::GroupContext(e)
    }
}

impl From<todu_fit_core::MultiStorageError> for SyncMealPlanError {
    fn from(e: todu_fit_core::MultiStorageError) -> Self {
        SyncMealPlanError::MultiStorage(e)
    }
}

/// Sync-aware meal plan repository.
///
/// All operations work directly with Automerge documents.
/// Uses the current group's mealplans document when identity is configured,
/// otherwise falls back to legacy DocType-based storage.
pub struct SyncMealPlanRepository {
    legacy_storage: DocumentStorage,
    multi_storage: MultiDocStorage,
    group_override: Option<String>,
}

#[allow(dead_code)]
impl SyncMealPlanRepository {
    /// Creates a new sync meal plan repository.
    pub fn new() -> Self {
        Self {
            legacy_storage: DocumentStorage::new(),
            multi_storage: MultiDocStorage::new(Config::default_data_dir()),
            group_override: None,
        }
    }

    /// Creates a new repository with a specific group override.
    pub fn with_group(group_name: &str) -> Self {
        Self {
            legacy_storage: DocumentStorage::new(),
            multi_storage: MultiDocStorage::new(Config::default_data_dir()),
            group_override: Some(group_name.to_string()),
        }
    }

    /// Creates a new sync meal plan repository with custom storage (for testing).
    #[cfg(test)]
    pub fn with_storage(storage: DocumentStorage) -> Self {
        Self {
            legacy_storage: storage,
            multi_storage: MultiDocStorage::new(Config::default_data_dir()),
            group_override: None,
        }
    }

    /// Resolves the mealplans document ID.
    /// Returns None if using legacy mode, Some(doc_id) if using group mode.
    fn resolve_doc_id(&self) -> Option<DocumentId> {
        if !is_identity_ready() {
            return None;
        }

        resolve_group_context(self.group_override.as_deref())
            .ok()
            .map(|ctx| ctx.mealplans_doc_id)
    }

    /// Loads the mealplans Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<AutoCommit, SyncMealPlanError> {
        if let Some(doc_id) = self.resolve_doc_id() {
            // Group mode: use multi-storage with document ID
            match self.multi_storage.load(&doc_id)? {
                Some(bytes) => AutoCommit::load(&bytes).map_err(|e| {
                    SyncMealPlanError::Reader(ReaderError::AutomergeError(e.to_string()))
                }),
                None => Ok(AutoCommit::new()),
            }
        } else {
            // Legacy mode: use DocType-based storage
            match self.legacy_storage.load(DocType::MealPlans)? {
                Some(doc) => Ok(doc),
                None => Ok(AutoCommit::new()),
            }
        }
    }

    /// Saves the document to storage.
    fn save_doc(&self, doc: &mut AutoCommit) -> Result<(), SyncMealPlanError> {
        if let Some(doc_id) = self.resolve_doc_id() {
            // Group mode: use multi-storage
            let bytes = doc.save();
            self.multi_storage.save(&doc_id, &bytes)?;
        } else {
            // Legacy mode
            self.legacy_storage.save(DocType::MealPlans, doc)?;
        }
        Ok(())
    }

    /// Creates a new meal plan.
    pub fn create(&self, plan: &MealPlan) -> Result<MealPlan, SyncMealPlanError> {
        let mut doc = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_mealplan(&mut doc, plan);

        // Save
        self.save_doc(&mut doc)?;

        // Return the plan (read back to confirm)
        self.get_by_id(plan.id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(plan.id.to_string()))
    }

    /// Updates an existing meal plan.
    pub fn update(&self, plan: &MealPlan) -> Result<MealPlan, SyncMealPlanError> {
        let mut doc = self.load_or_create_doc()?;

        // Write updated plan to Automerge (overwrites existing)
        writer::write_mealplan(&mut doc, plan);

        // Save
        self.save_doc(&mut doc)?;

        // Return the updated plan
        self.get_by_id(plan.id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(plan.id.to_string()))
    }

    /// Deletes a meal plan by ID.
    pub fn delete(&self, id: Uuid) -> Result<(), SyncMealPlanError> {
        let mut doc = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_mealplan(&mut doc, id);

        // Save
        self.save_doc(&mut doc)?;

        Ok(())
    }

    /// Gets a meal plan by ID.
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_mealplan_by_id(&doc, id)?)
    }

    /// Lists all meal plans.
    pub fn list(&self) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_all_mealplans(&doc)?)
    }

    /// Lists meal plans within a date range.
    pub fn list_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(list_mealplans_by_date_range(&doc, from, to)?)
    }

    /// Gets meal plans for a specific date.
    pub fn get_by_date(&self, date: NaiveDate) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(get_mealplans_by_date(&doc, date)?)
    }

    /// Gets a meal plan for a specific date and meal type.
    pub fn get_by_date_and_type(
        &self,
        date: NaiveDate,
        meal_type: MealType,
    ) -> Result<Option<MealPlan>, SyncMealPlanError> {
        let doc = self.load_or_create_doc()?;
        Ok(get_mealplan_by_date_and_type(&doc, date, meal_type)?)
    }

    /// Adds a dish to a meal plan.
    pub fn add_dish(&self, plan_id: Uuid, dish_id: Uuid) -> Result<(), SyncMealPlanError> {
        let mut plan = self
            .get_by_id(plan_id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(plan_id.to_string()))?;

        // Check if already in plan
        if plan.dish_ids.contains(&dish_id) {
            return Err(SyncMealPlanError::DishAlreadyInPlan(dish_id.to_string()));
        }

        plan.dish_ids.push(dish_id);
        self.update(&plan)?;

        Ok(())
    }

    /// Removes a dish from a meal plan.
    pub fn remove_dish(&self, plan_id: Uuid, dish_id: Uuid) -> Result<(), SyncMealPlanError> {
        let mut plan = self
            .get_by_id(plan_id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(plan_id.to_string()))?;

        // Check if in plan
        if !plan.dish_ids.contains(&dish_id) {
            return Err(SyncMealPlanError::DishNotInPlan(dish_id.to_string()));
        }

        plan.dish_ids.retain(|id| id != &dish_id);
        self.update(&plan)?;

        Ok(())
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
    use tempfile::TempDir;

    fn test_repo() -> (SyncMealPlanRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        let repo = SyncMealPlanRepository::with_storage(storage);
        (repo, temp_dir)
    }

    #[test]
    fn test_create_and_list() {
        let (repo, _temp) = test_repo();

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        repo.create(&plan).unwrap();

        let plans = repo.list().unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].date, date);
    }

    #[test]
    fn test_get_by_date() {
        let (repo, _temp) = test_repo();

        let date1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();

        repo.create(&MealPlan::new(
            date1,
            MealType::Breakfast,
            "Breakfast",
            "chef",
        ))
        .unwrap();
        repo.create(&MealPlan::new(date1, MealType::Dinner, "Dinner", "chef"))
            .unwrap();
        repo.create(&MealPlan::new(date2, MealType::Lunch, "Lunch", "chef"))
            .unwrap();

        let plans = repo.get_by_date(date1).unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn test_list_range() {
        let (repo, _temp) = test_repo();

        let date1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let date3 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        repo.create(&MealPlan::new(date1, MealType::Dinner, "Dinner 1", "chef"))
            .unwrap();
        repo.create(&MealPlan::new(date2, MealType::Dinner, "Dinner 2", "chef"))
            .unwrap();
        repo.create(&MealPlan::new(date3, MealType::Dinner, "Dinner 3", "chef"))
            .unwrap();

        let from = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let to = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap();
        let plans = repo.list_range(from, to).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].date, date2);
    }

    #[test]
    fn test_add_dish() {
        let (repo, _temp) = test_repo();

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        repo.create(&plan).unwrap();

        let dish_id = Uuid::new_v4();
        repo.add_dish(plan.id, dish_id).unwrap();

        let updated = repo.get_by_id(plan.id).unwrap().unwrap();
        assert_eq!(updated.dish_ids.len(), 1);
        assert!(updated.dish_ids.contains(&dish_id));
    }

    #[test]
    fn test_add_dish_already_in_plan() {
        let (repo, _temp) = test_repo();

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        repo.create(&plan).unwrap();

        let dish_id = Uuid::new_v4();
        repo.add_dish(plan.id, dish_id).unwrap();

        // Try to add same dish again
        let result = repo.add_dish(plan.id, dish_id);
        assert!(matches!(
            result,
            Err(SyncMealPlanError::DishAlreadyInPlan(_))
        ));
    }

    #[test]
    fn test_remove_dish() {
        let (repo, _temp) = test_repo();

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let dish_id1 = Uuid::new_v4();
        let dish_id2 = Uuid::new_v4();
        let mut plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        plan.dish_ids = vec![dish_id1, dish_id2];
        repo.create(&plan).unwrap();

        repo.remove_dish(plan.id, dish_id1).unwrap();

        let updated = repo.get_by_id(plan.id).unwrap().unwrap();
        assert_eq!(updated.dish_ids.len(), 1);
        assert!(!updated.dish_ids.contains(&dish_id1));
        assert!(updated.dish_ids.contains(&dish_id2));
    }

    #[test]
    fn test_remove_dish_not_in_plan() {
        let (repo, _temp) = test_repo();

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        repo.create(&plan).unwrap();

        let dish_id = Uuid::new_v4();
        let result = repo.remove_dish(plan.id, dish_id);
        assert!(matches!(result, Err(SyncMealPlanError::DishNotInPlan(_))));
    }

    #[test]
    fn test_update_mealplan() {
        let (repo, _temp) = test_repo();

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut plan = MealPlan::new(date, MealType::Dinner, "Original Title", "chef");
        repo.create(&plan).unwrap();

        plan.title = "Updated Title".to_string();
        let updated = repo.update(&plan).unwrap();

        assert_eq!(updated.title, "Updated Title");
    }
}
