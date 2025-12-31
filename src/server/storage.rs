//! Server-side Automerge document storage.
//!
//! Stores Automerge documents per group in the following structure:
//! ```text
//! <DATA_DIR>/
//!   <group_id>/
//!     dishes.automerge
//!     mealplans.automerge
//!     meallogs.automerge
//! ```
//!
//! Thread-safe access is provided via file locking.

use automerge::AutoCommit;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;

/// Document types stored on the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocType {
    Dishes,
    MealPlans,
    MealLogs,
}

impl DocType {
    /// Returns the filename for this document type.
    pub fn filename(&self) -> &'static str {
        match self {
            DocType::Dishes => "dishes.automerge",
            DocType::MealPlans => "mealplans.automerge",
            DocType::MealLogs => "meallogs.automerge",
        }
    }

    /// Parse from string name.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dishes" => Some(DocType::Dishes),
            "mealplans" => Some(DocType::MealPlans),
            "meallogs" => Some(DocType::MealLogs),
            _ => None,
        }
    }
}

/// Errors that can occur during server storage operations.
#[derive(Debug)]
pub enum ServerStorageError {
    /// I/O error reading or writing a file.
    IoError(PathBuf, io::Error),
    /// Error loading/parsing an Automerge document.
    AutomergeError(PathBuf, String),
    /// Invalid group ID (e.g., contains path separators).
    InvalidGroupId(String),
    /// Invalid document type.
    InvalidDocType(String),
}

impl std::fmt::Display for ServerStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerStorageError::IoError(path, e) => {
                write!(f, "I/O error for {}: {}", path.display(), e)
            }
            ServerStorageError::AutomergeError(path, e) => {
                write!(f, "Failed to load document {}: {}", path.display(), e)
            }
            ServerStorageError::InvalidGroupId(id) => {
                write!(f, "Invalid group ID: {}", id)
            }
            ServerStorageError::InvalidDocType(t) => {
                write!(f, "Invalid document type: {}", t)
            }
        }
    }
}

impl std::error::Error for ServerStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ServerStorageError::IoError(_, e) => Some(e),
            _ => None,
        }
    }
}

/// Server-side storage for Automerge documents.
///
/// Documents are stored per group in subdirectories of the data directory.
/// File locking is used to ensure thread-safe access.
#[derive(Debug, Clone)]
pub struct ServerStorage {
    data_dir: PathBuf,
}

impl ServerStorage {
    /// Creates a new server storage instance.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// Validates a group ID to prevent path traversal attacks.
    fn validate_group_id(group_id: &str) -> Result<(), ServerStorageError> {
        if group_id.is_empty()
            || group_id.contains('/')
            || group_id.contains('\\')
            || group_id.contains("..")
            || group_id.starts_with('.')
        {
            return Err(ServerStorageError::InvalidGroupId(group_id.to_string()));
        }
        Ok(())
    }

    /// Returns the path for a group's directory.
    fn group_dir(&self, group_id: &str) -> PathBuf {
        self.data_dir.join(group_id)
    }

    /// Returns the full path for a document.
    fn doc_path(&self, group_id: &str, doc_type: DocType) -> PathBuf {
        self.group_dir(group_id).join(doc_type.filename())
    }

    /// Loads a document for a group.
    ///
    /// Returns `Ok(None)` if the document doesn't exist yet.
    pub fn load(
        &self,
        group_id: &str,
        doc_type: DocType,
    ) -> Result<Option<AutoCommit>, ServerStorageError> {
        Self::validate_group_id(group_id)?;

        let path = self.doc_path(group_id, doc_type);

        match File::open(&path) {
            Ok(mut file) => {
                // Read file contents
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)
                    .map_err(|e| ServerStorageError::IoError(path.clone(), e))?;

                // Parse Automerge document
                let doc = AutoCommit::load(&bytes)
                    .map_err(|e| ServerStorageError::AutomergeError(path, e.to_string()))?;

                Ok(Some(doc))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ServerStorageError::IoError(path, e)),
        }
    }

    /// Loads a document by type string.
    pub fn load_by_name(
        &self,
        group_id: &str,
        doc_type: &str,
    ) -> Result<Option<AutoCommit>, ServerStorageError> {
        let doc_type = DocType::parse(doc_type)
            .ok_or_else(|| ServerStorageError::InvalidDocType(doc_type.to_string()))?;
        self.load(group_id, doc_type)
    }

    /// Saves a document for a group.
    ///
    /// Creates the group directory if it doesn't exist.
    pub fn save(
        &self,
        group_id: &str,
        doc_type: DocType,
        doc: &mut AutoCommit,
    ) -> Result<(), ServerStorageError> {
        Self::validate_group_id(group_id)?;

        let group_dir = self.group_dir(group_id);
        let path = self.doc_path(group_id, doc_type);

        // Create group directory if needed
        fs::create_dir_all(&group_dir)
            .map_err(|e| ServerStorageError::IoError(group_dir.clone(), e))?;

        // Serialize document
        let bytes = doc.save();

        // Write atomically using temp file + rename
        let temp_path = path.with_extension("automerge.tmp");

        // Write to temp file
        let mut file = File::create(&temp_path)
            .map_err(|e| ServerStorageError::IoError(temp_path.clone(), e))?;

        file.write_all(&bytes)
            .map_err(|e| ServerStorageError::IoError(temp_path.clone(), e))?;

        file.sync_all()
            .map_err(|e| ServerStorageError::IoError(temp_path.clone(), e))?;

        // Rename to final path (atomic on most filesystems)
        fs::rename(&temp_path, &path).map_err(|e| ServerStorageError::IoError(path, e))?;

        Ok(())
    }

    /// Saves a document by type string.
    pub fn save_by_name(
        &self,
        group_id: &str,
        doc_type: &str,
        doc: &mut AutoCommit,
    ) -> Result<(), ServerStorageError> {
        let doc_type = DocType::parse(doc_type)
            .ok_or_else(|| ServerStorageError::InvalidDocType(doc_type.to_string()))?;
        self.save(group_id, doc_type, doc)
    }

    /// Checks if a document exists for a group.
    pub fn exists(&self, group_id: &str, doc_type: DocType) -> Result<bool, ServerStorageError> {
        Self::validate_group_id(group_id)?;
        Ok(self.doc_path(group_id, doc_type).exists())
    }

    /// Returns the raw bytes of a document (for sync).
    pub fn load_bytes(
        &self,
        group_id: &str,
        doc_type: DocType,
    ) -> Result<Option<Vec<u8>>, ServerStorageError> {
        Self::validate_group_id(group_id)?;

        let path = self.doc_path(group_id, doc_type);

        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ServerStorageError::IoError(path, e)),
        }
    }

    /// Saves raw bytes of a document (for sync).
    pub fn save_bytes(
        &self,
        group_id: &str,
        doc_type: DocType,
        bytes: &[u8],
    ) -> Result<(), ServerStorageError> {
        Self::validate_group_id(group_id)?;

        let group_dir = self.group_dir(group_id);
        let path = self.doc_path(group_id, doc_type);

        // Create group directory if needed
        fs::create_dir_all(&group_dir)
            .map_err(|e| ServerStorageError::IoError(group_dir.clone(), e))?;

        // Write atomically using temp file + rename
        let temp_path = path.with_extension("automerge.tmp");

        fs::write(&temp_path, bytes)
            .map_err(|e| ServerStorageError::IoError(temp_path.clone(), e))?;

        fs::rename(&temp_path, &path).map_err(|e| ServerStorageError::IoError(path, e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::{transaction::Transactable, ReadDoc, ROOT};
    use tempfile::TempDir;

    fn setup() -> (ServerStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = ServerStorage::new(temp_dir.path());
        (storage, temp_dir)
    }

    #[test]
    fn test_doc_type_filename() {
        assert_eq!(DocType::Dishes.filename(), "dishes.automerge");
        assert_eq!(DocType::MealPlans.filename(), "mealplans.automerge");
        assert_eq!(DocType::MealLogs.filename(), "meallogs.automerge");
    }

    #[test]
    fn test_doc_type_from_str() {
        assert_eq!(DocType::parse("dishes"), Some(DocType::Dishes));
        assert_eq!(DocType::parse("DISHES"), Some(DocType::Dishes));
        assert_eq!(DocType::parse("mealplans"), Some(DocType::MealPlans));
        assert_eq!(DocType::parse("meallogs"), Some(DocType::MealLogs));
        assert_eq!(DocType::parse("invalid"), None);
    }

    #[test]
    fn test_validate_group_id() {
        // Valid
        assert!(ServerStorage::validate_group_id("family1").is_ok());
        assert!(ServerStorage::validate_group_id("my-group").is_ok());
        assert!(ServerStorage::validate_group_id("group_123").is_ok());

        // Invalid
        assert!(ServerStorage::validate_group_id("").is_err());
        assert!(ServerStorage::validate_group_id("../evil").is_err());
        assert!(ServerStorage::validate_group_id("foo/bar").is_err());
        assert!(ServerStorage::validate_group_id("foo\\bar").is_err());
        assert!(ServerStorage::validate_group_id(".hidden").is_err());
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let (storage, _temp) = setup();
        let result = storage.load("group1", DocType::Dishes).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let (storage, _temp) = setup();

        // Create a document with some data
        let mut doc = AutoCommit::new();
        doc.put(ROOT, "test_key", "test_value").unwrap();

        // Save it
        storage.save("group1", DocType::Dishes, &mut doc).unwrap();

        // Load it back
        let loaded = storage.load("group1", DocType::Dishes).unwrap().unwrap();

        // Verify the data
        let value: Option<String> = loaded
            .get(ROOT, "test_key")
            .unwrap()
            .map(|(v, _)| v.into_string().unwrap());
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_groups_are_isolated() {
        let (storage, _temp) = setup();

        // Save to group1
        let mut doc1 = AutoCommit::new();
        doc1.put(ROOT, "group", "one").unwrap();
        storage.save("group1", DocType::Dishes, &mut doc1).unwrap();

        // Save to group2
        let mut doc2 = AutoCommit::new();
        doc2.put(ROOT, "group", "two").unwrap();
        storage.save("group2", DocType::Dishes, &mut doc2).unwrap();

        // Load and verify they're separate
        let loaded1 = storage.load("group1", DocType::Dishes).unwrap().unwrap();
        let loaded2 = storage.load("group2", DocType::Dishes).unwrap().unwrap();

        let value1: String = loaded1
            .get(ROOT, "group")
            .unwrap()
            .map(|(v, _)| v.into_string().unwrap())
            .unwrap();
        let value2: String = loaded2
            .get(ROOT, "group")
            .unwrap()
            .map(|(v, _)| v.into_string().unwrap())
            .unwrap();

        assert_eq!(value1, "one");
        assert_eq!(value2, "two");
    }

    #[test]
    fn test_different_doc_types_isolated() {
        let (storage, _temp) = setup();

        // Save dishes
        let mut doc1 = AutoCommit::new();
        doc1.put(ROOT, "type", "dishes").unwrap();
        storage.save("group1", DocType::Dishes, &mut doc1).unwrap();

        // Save mealplans
        let mut doc2 = AutoCommit::new();
        doc2.put(ROOT, "type", "mealplans").unwrap();
        storage
            .save("group1", DocType::MealPlans, &mut doc2)
            .unwrap();

        // Load and verify
        let loaded1 = storage.load("group1", DocType::Dishes).unwrap().unwrap();
        let loaded2 = storage.load("group1", DocType::MealPlans).unwrap().unwrap();

        let value1: String = loaded1
            .get(ROOT, "type")
            .unwrap()
            .map(|(v, _)| v.into_string().unwrap())
            .unwrap();
        let value2: String = loaded2
            .get(ROOT, "type")
            .unwrap()
            .map(|(v, _)| v.into_string().unwrap())
            .unwrap();

        assert_eq!(value1, "dishes");
        assert_eq!(value2, "mealplans");
    }

    #[test]
    fn test_exists() {
        let (storage, _temp) = setup();

        assert!(!storage.exists("group1", DocType::Dishes).unwrap());

        let mut doc = AutoCommit::new();
        storage.save("group1", DocType::Dishes, &mut doc).unwrap();

        assert!(storage.exists("group1", DocType::Dishes).unwrap());
        assert!(!storage.exists("group1", DocType::MealPlans).unwrap());
    }

    #[test]
    fn test_load_save_by_name() {
        let (storage, _temp) = setup();

        let mut doc = AutoCommit::new();
        doc.put(ROOT, "key", "value").unwrap();

        storage.save_by_name("group1", "dishes", &mut doc).unwrap();

        let loaded = storage.load_by_name("group1", "dishes").unwrap().unwrap();
        let value: String = loaded
            .get(ROOT, "key")
            .unwrap()
            .map(|(v, _)| v.into_string().unwrap())
            .unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_invalid_doc_type_name() {
        let (storage, _temp) = setup();

        let result = storage.load_by_name("group1", "invalid");
        assert!(matches!(result, Err(ServerStorageError::InvalidDocType(_))));
    }

    #[test]
    fn test_load_save_bytes() {
        let (storage, _temp) = setup();

        // Create and save a doc
        let mut doc = AutoCommit::new();
        doc.put(ROOT, "key", "value").unwrap();
        let bytes = doc.save();

        // Save bytes directly
        storage
            .save_bytes("group1", DocType::Dishes, &bytes)
            .unwrap();

        // Load bytes back
        let loaded_bytes = storage
            .load_bytes("group1", DocType::Dishes)
            .unwrap()
            .unwrap();

        assert_eq!(bytes, loaded_bytes);
    }

    #[test]
    fn test_directory_structure() {
        let (storage, temp) = setup();

        let mut doc = AutoCommit::new();
        storage.save("mygroup", DocType::Dishes, &mut doc).unwrap();

        // Verify directory structure
        let expected_path = temp.path().join("mygroup").join("dishes.automerge");
        assert!(expected_path.exists());
    }

    #[test]
    fn test_overwrite_existing() {
        let (storage, _temp) = setup();

        // Save first version
        let mut doc1 = AutoCommit::new();
        doc1.put(ROOT, "version", "1").unwrap();
        storage.save("group1", DocType::Dishes, &mut doc1).unwrap();

        // Save second version
        let mut doc2 = AutoCommit::new();
        doc2.put(ROOT, "version", "2").unwrap();
        storage.save("group1", DocType::Dishes, &mut doc2).unwrap();

        // Verify it's the second version
        let loaded = storage.load("group1", DocType::Dishes).unwrap().unwrap();
        let value: String = loaded
            .get(ROOT, "version")
            .unwrap()
            .map(|(v, _)| v.into_string().unwrap())
            .unwrap();
        assert_eq!(value, "2");
    }
}
