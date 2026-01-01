//! User store for authentication.
//!
//! Loads user data from `users.automerge` in the data directory.
//! The file is created and managed by the `todufit-admin` CLI.
//!
//! # Document Format
//!
//! ```text
//! {
//!   "user@example.com": {
//!     "group_id": "family1",
//!     "name": "User Name",
//!     "created_at": "2024-01-01T00:00:00Z"
//!   },
//!   ...
//! }
//! ```

use automerge::{AutoCommit, ReadDoc, ROOT};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A user loaded from the user store.
#[derive(Debug, Clone)]
pub struct User {
    /// User's email address (unique identifier).
    pub email: String,
    /// Group ID for data access.
    pub group_id: String,
    /// Optional display name.
    pub name: Option<String>,
}

/// Errors that can occur when loading the user store.
#[derive(Debug)]
pub enum UserStoreError {
    /// I/O error reading the file.
    IoError(std::io::Error),
    /// Error parsing the Automerge document.
    AutomergeError(String),
}

impl std::fmt::Display for UserStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserStoreError::IoError(e) => write!(f, "I/O error: {}", e),
            UserStoreError::AutomergeError(e) => write!(f, "Automerge error: {}", e),
        }
    }
}

impl std::error::Error for UserStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UserStoreError::IoError(e) => Some(e),
            UserStoreError::AutomergeError(_) => None,
        }
    }
}

/// In-memory store for user data.
///
/// Users are loaded from `users.automerge` on initialization.
#[derive(Debug, Clone)]
pub struct UserStore {
    /// Path to the users.automerge file.
    path: PathBuf,
    /// Users indexed by email.
    users: HashMap<String, User>,
}

impl UserStore {
    /// Loads the user store from the data directory.
    ///
    /// If `users.automerge` doesn't exist, returns an empty store.
    /// If the file is corrupt, logs a warning and returns an empty store.
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("users.automerge");

        match Self::load_users(&path) {
            Ok(users) => {
                tracing::info!("Loaded {} user(s)", users.len());
                Self { path, users }
            }
            Err(e) => {
                if matches!(&e, UserStoreError::IoError(io_err) if io_err.kind() == std::io::ErrorKind::NotFound)
                {
                    tracing::info!("No users.automerge found, starting with 0 users");
                } else {
                    tracing::warn!("Failed to load users.automerge: {}", e);
                }
                Self {
                    path,
                    users: HashMap::new(),
                }
            }
        }
    }

    /// Load users from the Automerge file.
    fn load_users(path: &Path) -> Result<HashMap<String, User>, UserStoreError> {
        let bytes = std::fs::read(path).map_err(UserStoreError::IoError)?;

        let doc =
            AutoCommit::load(&bytes).map_err(|e| UserStoreError::AutomergeError(e.to_string()))?;

        let mut users = HashMap::new();

        for email in doc.keys(ROOT) {
            if let Some(user) = Self::parse_user(&doc, &email) {
                users.insert(email.to_string(), user);
            }
        }

        Ok(users)
    }

    /// Parse a single user from the document.
    fn parse_user(doc: &AutoCommit, email: &str) -> Option<User> {
        let (_, user_obj) = doc.get(ROOT, email).ok()??;

        let group_id = doc
            .get(&user_obj, "group_id")
            .ok()?
            .and_then(|(v, _)| v.into_string().ok())?;

        let name = doc
            .get(&user_obj, "name")
            .ok()
            .flatten()
            .and_then(|(v, _)| v.into_string().ok());

        Some(User {
            email: email.to_string(),
            group_id,
            name,
        })
    }

    /// Get a user by email.
    pub fn get_user(&self, email: &str) -> Option<&User> {
        self.users.get(email)
    }

    /// Reload users from disk.
    ///
    /// Returns the new user count, or an error if loading failed.
    pub fn reload(&mut self) -> Result<usize, UserStoreError> {
        let users = Self::load_users(&self.path)?;
        let count = users.len();
        self.users = users;
        tracing::info!("Reloaded {} user(s)", count);
        Ok(count)
    }

    /// Returns the number of loaded users.
    pub fn len(&self) -> usize {
        self.users.len()
    }

    /// Returns true if no users are loaded.
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::{transaction::Transactable, ObjType};
    use tempfile::TempDir;

    fn create_users_file(dir: &Path, users: &[(&str, &str, Option<&str>)]) {
        let mut doc = AutoCommit::new();

        for (email, group_id, name) in users {
            let user_obj = doc.put_object(ROOT, *email, ObjType::Map).unwrap();
            doc.put(&user_obj, "group_id", *group_id).unwrap();
            doc.put(&user_obj, "created_at", "2024-01-01T00:00:00Z")
                .unwrap();
            if let Some(n) = name {
                doc.put(&user_obj, "name", *n).unwrap();
            }
        }

        let bytes = doc.save();
        std::fs::write(dir.join("users.automerge"), bytes).unwrap();
    }

    #[test]
    fn test_load_empty_when_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let store = UserStore::load(temp_dir.path());

        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_load_users() {
        let temp_dir = TempDir::new().unwrap();
        create_users_file(
            temp_dir.path(),
            &[
                ("alice@example.com", "family1", Some("Alice")),
                ("bob@example.com", "family1", None),
            ],
        );

        let store = UserStore::load(temp_dir.path());

        assert_eq!(store.len(), 2);

        let alice = store.get_user("alice@example.com").unwrap();
        assert_eq!(alice.email, "alice@example.com");
        assert_eq!(alice.group_id, "family1");
        assert_eq!(alice.name.as_deref(), Some("Alice"));

        let bob = store.get_user("bob@example.com").unwrap();
        assert_eq!(bob.email, "bob@example.com");
        assert_eq!(bob.group_id, "family1");
        assert!(bob.name.is_none());
    }

    #[test]
    fn test_get_nonexistent_user() {
        let temp_dir = TempDir::new().unwrap();
        create_users_file(temp_dir.path(), &[("alice@example.com", "family1", None)]);

        let store = UserStore::load(temp_dir.path());

        assert!(store.get_user("nonexistent@example.com").is_none());
    }

    #[test]
    fn test_reload() {
        let temp_dir = TempDir::new().unwrap();
        create_users_file(temp_dir.path(), &[("alice@example.com", "family1", None)]);

        let mut store = UserStore::load(temp_dir.path());
        assert_eq!(store.len(), 1);

        // Add another user
        create_users_file(
            temp_dir.path(),
            &[
                ("alice@example.com", "family1", None),
                ("bob@example.com", "family2", Some("Bob")),
            ],
        );

        let count = store.reload().unwrap();
        assert_eq!(count, 2);
        assert_eq!(store.len(), 2);

        let bob = store.get_user("bob@example.com").unwrap();
        assert_eq!(bob.group_id, "family2");
        assert_eq!(bob.name.as_deref(), Some("Bob"));
    }

    #[test]
    fn test_corrupt_file_returns_empty() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("users.automerge"),
            b"not valid automerge",
        )
        .unwrap();

        let store = UserStore::load(temp_dir.path());

        // Should gracefully handle corrupt file
        assert!(store.is_empty());
    }

    #[test]
    fn test_user_without_group_id_skipped() {
        let temp_dir = TempDir::new().unwrap();

        // Create a malformed user entry (no group_id)
        let mut doc = AutoCommit::new();
        let user_obj = doc
            .put_object(ROOT, "bad@example.com", ObjType::Map)
            .unwrap();
        doc.put(&user_obj, "name", "Bad User").unwrap();
        // No group_id!

        // Add a valid user
        let valid_obj = doc
            .put_object(ROOT, "good@example.com", ObjType::Map)
            .unwrap();
        doc.put(&valid_obj, "group_id", "family1").unwrap();

        let bytes = doc.save();
        std::fs::write(temp_dir.path().join("users.automerge"), bytes).unwrap();

        let store = UserStore::load(temp_dir.path());

        // Only the valid user should be loaded
        assert_eq!(store.len(), 1);
        assert!(store.get_user("bad@example.com").is_none());
        assert!(store.get_user("good@example.com").is_some());
    }
}
