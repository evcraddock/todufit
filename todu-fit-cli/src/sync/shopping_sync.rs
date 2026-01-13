//! Sync-aware shopping cart repository that reads/writes Automerge documents.
//!
//! This module provides a repository layer that uses the current group's
//! shopping carts document. Identity must be initialized first.

use std::path::PathBuf;

use automerge::AutoCommit;

use todu_fit_core::{write_shopping_cart, DocumentId, MultiDocStorage, ShoppingCart};

use crate::sync::group_context::{resolve_group_context, GroupContextError};
use crate::sync::reader::{read_all_shopping_carts, read_shopping_cart_by_week, ReaderError};

/// Error type for sync shopping cart operations.
#[derive(Debug)]
#[allow(dead_code)]
pub enum SyncShoppingError {
    /// Reader error (parsing Automerge data).
    Reader(ReaderError),
    /// Shopping cart not found.
    NotFound(String),
    /// Item not found.
    ItemNotFound(String),
    /// Item is not a manual item (cannot remove).
    NotManualItem(String),
    /// Group context error.
    GroupContext(GroupContextError),
    /// Multi-storage error.
    MultiStorage(todu_fit_core::MultiStorageError),
}

impl std::fmt::Display for SyncShoppingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncShoppingError::Reader(e) => write!(f, "Reader error: {}", e),
            SyncShoppingError::NotFound(week) => {
                write!(f, "Shopping cart not found for week: {}", week)
            }
            SyncShoppingError::ItemNotFound(name) => write!(f, "Item not found: {}", name),
            SyncShoppingError::NotManualItem(name) => {
                write!(
                    f,
                    "Cannot remove '{}': only manual items can be removed",
                    name
                )
            }
            SyncShoppingError::GroupContext(e) => write!(f, "{}", e),
            SyncShoppingError::MultiStorage(e) => write!(f, "Storage error: {}", e),
        }
    }
}

impl std::error::Error for SyncShoppingError {}

impl From<ReaderError> for SyncShoppingError {
    fn from(e: ReaderError) -> Self {
        SyncShoppingError::Reader(e)
    }
}

impl From<GroupContextError> for SyncShoppingError {
    fn from(e: GroupContextError) -> Self {
        SyncShoppingError::GroupContext(e)
    }
}

impl From<todu_fit_core::MultiStorageError> for SyncShoppingError {
    fn from(e: todu_fit_core::MultiStorageError) -> Self {
        SyncShoppingError::MultiStorage(e)
    }
}

/// Sync-aware shopping cart repository.
///
/// All operations work directly with Automerge documents.
/// Uses the current group's shopping carts document.
pub struct SyncShoppingRepository {
    storage: MultiDocStorage,
    data_dir: PathBuf,
    group_override: Option<String>,
}

impl SyncShoppingRepository {
    /// Creates a new sync shopping cart repository.
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

    /// Resolves the shopping carts document ID from the current group context.
    fn resolve_doc_id(&self) -> Result<DocumentId, SyncShoppingError> {
        let ctx = resolve_group_context(&self.data_dir, self.group_override.as_deref())?;
        Ok(ctx.shopping_carts_doc_id)
    }

    /// Loads the shopping carts Automerge document, or creates a new empty one.
    fn load_or_create_doc(&self) -> Result<(AutoCommit, DocumentId), SyncShoppingError> {
        let doc_id = self.resolve_doc_id()?;
        let doc = match self.storage.load(&doc_id)? {
            Some(bytes) => AutoCommit::load(&bytes).map_err(|e| {
                SyncShoppingError::Reader(ReaderError::AutomergeError(e.to_string()))
            })?,
            None => AutoCommit::new(),
        };
        Ok((doc, doc_id))
    }

    /// Saves the document to storage.
    fn save_doc(&self, doc: &mut AutoCommit, doc_id: &DocumentId) -> Result<(), SyncShoppingError> {
        let bytes = doc.save();
        self.storage.save(doc_id, &bytes)?;
        Ok(())
    }

    /// Gets a shopping cart for a specific week.
    /// Returns an empty cart if none exists.
    pub fn get_or_create(&self, week: &str) -> Result<ShoppingCart, SyncShoppingError> {
        let (doc, _) = self.load_or_create_doc()?;
        match read_shopping_cart_by_week(&doc, week)? {
            Some(cart) => Ok(cart),
            None => Ok(ShoppingCart::new(week)),
        }
    }

    /// Gets a shopping cart for a specific week.
    #[allow(dead_code)]
    pub fn get(&self, week: &str) -> Result<Option<ShoppingCart>, SyncShoppingError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(read_shopping_cart_by_week(&doc, week)?)
    }

    /// Lists all shopping carts (sorted newest first).
    #[allow(dead_code)]
    pub fn list(&self) -> Result<Vec<ShoppingCart>, SyncShoppingError> {
        let (doc, _) = self.load_or_create_doc()?;
        Ok(read_all_shopping_carts(&doc)?)
    }

    /// Saves a shopping cart.
    pub fn save(&self, cart: &ShoppingCart) -> Result<(), SyncShoppingError> {
        let (mut doc, doc_id) = self.load_or_create_doc()?;
        write_shopping_cart(&mut doc, cart);
        self.save_doc(&mut doc, &doc_id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use todu_fit_core::ManualItem;

    /// Test helper that bypasses identity/group requirements.
    struct TestShoppingRepo {
        storage: MultiDocStorage,
        doc_id: DocumentId,
    }

    impl TestShoppingRepo {
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

        fn get_or_create(&self, week: &str) -> ShoppingCart {
            let doc = self.load_or_create_doc();
            match read_shopping_cart_by_week(&doc, week).unwrap() {
                Some(cart) => cart,
                None => ShoppingCart::new(week),
            }
        }

        fn save(&self, cart: &ShoppingCart) {
            let mut doc = self.load_or_create_doc();
            write_shopping_cart(&mut doc, cart);
            self.save_doc(&mut doc);
        }
    }

    #[test]
    fn test_create_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestShoppingRepo::new(&temp_dir);

        let mut cart = repo.get_or_create("2026-01-11");
        cart.check("eggs");
        cart.add_manual_item(ManualItem::new("Paper towels"));
        repo.save(&cart);

        let loaded = repo.get_or_create("2026-01-11");
        assert!(loaded.is_checked("eggs"));
        assert_eq!(loaded.manual_items.len(), 1);
    }

    #[test]
    fn test_check_uncheck() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestShoppingRepo::new(&temp_dir);

        let mut cart = repo.get_or_create("2026-01-11");
        cart.check("milk");
        repo.save(&cart);

        let mut loaded = repo.get_or_create("2026-01-11");
        assert!(loaded.is_checked("milk"));

        loaded.uncheck("milk");
        repo.save(&loaded);

        let reloaded = repo.get_or_create("2026-01-11");
        assert!(!reloaded.is_checked("milk"));
    }

    #[test]
    fn test_manual_items() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestShoppingRepo::new(&temp_dir);

        let mut cart = repo.get_or_create("2026-01-11");
        cart.add_manual_item(ManualItem::with_quantity("Soap", "3", "bars"));
        repo.save(&cart);

        let loaded = repo.get_or_create("2026-01-11");
        let item = loaded.find_manual_item("Soap").unwrap();
        assert_eq!(item.quantity, Some("3".to_string()));
        assert_eq!(item.unit, Some("bars".to_string()));
    }

    #[test]
    fn test_remove_manual_item() {
        let temp_dir = TempDir::new().unwrap();
        let repo = TestShoppingRepo::new(&temp_dir);

        let mut cart = repo.get_or_create("2026-01-11");
        cart.add_manual_item(ManualItem::new("Test item"));
        repo.save(&cart);

        let mut loaded = repo.get_or_create("2026-01-11");
        assert!(loaded.remove_manual_item("Test item"));
        repo.save(&loaded);

        let reloaded = repo.get_or_create("2026-01-11");
        assert!(reloaded.find_manual_item("Test item").is_none());
    }
}
