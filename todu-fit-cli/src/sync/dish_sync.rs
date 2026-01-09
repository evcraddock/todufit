//! Sync-aware dish repository that reads/writes Automerge documents.
//!
//! This module provides a repository layer that:
//! 1. Writes changes to Automerge documents (source of truth)
//! 2. Reads directly from Automerge documents (in-memory queries)

use automerge::AutoCommit;
use uuid::Uuid;

use crate::models::{Dish, Ingredient};
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
}

impl std::fmt::Display for SyncDishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncDishError::Storage(e) => write!(f, "Storage error: {}", e),
            SyncDishError::Reader(e) => write!(f, "Reader error: {}", e),
            SyncDishError::NotFound(id) => write!(f, "Dish not found: {}", id),
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

/// Sync-aware dish repository.
///
/// All operations work directly with Automerge documents.
pub struct SyncDishRepository {
    storage: DocumentStorage,
}
#[allow(dead_code)]
impl SyncDishRepository {
    /// Creates a new sync dish repository.
    pub fn new() -> Self {
        Self {
            storage: DocumentStorage::new(),
        }
    }

    /// Creates a new sync dish repository with custom storage.
    #[cfg(test)]
    pub fn with_storage(storage: DocumentStorage) -> Self {
        Self { storage }
    }

    /// Loads the dishes Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<AutoCommit, SyncDishError> {
        match self.storage.load(DocType::Dishes)? {
            Some(doc) => Ok(doc),
            None => Ok(AutoCommit::new()),
        }
    }

    /// Saves the document to storage.
    fn save_doc(&self, doc: &mut AutoCommit) -> Result<(), SyncDishError> {
        self.storage.save(DocType::Dishes, doc)?;
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

    /// Deletes a dish.
    pub fn delete(&self, id: Uuid) -> Result<(), SyncDishError> {
        let mut doc = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_dish(&mut doc, id);

        // Save
        self.save_doc(&mut doc)?;

        Ok(())
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
        dish.updated_at = chrono::Utc::now();

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

        // Remove ingredient
        let name_lower = ingredient_name.to_lowercase();
        dish.ingredients
            .retain(|i| i.name.to_lowercase() != name_lower);
        dish.updated_at = chrono::Utc::now();

        // Update
        self.update(&dish)?;

        Ok(())
    }

    // ========== Read operations (from Automerge) ==========

    /// Gets a dish by ID.
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<Dish>, SyncDishError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_dish_by_id(&doc, id)?)
    }

    /// Gets a dish by name (case-insensitive exact match).
    pub fn get_by_name(&self, name: &str) -> Result<Option<Dish>, SyncDishError> {
        let doc = self.load_or_create_doc()?;
        Ok(find_dish_by_name(&doc, name)?)
    }

    /// Lists all dishes.
    pub fn list(&self) -> Result<Vec<Dish>, SyncDishError> {
        let doc = self.load_or_create_doc()?;
        Ok(read_all_dishes(&doc)?)
    }

    /// Searches dishes by name (partial match).
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

    fn setup() -> (SyncDishRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        let repo = SyncDishRepository::with_storage(storage);
        (repo, temp_dir)
    }

    #[test]
    fn test_create_dish() {
        let (repo, _temp) = setup();

        let dish = Dish::new("Test Pasta", "chef");
        let created = repo.create(&dish).unwrap();

        assert_eq!(created.name, "Test Pasta");
        assert_eq!(created.id, dish.id);
    }

    #[test]
    fn test_create_and_list() {
        let (repo, _temp) = setup();

        repo.create(&Dish::new("Pasta", "chef")).unwrap();
        repo.create(&Dish::new("Salad", "chef")).unwrap();

        let dishes = repo.list().unwrap();
        assert_eq!(dishes.len(), 2);
    }

    #[test]
    fn test_update_dish() {
        let (repo, _temp) = setup();

        let dish = Dish::new("Original", "chef");
        repo.create(&dish).unwrap();

        let mut updated = dish.clone();
        updated.name = "Updated".to_string();
        let result = repo.update(&updated).unwrap();

        assert_eq!(result.name, "Updated");
    }

    #[test]
    fn test_delete_dish() {
        let (repo, _temp) = setup();

        let dish = Dish::new("To Delete", "chef");
        let id = dish.id;
        repo.create(&dish).unwrap();

        repo.delete(id).unwrap();

        let result = repo.get_by_id(id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_by_name() {
        let (repo, _temp) = setup();

        repo.create(&Dish::new("Pasta Carbonara", "chef")).unwrap();

        let dish = repo.get_by_name("pasta carbonara").unwrap();
        assert!(dish.is_some());
        assert_eq!(dish.unwrap().name, "Pasta Carbonara");
    }

    #[test]
    fn test_search() {
        let (repo, _temp) = setup();

        repo.create(&Dish::new("Pasta Carbonara", "chef")).unwrap();
        repo.create(&Dish::new("Pasta Bolognese", "chef")).unwrap();
        repo.create(&Dish::new("Caesar Salad", "chef")).unwrap();

        let results = repo.search("pasta").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_filter_by_tag() {
        let (repo, _temp) = setup();

        let mut italian = Dish::new("Pasta", "chef");
        italian.tags = vec!["italian".to_string()];
        repo.create(&italian).unwrap();

        let mut mexican = Dish::new("Tacos", "chef");
        mexican.tags = vec!["mexican".to_string()];
        repo.create(&mexican).unwrap();

        let results = repo.filter_by_tag("italian").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Pasta");
    }

    #[test]
    fn test_add_ingredient() {
        let (repo, _temp) = setup();

        let dish = Dish::new("Pasta", "chef");
        repo.create(&dish).unwrap();

        repo.add_ingredient(dish.id, Ingredient::new("pasta", 200.0, "g"))
            .unwrap();

        let updated = repo.get_by_id(dish.id).unwrap().unwrap();
        assert_eq!(updated.ingredients.len(), 1);
        assert_eq!(updated.ingredients[0].name, "pasta");
    }

    #[test]
    fn test_remove_ingredient() {
        let (repo, _temp) = setup();

        let mut dish = Dish::new("Pasta", "chef");
        dish.ingredients = vec![
            Ingredient::new("pasta", 200.0, "g"),
            Ingredient::new("sauce", 100.0, "ml"),
        ];
        repo.create(&dish).unwrap();

        repo.remove_ingredient(dish.id, "pasta").unwrap();

        let updated = repo.get_by_id(dish.id).unwrap().unwrap();
        assert_eq!(updated.ingredients.len(), 1);
        assert_eq!(updated.ingredients[0].name, "sauce");
    }

    #[test]
    fn test_automerge_doc_persists() {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());

        let dish = Dish::new("Persistent Dish", "chef");
        let dish_id = dish.id;

        {
            let repo = SyncDishRepository::with_storage(storage.clone());
            repo.create(&dish).unwrap();
        }

        // Create new repo instance and verify dish is still there
        let repo = SyncDishRepository::with_storage(storage);
        let loaded = repo.get_by_id(dish_id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, "Persistent Dish");
    }
}
