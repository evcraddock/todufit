//! Automerge document storage for the CLI.
//!
//! Re-exports from todu-fit-core and provides CLI-specific defaults.

use std::path::PathBuf;

use crate::config::Config;

// Re-export core types
pub use todu_fit_core::automerge::DocumentStorage as CoreDocumentStorage;
pub use todu_fit_core::automerge::{DocType, StorageError};

/// CLI wrapper for DocumentStorage that provides a default constructor.
#[derive(Clone, Debug)]
pub struct DocumentStorage(CoreDocumentStorage);

impl DocumentStorage {
    /// Creates a new storage instance with the default data directory.
    pub fn new() -> Self {
        Self(CoreDocumentStorage::new(Config::default_data_dir()))
    }

    /// Creates a new storage instance with a custom data directory.
    pub fn with_data_dir(data_dir: PathBuf) -> Self {
        Self(CoreDocumentStorage::new(data_dir))
    }

    /// Returns the full path for a document type.
    pub fn path(&self, doc_type: DocType) -> PathBuf {
        self.0.path(doc_type)
    }

    /// Checks if a document exists on disk.
    pub fn exists(&self, doc_type: DocType) -> bool {
        self.0.exists(doc_type)
    }

    /// Loads a document from disk.
    pub fn load(&self, doc_type: DocType) -> Result<Option<automerge::AutoCommit>, StorageError> {
        self.0.load(doc_type)
    }

    /// Loads a document or creates a new one if it doesn't exist.
    pub fn load_or_create(&self, doc_type: DocType) -> Result<automerge::AutoCommit, StorageError> {
        self.0.load_or_create(doc_type)
    }

    /// Saves a document to disk.
    pub fn save(
        &self,
        doc_type: DocType,
        doc: &mut automerge::AutoCommit,
    ) -> Result<(), StorageError> {
        self.0.save(doc_type, doc)
    }
}

impl Default for DocumentStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::{transaction::Transactable, AutoCommit, ReadDoc, ROOT};
    use tempfile::TempDir;

    fn test_storage() -> (DocumentStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::with_data_dir(temp_dir.path().to_path_buf());
        (storage, temp_dir)
    }

    #[test]
    fn test_doc_type_filename() {
        assert_eq!(DocType::Dishes.filename(), "dishes.automerge");
        assert_eq!(DocType::MealPlans.filename(), "mealplans.automerge");
        assert_eq!(DocType::MealLogs.filename(), "meallogs.automerge");
    }

    #[test]
    fn test_storage_path() {
        let (storage, _temp) = test_storage();
        let path = storage.path(DocType::Dishes);
        assert!(path.ends_with("dishes.automerge"));
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let (storage, _temp) = test_storage();
        let result = storage.load(DocType::Dishes).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_exists_false_initially() {
        let (storage, _temp) = test_storage();
        assert!(!storage.exists(DocType::Dishes));
        assert!(!storage.exists(DocType::MealPlans));
        assert!(!storage.exists(DocType::MealLogs));
    }

    #[test]
    fn test_save_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("nested").join("data");
        let storage = DocumentStorage::with_data_dir(nested_dir.clone());

        let mut doc = AutoCommit::new();
        storage.save(DocType::Dishes, &mut doc).unwrap();

        assert!(nested_dir.exists());
        assert!(storage.exists(DocType::Dishes));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let (storage, _temp) = test_storage();

        // Create a document with some data
        let mut doc = AutoCommit::new();
        doc.put(ROOT, "test_key", "test_value").unwrap();

        // Save it
        storage.save(DocType::Dishes, &mut doc).unwrap();

        // Load it back
        let loaded = storage.load(DocType::Dishes).unwrap().unwrap();

        // Verify the data
        let value: Option<String> = loaded
            .get(ROOT, "test_key")
            .unwrap()
            .map(|(v, _)| v.into_string().unwrap());
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_save_and_load_all_doc_types() {
        let (storage, _temp) = test_storage();

        for doc_type in [DocType::Dishes, DocType::MealPlans, DocType::MealLogs] {
            let mut doc = AutoCommit::new();
            doc.put(ROOT, "type", format!("{:?}", doc_type)).unwrap();

            storage.save(doc_type, &mut doc).unwrap();
            assert!(storage.exists(doc_type));

            let loaded = storage.load(doc_type).unwrap().unwrap();
            let value: String = loaded
                .get(ROOT, "type")
                .unwrap()
                .map(|(v, _)| v.into_string().unwrap())
                .unwrap();
            assert_eq!(value, format!("{:?}", doc_type));
        }
    }

    #[test]
    fn test_exists_after_save() {
        let (storage, _temp) = test_storage();

        assert!(!storage.exists(DocType::Dishes));

        let mut doc = AutoCommit::new();
        storage.save(DocType::Dishes, &mut doc).unwrap();

        assert!(storage.exists(DocType::Dishes));
    }

    #[test]
    fn test_overwrite_existing_document() {
        let (storage, _temp) = test_storage();

        // Save first version
        let mut doc1 = AutoCommit::new();
        doc1.put(ROOT, "version", "1").unwrap();
        storage.save(DocType::Dishes, &mut doc1).unwrap();

        // Save second version
        let mut doc2 = AutoCommit::new();
        doc2.put(ROOT, "version", "2").unwrap();
        storage.save(DocType::Dishes, &mut doc2).unwrap();

        // Load and verify it's the second version
        let loaded = storage.load(DocType::Dishes).unwrap().unwrap();
        let value: String = loaded
            .get(ROOT, "version")
            .unwrap()
            .map(|(v, _)| v.into_string().unwrap())
            .unwrap();
        assert_eq!(value, "2");
    }
}
