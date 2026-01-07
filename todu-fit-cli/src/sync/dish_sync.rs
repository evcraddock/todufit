//! Sync-aware dish repository that writes to Automerge and projects to SQLite.
//!
//! This module provides a repository layer that:
//! 1. Writes changes to Automerge documents (source of truth)
//! 2. Projects changes to SQLite (for fast queries)
//! 3. Reads from SQLite for queries

use automerge::AutoCommit;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{Dish, Ingredient};
use crate::sync::projection::DishProjection;
use crate::sync::storage::{DocType, DocumentStorage, StorageError};
use crate::sync::writer;

/// Error type for sync dish operations.
#[derive(Debug)]
pub enum SyncDishError {
    /// Storage error (loading/saving Automerge docs).
    Storage(StorageError),
    /// Projection error (syncing to SQLite).
    Projection(crate::sync::projection::ProjectionError),
    /// SQLite error (queries).
    Sqlite(sqlx::Error),
    /// Dish not found.
    NotFound(String),
}

impl std::fmt::Display for SyncDishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncDishError::Storage(e) => write!(f, "Storage error: {}", e),
            SyncDishError::Projection(e) => write!(f, "Projection error: {}", e),
            SyncDishError::Sqlite(e) => write!(f, "SQLite error: {}", e),
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

impl From<crate::sync::projection::ProjectionError> for SyncDishError {
    fn from(e: crate::sync::projection::ProjectionError) -> Self {
        SyncDishError::Projection(e)
    }
}

impl From<sqlx::Error> for SyncDishError {
    fn from(e: sqlx::Error) -> Self {
        SyncDishError::Sqlite(e)
    }
}

/// Sync-aware dish repository.
///
/// Writes go to Automerge first, then project to SQLite.
/// Reads come from SQLite for fast queries.
pub struct SyncDishRepository {
    storage: DocumentStorage,
    pool: SqlitePool,
}

impl SyncDishRepository {
    /// Creates a new sync dish repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            storage: DocumentStorage::new(),
            pool,
        }
    }

    /// Creates a new sync dish repository with custom storage.
    #[cfg(test)]
    pub fn with_storage(storage: DocumentStorage, pool: SqlitePool) -> Self {
        Self { storage, pool }
    }

    /// Loads the dishes Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<AutoCommit, SyncDishError> {
        match self.storage.load(DocType::Dishes)? {
            Some(doc) => Ok(doc),
            None => Ok(AutoCommit::new()),
        }
    }

    /// Saves the document and projects to SQLite.
    async fn save_and_project(&self, doc: &mut AutoCommit) -> Result<(), SyncDishError> {
        // Save to Automerge storage
        self.storage.save(DocType::Dishes, doc)?;

        // Project to SQLite
        DishProjection::project_all(doc, &self.pool).await?;

        Ok(())
    }

    /// Creates a new dish.
    pub async fn create(&self, dish: &Dish) -> Result<Dish, SyncDishError> {
        let mut doc = self.load_or_create_doc()?;

        // Write to Automerge
        writer::write_dish(&mut doc, dish);

        // Save and project
        self.save_and_project(&mut doc).await?;

        // Return the dish (read from SQLite to confirm)
        self.get_by_id(dish.id)
            .await?
            .ok_or_else(|| SyncDishError::NotFound(dish.id.to_string()))
    }

    /// Updates an existing dish.
    pub async fn update(&self, dish: &Dish) -> Result<Dish, SyncDishError> {
        let mut doc = self.load_or_create_doc()?;

        // Write updated dish to Automerge (overwrites existing)
        writer::write_dish(&mut doc, dish);

        // Save and project
        self.save_and_project(&mut doc).await?;

        // Return the updated dish
        self.get_by_id(dish.id)
            .await?
            .ok_or_else(|| SyncDishError::NotFound(dish.id.to_string()))
    }

    /// Deletes a dish.
    pub async fn delete(&self, id: Uuid) -> Result<(), SyncDishError> {
        let mut doc = self.load_or_create_doc()?;

        // Delete from Automerge
        writer::delete_dish(&mut doc, id);

        // Save and project
        self.save_and_project(&mut doc).await?;

        Ok(())
    }

    /// Adds an ingredient to a dish.
    pub async fn add_ingredient(
        &self,
        dish_id: Uuid,
        ingredient: &Ingredient,
    ) -> Result<(), SyncDishError> {
        // Get the current dish
        let mut dish = self
            .get_by_id(dish_id)
            .await?
            .ok_or_else(|| SyncDishError::NotFound(dish_id.to_string()))?;

        // Add the ingredient
        dish.ingredients.push(ingredient.clone());
        dish.updated_at = chrono::Utc::now();

        // Update via Automerge
        self.update(&dish).await?;

        Ok(())
    }

    /// Removes an ingredient from a dish.
    pub async fn remove_ingredient(
        &self,
        dish_id: Uuid,
        ingredient_name: &str,
    ) -> Result<(), SyncDishError> {
        // Get the current dish
        let mut dish = self
            .get_by_id(dish_id)
            .await?
            .ok_or_else(|| SyncDishError::NotFound(dish_id.to_string()))?;

        // Remove the ingredient (case-insensitive)
        let name_lower = ingredient_name.to_lowercase();
        dish.ingredients
            .retain(|i| i.name.to_lowercase() != name_lower);
        dish.updated_at = chrono::Utc::now();

        // Update via Automerge
        self.update(&dish).await?;

        Ok(())
    }

    // ========== Read operations (from SQLite) ==========

    /// Gets a dish by ID.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Dish>, SyncDishError> {
        use crate::db::DishRepository;
        let repo = DishRepository::new(self.pool.clone());
        Ok(repo.get_by_id(id).await?)
    }

    /// Gets a dish by name.
    pub async fn get_by_name(&self, name: &str) -> Result<Option<Dish>, SyncDishError> {
        use crate::db::DishRepository;
        let repo = DishRepository::new(self.pool.clone());
        Ok(repo.get_by_name(name).await?)
    }

    /// Lists all dishes.
    pub async fn list(&self) -> Result<Vec<Dish>, SyncDishError> {
        use crate::db::DishRepository;
        let repo = DishRepository::new(self.pool.clone());
        Ok(repo.list().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    async fn setup() -> (SyncDishRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        let repo = SyncDishRepository::with_storage(storage, pool);
        (repo, temp_dir)
    }

    #[tokio::test]
    async fn test_create_dish() {
        let (repo, _temp) = setup().await;

        let dish = Dish::new("Test Pasta", "chef");
        let created = repo.create(&dish).await.unwrap();

        assert_eq!(created.name, "Test Pasta");
        assert_eq!(created.id, dish.id);
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let (repo, _temp) = setup().await;

        repo.create(&Dish::new("Dish A", "chef")).await.unwrap();
        repo.create(&Dish::new("Dish B", "chef")).await.unwrap();

        let dishes = repo.list().await.unwrap();
        assert_eq!(dishes.len(), 2);
    }

    #[tokio::test]
    async fn test_update_dish() {
        let (repo, _temp) = setup().await;

        let dish = Dish::new("Original", "chef");
        repo.create(&dish).await.unwrap();

        let mut updated = dish.clone();
        updated.name = "Updated".to_string();
        let result = repo.update(&updated).await.unwrap();

        assert_eq!(result.name, "Updated");
    }

    #[tokio::test]
    async fn test_delete_dish() {
        let (repo, _temp) = setup().await;

        let dish = Dish::new("To Delete", "chef");
        let id = dish.id;
        repo.create(&dish).await.unwrap();

        repo.delete(id).await.unwrap();

        let result = repo.get_by_id(id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_add_ingredient() {
        let (repo, _temp) = setup().await;

        let dish = Dish::new("Pasta", "chef");
        repo.create(&dish).await.unwrap();

        let ingredient = Ingredient::new("tomato", 2.0, "pieces");
        repo.add_ingredient(dish.id, &ingredient).await.unwrap();

        let updated = repo.get_by_id(dish.id).await.unwrap().unwrap();
        assert_eq!(updated.ingredients.len(), 1);
        assert_eq!(updated.ingredients[0].name, "tomato");
    }

    #[tokio::test]
    async fn test_remove_ingredient() {
        let (repo, _temp) = setup().await;

        let dish = Dish::new("Pasta", "chef").with_ingredients(vec![
            Ingredient::new("pasta", 200.0, "g"),
            Ingredient::new("sauce", 1.0, "cup"),
        ]);
        repo.create(&dish).await.unwrap();

        repo.remove_ingredient(dish.id, "PASTA").await.unwrap(); // case-insensitive

        let updated = repo.get_by_id(dish.id).await.unwrap().unwrap();
        assert_eq!(updated.ingredients.len(), 1);
        assert_eq!(updated.ingredients[0].name, "sauce");
    }

    #[tokio::test]
    async fn test_automerge_doc_persists() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());

        // Create a dish
        let dish = Dish::new("Persistent Dish", "chef");
        let dish_id = dish.id;
        {
            let repo = SyncDishRepository::with_storage(storage.clone(), pool.clone());
            repo.create(&dish).await.unwrap();
        }

        // Verify Automerge doc exists and contains the dish
        let doc = storage.load(DocType::Dishes).unwrap().unwrap();
        use automerge::ReadDoc;
        assert!(doc
            .get(automerge::ROOT, &dish_id.to_string())
            .unwrap()
            .is_some());
    }
}
