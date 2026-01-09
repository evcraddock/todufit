//! Multi-document storage layer for Automerge documents.
//!
//! Stores documents by their DocumentId rather than by type. Each document
//! is stored as `<doc_id>.automerge` in the data directory.
//!
//! Storage layout:
//! ```text
//! ~/.local/share/fit/
//! ├── root_doc_id                    # text file with identity doc ID
//! ├── <identity-id>.automerge
//! ├── <meallogs-id>.automerge
//! ├── <group-id>.automerge
//! ├── <dishes-id>.automerge
//! └── <mealplans-id>.automerge
//! ```

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::document_id::DocumentId;

/// File extension for Automerge documents.
const DOC_EXTENSION: &str = "automerge";

/// Filename for the root document ID file.
const ROOT_DOC_ID_FILE: &str = "root_doc_id";

/// Multi-document storage for Automerge documents.
///
/// Stores and retrieves documents by their DocumentId.
#[derive(Clone, Debug)]
pub struct MultiDocStorage {
    data_dir: PathBuf,
}

impl MultiDocStorage {
    /// Creates a new storage instance with a custom data directory.
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Returns the data directory path.
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Returns the full path for a document.
    pub fn doc_path(&self, doc_id: &DocumentId) -> PathBuf {
        self.data_dir
            .join(format!("{}.{}", doc_id.to_bs58check(), DOC_EXTENSION))
    }

    /// Checks if a document exists on disk.
    pub fn exists(&self, doc_id: &DocumentId) -> bool {
        self.doc_path(doc_id).exists()
    }

    /// Loads a document from disk.
    ///
    /// Returns `Ok(None)` if the file doesn't exist.
    /// Returns `Err` for other I/O or parsing errors.
    pub fn load(&self, doc_id: &DocumentId) -> Result<Option<Vec<u8>>, MultiStorageError> {
        let path = self.doc_path(doc_id);

        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(MultiStorageError::IoError(path, e)),
        }
    }

    /// Saves a document to disk.
    ///
    /// Creates the data directory if it doesn't exist.
    pub fn save(&self, doc_id: &DocumentId, bytes: &[u8]) -> Result<(), MultiStorageError> {
        // Ensure data directory exists
        fs::create_dir_all(&self.data_dir)
            .map_err(|e| MultiStorageError::IoError(self.data_dir.clone(), e))?;

        let path = self.doc_path(doc_id);
        fs::write(&path, bytes).map_err(|e| MultiStorageError::IoError(path, e))?;

        Ok(())
    }

    /// Deletes a document from disk.
    ///
    /// Returns `Ok(true)` if the file was deleted, `Ok(false)` if it didn't exist.
    pub fn delete(&self, doc_id: &DocumentId) -> Result<bool, MultiStorageError> {
        let path = self.doc_path(doc_id);

        match fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(MultiStorageError::IoError(path, e)),
        }
    }

    /// Lists all document IDs stored in the data directory.
    pub fn list(&self) -> Result<Vec<DocumentId>, MultiStorageError> {
        let entries = match fs::read_dir(&self.data_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(MultiStorageError::IoError(self.data_dir.clone(), e)),
        };

        let mut doc_ids = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| MultiStorageError::IoError(self.data_dir.clone(), e))?;
            let path = entry.path();

            // Skip non-files
            if !path.is_file() {
                continue;
            }

            // Check extension
            if path.extension().and_then(|s| s.to_str()) != Some(DOC_EXTENSION) {
                continue;
            }

            // Extract document ID from filename
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(doc_id) = DocumentId::from_bs58check(stem) {
                    doc_ids.push(doc_id);
                }
            }
        }

        Ok(doc_ids)
    }

    // ==================== Root Document ID ====================

    /// Returns the path to the root document ID file.
    fn root_doc_id_path(&self) -> PathBuf {
        self.data_dir.join(ROOT_DOC_ID_FILE)
    }

    /// Saves the root document ID.
    ///
    /// The root document ID identifies the user's identity document.
    pub fn save_root_id(&self, doc_id: &DocumentId) -> Result<(), MultiStorageError> {
        // Ensure data directory exists
        fs::create_dir_all(&self.data_dir)
            .map_err(|e| MultiStorageError::IoError(self.data_dir.clone(), e))?;

        let path = self.root_doc_id_path();
        let content = doc_id.to_bs58check();

        fs::write(&path, content).map_err(|e| MultiStorageError::IoError(path, e))?;

        Ok(())
    }

    /// Loads the root document ID.
    ///
    /// Returns `Ok(None)` if no root ID has been set.
    pub fn load_root_id(&self) -> Result<Option<DocumentId>, MultiStorageError> {
        let path = self.root_doc_id_path();

        match fs::read_to_string(&path) {
            Ok(content) => {
                let content = content.trim();
                let doc_id = DocumentId::from_bs58check(content).map_err(|e| {
                    MultiStorageError::InvalidDocId(content.to_string(), e.to_string())
                })?;
                Ok(Some(doc_id))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(MultiStorageError::IoError(path, e)),
        }
    }

    /// Checks if a root document ID exists.
    pub fn has_root_id(&self) -> bool {
        self.root_doc_id_path().exists()
    }
}

/// Errors that can occur during multi-document storage operations.
#[derive(Debug)]
pub enum MultiStorageError {
    /// I/O error reading or writing a file.
    IoError(PathBuf, io::Error),
    /// Invalid document ID format.
    InvalidDocId(String, String),
}

impl std::fmt::Display for MultiStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiStorageError::IoError(path, e) => {
                write!(f, "I/O error for {}: {}", path.display(), e)
            }
            MultiStorageError::InvalidDocId(id, e) => {
                write!(f, "Invalid document ID '{}': {}", id, e)
            }
        }
    }
}

impl std::error::Error for MultiStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MultiStorageError::IoError(_, e) => Some(e),
            MultiStorageError::InvalidDocId(_, _) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_storage() -> (MultiDocStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = MultiDocStorage::new(temp_dir.path().to_path_buf());
        (storage, temp_dir)
    }

    #[test]
    fn test_doc_path() {
        let (storage, _temp) = test_storage();
        let doc_id = DocumentId::new();
        let path = storage.doc_path(&doc_id);

        assert!(path.to_string_lossy().contains(&doc_id.to_bs58check()));
        assert!(path.to_string_lossy().ends_with(".automerge"));
    }

    #[test]
    fn test_exists_false_initially() {
        let (storage, _temp) = test_storage();
        let doc_id = DocumentId::new();
        assert!(!storage.exists(&doc_id));
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let (storage, _temp) = test_storage();
        let doc_id = DocumentId::new();
        let result = storage.load(&doc_id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("nested").join("data");
        let storage = MultiDocStorage::new(nested_dir.clone());

        let doc_id = DocumentId::new();
        storage.save(&doc_id, b"test data").unwrap();

        assert!(nested_dir.exists());
        assert!(storage.exists(&doc_id));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let (storage, _temp) = test_storage();
        let doc_id = DocumentId::new();
        let data = b"test document content";

        storage.save(&doc_id, data).unwrap();
        let loaded = storage.load(&doc_id).unwrap().unwrap();

        assert_eq!(loaded, data);
    }

    #[test]
    fn test_exists_after_save() {
        let (storage, _temp) = test_storage();
        let doc_id = DocumentId::new();

        assert!(!storage.exists(&doc_id));
        storage.save(&doc_id, b"test").unwrap();
        assert!(storage.exists(&doc_id));
    }

    #[test]
    fn test_delete_existing() {
        let (storage, _temp) = test_storage();
        let doc_id = DocumentId::new();

        storage.save(&doc_id, b"test").unwrap();
        assert!(storage.exists(&doc_id));

        let deleted = storage.delete(&doc_id).unwrap();
        assert!(deleted);
        assert!(!storage.exists(&doc_id));
    }

    #[test]
    fn test_delete_nonexistent() {
        let (storage, _temp) = test_storage();
        let doc_id = DocumentId::new();

        let deleted = storage.delete(&doc_id).unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_list_empty() {
        let (storage, _temp) = test_storage();
        let ids = storage.list().unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_list_multiple_documents() {
        let (storage, _temp) = test_storage();

        let id1 = DocumentId::new();
        let id2 = DocumentId::new();
        let id3 = DocumentId::new();

        storage.save(&id1, b"doc1").unwrap();
        storage.save(&id2, b"doc2").unwrap();
        storage.save(&id3, b"doc3").unwrap();

        let ids = storage.list().unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[test]
    fn test_list_ignores_non_automerge_files() {
        let (storage, temp_dir) = test_storage();

        let doc_id = DocumentId::new();
        storage.save(&doc_id, b"doc").unwrap();

        // Create non-automerge files
        std::fs::write(temp_dir.path().join("test.txt"), "not a doc").unwrap();
        std::fs::write(temp_dir.path().join("root_doc_id"), "some id").unwrap();

        let ids = storage.list().unwrap();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&doc_id));
    }

    // ==================== Root Document ID Tests ====================

    #[test]
    fn test_has_root_id_false_initially() {
        let (storage, _temp) = test_storage();
        assert!(!storage.has_root_id());
    }

    #[test]
    fn test_load_root_id_none_initially() {
        let (storage, _temp) = test_storage();
        let root_id = storage.load_root_id().unwrap();
        assert!(root_id.is_none());
    }

    #[test]
    fn test_save_and_load_root_id() {
        let (storage, _temp) = test_storage();
        let doc_id = DocumentId::new();

        storage.save_root_id(&doc_id).unwrap();
        assert!(storage.has_root_id());

        let loaded = storage.load_root_id().unwrap().unwrap();
        assert_eq!(loaded, doc_id);
    }

    #[test]
    fn test_root_id_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("nested").join("data");
        let storage = MultiDocStorage::new(nested_dir.clone());

        let doc_id = DocumentId::new();
        storage.save_root_id(&doc_id).unwrap();

        assert!(nested_dir.exists());
        assert!(storage.has_root_id());
    }

    #[test]
    fn test_root_id_overwrites() {
        let (storage, _temp) = test_storage();

        let id1 = DocumentId::new();
        let id2 = DocumentId::new();

        storage.save_root_id(&id1).unwrap();
        storage.save_root_id(&id2).unwrap();

        let loaded = storage.load_root_id().unwrap().unwrap();
        assert_eq!(loaded, id2);
    }
}
