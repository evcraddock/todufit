//! Sync-aware meal plan repository that reads/writes Automerge documents.
//!
//! This module provides a repository layer that uses the current group's
//! mealplans document. Identity must be initialized first.

use std::path::PathBuf;

use automerge::AutoCommit;
use chrono::NaiveDate;
use uuid::Uuid;

use todu_fit_core::{DocumentId, MultiDocStorage};

use crate::models::{MealPlan, MealType};
use crate::sync::group_context::{resolve_group_context, GroupContextError};
use crate::sync::reader::{
    get_mealplan_by_date_and_type, get_mealplans_by_date, list_mealplans_by_date_range,
    read_all_mealplans, read_mealplan_by_id, ReaderError,
};
use crate::sync::writer;

/// Error type for sync meal plan operations.
#[allow(dead_code)]
#[derive(Debug)]
pub enum SyncMealPlanError {
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
/// Uses the current group's mealplans document.
pub struct SyncMealPlanRepository {
    storage: MultiDocStorage,
    data_dir: PathBuf,
    group_override: Option<String>,
}

impl SyncMealPlanRepository {
    /// Creates a new sync meal plan repository.
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            storage: MultiDocStorage::new(data_dir.clone()),
            data_dir,
            group_override: None,
        }
    }

    /// Creates a new repository with a specific group override.
    #[allow(dead_code)]
    pub fn with_group(data_dir: PathBuf, group_name: &str) -> Self {
        Self {
            storage: MultiDocStorage::new(data_dir.clone()),
            data_dir,
            group_override: Some(group_name.to_string()),
        }
    }

    /// Resolves the mealplans document ID from the current group context.
    fn resolve_doc_id(&self) -> Result<DocumentId, SyncMealPlanError> {
        let ctx = resolve_group_context(&self.data_dir, self.group_override.as_deref())?;
        Ok(ctx.mealplans_doc_id)
    }

    /// Loads the mealplans Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<(AutoCommit, DocumentId), SyncMealPlanError> {
        let doc_id = self.resolve_doc_id()?;
        let doc = match self.storage.load(&doc_id)? {
            Some(bytes) => AutoCommit::load(&bytes).map_err(|e| {
                SyncMealPlanError::Reader(ReaderError::AutomergeError(e.to_string()))
            })?,
            None => AutoCommit::new(),
        };
        Ok((doc, doc_id))
    }

    /// Saves the document to storage.
    fn save_doc(&self, doc: &mut AutoCommit, doc_id: &DocumentId) -> Result<(), SyncMealPlanError> {
        let bytes = doc.save();
        self.storage.save(doc_id, &bytes)?;
        Ok(())
    }

    /// Creates a new meal plan.
    pub fn create(&self, plan: &MealPlan) -> Result<MealPlan, SyncMealPlanError> {
        let (mut doc, doc_id) = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_mealplan(&mut doc, plan);

        // Save
        self.save_doc(&mut doc, &doc_id)?;

        // Return the plan (read back to confirm)
        self.get_by_id(plan.id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(plan.id.to_string()))
    }

    /// Updates an existing meal plan.
    pub fn update(&self, plan: &MealPlan) -> Result<MealPlan, SyncMealPlanError> {
        let (mut doc, doc_id) = self.load_or_create_doc()?;

        // Write updated plan to Automerge (overwrites existing)
        writer::write_mealplan(&mut doc, plan);

        // Save
        self.save_doc(&mut doc, &doc_id)?;

        // Return the updated plan
        self.get_by_id(plan.id)?
            .ok_or_else(|| SyncMealPlanError::NotFound(plan.id.to_string()))
    }

    /// Deletes a meal plan by ID.
    #[allow(dead_code)]
    pub fn delete(&self, id: Uuid) -> Result<(), SyncMealPlanError> {
        let (mut doc, doc_id) = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_mealplan(&mut doc, id);

        // Save
        self.save_doc(&mut doc, &doc_id)?;

        Ok(())
    }

    /// Gets a meal plan by ID.
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<MealPlan>, SyncMealPlanError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(read_mealplan_by_id(&doc, id)?)
    }

    /// Lists all meal plans.
    #[allow(dead_code)]
    pub fn list(&self) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(read_all_mealplans(&doc)?)
    }

    /// Lists meal plans within a date range.
    pub fn list_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(list_mealplans_by_date_range(&doc, from, to)?)
    }

    /// Gets meal plans for a specific date.
    #[allow(dead_code)]
    pub fn get_by_date(&self, date: NaiveDate) -> Result<Vec<MealPlan>, SyncMealPlanError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(get_mealplans_by_date(&doc, date)?)
    }

    /// Gets a meal plan for a specific date and meal type.
    pub fn get_by_date_and_type(
        &self,
        date: NaiveDate,
        meal_type: MealType,
    ) -> Result<Option<MealPlan>, SyncMealPlanError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(get_mealplan_by_date_and_type(&doc, date, meal_type)?)
    }

    /// Adds a dish to a meal plan.
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::writer;
    use tempfile::TempDir;

    /// Test helper that bypasses identity/group requirements.
    struct TestMealPlanRepo {
        storage: MultiDocStorage,
        doc_id: DocumentId,
    }

    impl TestMealPlanRepo {
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

        fn create(&self, plan: &MealPlan) -> MealPlan {
            let mut doc = self.load_or_create_doc();
            writer::write_mealplan(&mut doc, plan);
            self.save_doc(&mut doc);
            self.get_by_id(plan.id).unwrap()
        }

        fn update(&self, plan: &MealPlan) -> MealPlan {
            let mut doc = self.load_or_create_doc();
            writer::write_mealplan(&mut doc, plan);
            self.save_doc(&mut doc);
            self.get_by_id(plan.id).unwrap()
        }

        fn get_by_id(&self, id: Uuid) -> Option<MealPlan> {
            let doc = self.load_or_create_doc();
            read_mealplan_by_id(&doc, id).unwrap()
        }

        fn list(&self) -> Vec<MealPlan> {
            let doc = self.load_or_create_doc();
            read_all_mealplans(&doc).unwrap()
        }

        fn list_range(&self, from: NaiveDate, to: NaiveDate) -> Vec<MealPlan> {
            let doc = self.load_or_create_doc();
            list_mealplans_by_date_range(&doc, from, to).unwrap()
        }

        fn get_by_date(&self, date: NaiveDate) -> Vec<MealPlan> {
            let doc = self.load_or_create_doc();
            get_mealplans_by_date(&doc, date).unwrap()
        }

        fn add_dish(&self, plan_id: Uuid, dish_id: Uuid) -> Result<(), SyncMealPlanError> {
            let mut plan = self
                .get_by_id(plan_id)
                .ok_or_else(|| SyncMealPlanError::NotFound(plan_id.to_string()))?;

            if plan.dish_ids.contains(&dish_id) {
                return Err(SyncMealPlanError::DishAlreadyInPlan(dish_id.to_string()));
            }

            plan.dish_ids.push(dish_id);
            self.update(&plan);
            Ok(())
        }

        fn remove_dish(&self, plan_id: Uuid, dish_id: Uuid) -> Result<(), SyncMealPlanError> {
            let mut plan = self
                .get_by_id(plan_id)
                .ok_or_else(|| SyncMealPlanError::NotFound(plan_id.to_string()))?;

            if !plan.dish_ids.contains(&dish_id) {
                return Err(SyncMealPlanError::DishNotInPlan(dish_id.to_string()));
            }

            plan.dish_ids.retain(|id| id != &dish_id);
            self.update(&plan);
            Ok(())
        }
    }

    #[test]
    fn test_create_and_list() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealPlanRepo::new(&temp_dir);

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        repo.create(&plan);

        let plans = repo.list();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].date, date);
    }

    #[test]
    fn test_get_by_date() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealPlanRepo::new(&temp_dir);

        let date1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();

        repo.create(&MealPlan::new(
            date1,
            MealType::Breakfast,
            "Breakfast",
            "chef",
        ));
        repo.create(&MealPlan::new(date1, MealType::Dinner, "Dinner", "chef"));
        repo.create(&MealPlan::new(date2, MealType::Lunch, "Lunch", "chef"));

        let plans = repo.get_by_date(date1);
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn test_list_range() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealPlanRepo::new(&temp_dir);

        let date1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let date3 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        repo.create(&MealPlan::new(date1, MealType::Dinner, "Dinner 1", "chef"));
        repo.create(&MealPlan::new(date2, MealType::Dinner, "Dinner 2", "chef"));
        repo.create(&MealPlan::new(date3, MealType::Dinner, "Dinner 3", "chef"));

        let from = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let to = NaiveDate::from_ymd_opt(2025, 1, 8).unwrap();
        let plans = repo.list_range(from, to);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].date, date2);
    }

    #[test]
    fn test_add_dish() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealPlanRepo::new(&temp_dir);

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        repo.create(&plan);

        let dish_id = Uuid::new_v4();
        repo.add_dish(plan.id, dish_id).unwrap();

        let updated = repo.get_by_id(plan.id).unwrap();
        assert_eq!(updated.dish_ids.len(), 1);
        assert!(updated.dish_ids.contains(&dish_id));
    }

    #[test]
    fn test_add_dish_already_in_plan() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealPlanRepo::new(&temp_dir);

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        repo.create(&plan);

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
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealPlanRepo::new(&temp_dir);

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let dish_id1 = Uuid::new_v4();
        let dish_id2 = Uuid::new_v4();
        let mut plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        plan.dish_ids = vec![dish_id1, dish_id2];
        repo.create(&plan);

        repo.remove_dish(plan.id, dish_id1).unwrap();

        let updated = repo.get_by_id(plan.id).unwrap();
        assert_eq!(updated.dish_ids.len(), 1);
        assert!(!updated.dish_ids.contains(&dish_id1));
        assert!(updated.dish_ids.contains(&dish_id2));
    }

    #[test]
    fn test_remove_dish_not_in_plan() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealPlanRepo::new(&temp_dir);

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner Plan", "chef");
        repo.create(&plan);

        let dish_id = Uuid::new_v4();
        let result = repo.remove_dish(plan.id, dish_id);
        assert!(matches!(result, Err(SyncMealPlanError::DishNotInPlan(_))));
    }

    #[test]
    fn test_update_mealplan() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestMealPlanRepo::new(&temp_dir);

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mut plan = MealPlan::new(date, MealType::Dinner, "Original Title", "chef");
        repo.create(&plan);

        plan.title = "Updated Title".to_string();
        let updated = repo.update(&plan);

        assert_eq!(updated.title, "Updated Title");
    }
}
