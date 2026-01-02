//! Automerge document storage for persisting documents to disk.

use automerge::AutoCommit;
use std::fs;
use std::io;
use std::path::PathBuf;

use super::DocType;

/// Storage for Automerge documents.
///
/// Handles loading and saving documents to the filesystem.
#[derive(Clone)]
pub struct DocumentStorage {
    data_dir: PathBuf,
}

impl DocumentStorage {
    /// Creates a new storage instance with a custom data directory.
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Returns the data directory path.
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Returns the full path for a document type.
    pub fn path(&self, doc_type: DocType) -> PathBuf {
        self.data_dir.join(doc_type.filename())
    }

    /// Checks if a document exists on disk.
    pub fn exists(&self, doc_type: DocType) -> bool {
        self.path(doc_type).exists()
    }

    /// Loads a document from disk.
    ///
    /// Returns `Ok(None)` if the file doesn't exist.
    /// Returns `Err` for other I/O or parsing errors.
    pub fn load(&self, doc_type: DocType) -> Result<Option<AutoCommit>, StorageError> {
        let path = self.path(doc_type);

        match fs::read(&path) {
            Ok(bytes) => {
                let doc = AutoCommit::load(&bytes)
                    .map_err(|e| StorageError::LoadError(path, e.to_string()))?;
                Ok(Some(doc))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(StorageError::IoError(path, e)),
        }
    }

    /// Loads a document or creates a new one if it doesn't exist.
    pub fn load_or_create(&self, doc_type: DocType) -> Result<AutoCommit, StorageError> {
        match self.load(doc_type)? {
            Some(doc) => Ok(doc),
            None => Ok(AutoCommit::new()),
        }
    }

    /// Saves a document to disk.
    ///
    /// Creates the data directory if it doesn't exist.
    pub fn save(&self, doc_type: DocType, doc: &mut AutoCommit) -> Result<(), StorageError> {
        // Ensure data directory exists
        fs::create_dir_all(&self.data_dir)
            .map_err(|e| StorageError::IoError(self.data_dir.clone(), e))?;

        let path = self.path(doc_type);
        let bytes = doc.save();

        fs::write(&path, bytes).map_err(|e| StorageError::IoError(path, e))?;

        Ok(())
    }
}

/// Errors that can occur during document storage operations.
#[derive(Debug)]
pub enum StorageError {
    /// I/O error reading or writing a file.
    IoError(PathBuf, io::Error),
    /// Error loading/parsing an Automerge document.
    LoadError(PathBuf, String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::IoError(path, e) => {
                write!(f, "I/O error for {}: {}", path.display(), e)
            }
            StorageError::LoadError(path, e) => {
                write!(f, "Failed to load document {}: {}", path.display(), e)
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::IoError(_, e) => Some(e),
            StorageError::LoadError(_, _) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::{transaction::Transactable, ReadDoc, ROOT};
    use tempfile::TempDir;

    fn test_storage() -> (DocumentStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = DocumentStorage::new(temp_dir.path().to_path_buf());
        (storage, temp_dir)
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
    fn test_load_or_create_nonexistent() {
        let (storage, _temp) = test_storage();
        let doc = storage.load_or_create(DocType::Dishes).unwrap();
        assert_eq!(doc.length(ROOT), 0);
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
        let storage = DocumentStorage::new(nested_dir.clone());

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
