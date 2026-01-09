//! Sync-aware dish repository that reads/writes Automerge documents.
//!
//! This module provides a repository layer that:
//! 1. Uses the current group's dishes document when identity is configured
//! 2. Falls back to legacy DocType-based storage otherwise
//! 3. Reads directly from Automerge documents (in-memory queries)

use automerge::AutoCommit;
use uuid::Uuid;

use todu_fit_core::{DocumentId, MultiDocStorage};

use crate::config::Config;
use crate::models::{Dish, Ingredient};
use crate::sync::group_context::{is_identity_ready, resolve_group_context, GroupContextError};
use crate::sync::reader::{
    filter_dishes_by_tag, find_dish_by_name, read_all_dishes, read_dish_by_id,
    search_dishes_by_name, ReaderError,
};
use crate::sync::storage::{DocType, DocumentStorage, StorageError};
use crate::sync::writer;

/// Error type for sync dish operations.
#[derive(Debug)]
pub enum SyncDishError {
    /// Storage error (loading/saving Automerge docs).
    Storage(StorageError),
    /// Reader error (parsing Automerge data).
    Reader(ReaderError),
    /// Dish not found.
    NotFound(String),
    /// Group context error.
    GroupContext(GroupContextError),
    /// Multi-storage error.
    MultiStorage(todu_fit_core::MultiStorageError),
}

impl std::fmt::Display for SyncDishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncDishError::Storage(e) => write!(f, "Storage error: {}", e),
            SyncDishError::Reader(e) => write!(f, "Reader error: {}", e),
            SyncDishError::NotFound(id) => write!(f, "Dish not found: {}", id),
            SyncDishError::GroupContext(e) => write!(f, "{}", e),
            SyncDishError::MultiStorage(e) => write!(f, "Storage error: {}", e),
        }
    }
}

impl std::error::Error for SyncDishError {}

impl From<StorageError> for SyncDishError {
    fn from(e: StorageError) -> Self {
        SyncDishError::Storage(e)
    }
}

impl From<ReaderError> for SyncDishError {
    fn from(e: ReaderError) -> Self {
        SyncDishError::Reader(e)
    }
}

impl From<GroupContextError> for SyncDishError {
    fn from(e: GroupContextError) -> Self {
        SyncDishError::GroupContext(e)
    }
}

impl From<todu_fit_core::MultiStorageError> for SyncDishError {
    fn from(e: todu_fit_core::MultiStorageError) -> Self {
        SyncDishError::MultiStorage(e)
    }
}

/// Sync-aware dish repository.
///
/// All operations work directly with Automerge documents.
/// Uses the current group's dishes document when identity is configured,
/// otherwise falls back to legacy DocType-based storage.
pub struct SyncDishRepository {
    legacy_storage: DocumentStorage,
    multi_storage: MultiDocStorage,
    group_override: Option<String>,
}

#[allow(dead_code)]
impl SyncDishRepository {
    /// Creates a new sync dish repository.
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

    /// Creates a new sync dish repository with custom storage (for testing).
    #[cfg(test)]
    pub fn with_storage(storage: DocumentStorage) -> Self {
        Self {
            legacy_storage: storage,
            multi_storage: MultiDocStorage::new(Config::default_data_dir()),
            group_override: None,
        }
    }

    /// Resolves the dishes document ID.
    /// Returns None if using legacy mode, Some(doc_id) if using group mode.
    fn resolve_doc_id(&self) -> Option<DocumentId> {
        if !is_identity_ready() {
            return None;
        }

        resolve_group_context(self.group_override.as_deref())
            .ok()
            .map(|ctx| ctx.dishes_doc_id)
    }

    /// Loads the dishes Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<AutoCommit, SyncDishError> {
        if let Some(doc_id) = self.resolve_doc_id() {
            // Group mode: use multi-storage with document ID
            match self.multi_storage.load(&doc_id)? {
                Some(bytes) => AutoCommit::load(&bytes)
                    .map_err(|e| SyncDishError::Reader(ReaderError::AutomergeError(e.to_string()))),
                None => Ok(AutoCommit::new()),
            }
        } else {
            // Legacy mode: use DocType-based storage
            match self.legacy_storage.load(DocType::Dishes)? {
                Some(doc) => Ok(doc),
                None => Ok(AutoCommit::new()),
            }
        }
    }

    /// Saves the document to storage.
    fn save_doc(&self, doc: &mut AutoCommit) -> Result<(), SyncDishError> {
        if let Some(doc_id) = self.resolve_doc_id() {
            // Group mode: use multi-storage
            let bytes = doc.save();
            self.multi_storage.save(&doc_id, &bytes)?;
        } else {
            // Legacy mode
            self.legacy_storage.save(DocType::Dishes, doc)?;
        }
        Ok(())
    }

    /// Creates a new dish.
    pub fn create(&self, dish: &Dish) -> Result<Dish, SyncDishError> {
        let mut doc = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_dish(&mut doc, dish);

        // Save
        self.save_doc(&mut doc)?;

        // Return the dish (read back to confirm)
        self.get_by_id(dish.id)?
            .ok_or_else(|| SyncDishError::NotFound(dish.id.to_string()))
    }

    /// Updates an existing dish.
    pub fn update(&self, dish: &Dish) -> Result<Dish, SyncDishError> {
        let mut doc = self.load_or_create_doc()?;

        // Write updated dish to Automerge (overwrites existing)
        writer::write_dish(&mut doc, dish);

        // Save
        self.save_doc(&mut doc)?;

        // Return the updated dish
        self.get_by_id(dish.id)?
            .ok_or_else(|| SyncDishError::NotFound(dish.id.to_string()))
    }

    /// Deletes a dish by ID.
    pub fn delete(&self, id: Uuid) -> Result<(), SyncDishError> {
        let mut doc = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_dish(&mut doc, id);

        // Save
        self.save_doc(&mut doc)?;

        Ok(())
    }

    /// Gets a dish by ID.
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<Dish>, SyncDishError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_dish_by_id(&doc, id)?)
    }

    /// Gets a dish by name (case-insensitive).
    pub fn get_by_name(&self, name: &str) -> Result<Option<Dish>, SyncDishError> {
        let doc = self.load_or_create_doc()?;
        Ok(find_dish_by_name(&doc, name)?)
    }

    /// Lists all dishes.
    pub fn list(&self) -> Result<Vec<Dish>, SyncDishError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_all_dishes(&doc)?)
    }

    /// Adds an ingredient to a dish.
    pub fn add_ingredient(
        &self,
        dish_id: Uuid,
        ingredient: Ingredient,
    ) -> Result<(), SyncDishError> {
        // Get current dish
        let mut dish = self
            .get_by_id(dish_id)?
            .ok_or_else(|| SyncDishError::NotFound(dish_id.to_string()))?;

        // Add ingredient
        dish.ingredients.push(ingredient);

        // Update
        self.update(&dish)?;

        Ok(())
    }

    /// Removes an ingredient from a dish by name.
    pub fn remove_ingredient(
        &self,
        dish_id: Uuid,
        ingredient_name: &str,
    ) -> Result<(), SyncDishError> {
        // Get current dish
        let mut dish = self
            .get_by_id(dish_id)?
            .ok_or_else(|| SyncDishError::NotFound(dish_id.to_string()))?;

        // Remove ingredient (case-insensitive)
        let name_lower = ingredient_name.to_lowercase();
        dish.ingredients
            .retain(|i| i.name.to_lowercase() != name_lower);

        // Update
        self.update(&dish)?;

        Ok(())
    }

    /// Searches dishes by name (partial, case-insensitive).
    pub fn search(&self, query: &str) -> Result<Vec<Dish>, SyncDishError> {
        let doc = self.load_or_create_doc()?;
        Ok(search_dishes_by_name(&doc, query)?)
    }

    /// Filters dishes by tag.
    pub fn filter_by_tag(&self, tag: &str) -> Result<Vec<Dish>, SyncDishError> {
        let doc = self.load_or_create_doc()?;
        Ok(filter_dishes_by_tag(&doc, tag)?)
    }
}

impl Default for SyncDishRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_repo() -> (SyncDishRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        let repo = SyncDishRepository::with_storage(storage);
        (repo, temp_dir)
    }

    #[test]
    fn test_create_and_get() {
        let (repo, _temp) = test_repo();

        let dish = Dish::new("Test Pasta", "chef");
        let created = repo.create(&dish).unwrap();

        assert_eq!(created.name, "Test Pasta");
        assert_eq!(created.created_by, "chef");

        // Get by ID
        let fetched = repo.get_by_id(dish.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Test Pasta");

        // Get by name
        let fetched = repo.get_by_name("test pasta").unwrap().unwrap();
        assert_eq!(fetched.id, dish.id);
    }

    #[test]
    fn test_update() {
        let (repo, _temp) = test_repo();

        let mut dish = Dish::new("Original Name", "chef");
        repo.create(&dish).unwrap();

        dish.name = "Updated Name".to_string();
        let updated = repo.update(&dish).unwrap();

        assert_eq!(updated.name, "Updated Name");

        // Verify the update persisted
        let fetched = repo.get_by_id(dish.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Updated Name");
    }

    #[test]
    fn test_delete() {
        let (repo, _temp) = test_repo();

        let dish = Dish::new("To Delete", "chef");
        repo.create(&dish).unwrap();

        // Verify it exists
        assert!(repo.get_by_id(dish.id).unwrap().is_some());

        // Delete
        repo.delete(dish.id).unwrap();

        // Verify it's gone
        assert!(repo.get_by_id(dish.id).unwrap().is_none());
    }

    #[test]
    fn test_list() {
        let (repo, _temp) = test_repo();

        repo.create(&Dish::new("Dish 1", "chef")).unwrap();
        repo.create(&Dish::new("Dish 2", "chef")).unwrap();
        repo.create(&Dish::new("Dish 3", "chef")).unwrap();

        let dishes = repo.list().unwrap();
        assert_eq!(dishes.len(), 3);
    }

    #[test]
    fn test_search() {
        let (repo, _temp) = test_repo();

        repo.create(&Dish::new("Chicken Pasta", "chef")).unwrap();
        repo.create(&Dish::new("Beef Stew", "chef")).unwrap();
        repo.create(&Dish::new("Chicken Soup", "chef")).unwrap();

        let results = repo.search("chicken").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_filter_by_tag() {
        let (repo, _temp) = test_repo();

        let mut dish1 = Dish::new("Dish 1", "chef");
        dish1.tags = vec!["keto".to_string(), "quick".to_string()];
        repo.create(&dish1).unwrap();

        let mut dish2 = Dish::new("Dish 2", "chef");
        dish2.tags = vec!["vegetarian".to_string()];
        repo.create(&dish2).unwrap();

        let mut dish3 = Dish::new("Dish 3", "chef");
        dish3.tags = vec!["keto".to_string()];
        repo.create(&dish3).unwrap();

        let results = repo.filter_by_tag("keto").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_add_ingredient() {
        let (repo, _temp) = test_repo();

        let dish = Dish::new("Test Dish", "chef");
        repo.create(&dish).unwrap();

        let ingredient = Ingredient::new("Salt", 1.0, "tsp");
        repo.add_ingredient(dish.id, ingredient).unwrap();

        let fetched = repo.get_by_id(dish.id).unwrap().unwrap();
        assert_eq!(fetched.ingredients.len(), 1);
        assert_eq!(fetched.ingredients[0].name, "Salt");
    }

    #[test]
    fn test_remove_ingredient() {
        let (repo, _temp) = test_repo();

        let mut dish = Dish::new("Test Dish", "chef");
        dish.ingredients = vec![
            Ingredient::new("Salt", 1.0, "tsp"),
            Ingredient::new("Pepper", 0.5, "tsp"),
        ];
        repo.create(&dish).unwrap();

        repo.remove_ingredient(dish.id, "salt").unwrap();

        let fetched = repo.get_by_id(dish.id).unwrap().unwrap();
        assert_eq!(fetched.ingredients.len(), 1);
        assert_eq!(fetched.ingredients[0].name, "Pepper");
    }
}
