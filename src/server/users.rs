//! User store for authentication.
//!
//! Loads user data and API key hashes from `users.automerge` in the data directory.
//! The file is created and managed by the `todufit-admin` CLI.
//!
//! # Document Format
//!
//! ```text
//! {
//!   "user@example.com": {
//!     "group_id": "family1",
//!     "name": "User Name",
//!     "created_at": "2024-01-01T00:00:00Z",
//!     "api_key_hashes": ["sha256hash1", "sha256hash2"]
//!   },
//!   ...
//! }
//! ```

use automerge::{transaction::Transactable, AutoCommit, ObjType, ReadDoc, ROOT};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Loaded user data and key hash mappings from file.
type UserLoadResult = (HashMap<String, User>, HashMap<String, String>);

/// A user loaded from the user store.
#[derive(Debug, Clone)]
pub struct User {
    /// User's email address (unique identifier).
    pub email: String,
    /// Group ID for data access.
    pub group_id: String,
    /// Optional display name.
    pub name: Option<String>,
    /// SHA256 hashes of valid API keys (base64url encoded).
    pub api_key_hashes: HashSet<String>,
}

/// Authenticated user info returned after successful API key validation.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub group_id: String,
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

/// Hash an API key using SHA256 and return base64url encoded result.
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let result = hasher.finalize();
    URL_SAFE_NO_PAD.encode(result)
}

/// In-memory store for user data and API keys.
///
/// Users and API key hashes are loaded from `users.automerge` on initialization.
#[derive(Debug)]
pub struct UserStore {
    /// Path to the users.automerge file.
    path: PathBuf,
    /// Users indexed by email.
    users: HashMap<String, User>,
    /// API key hash -> email lookup for fast validation.
    key_hash_to_email: HashMap<String, String>,
}

impl UserStore {
    /// Loads the user store from the data directory.
    ///
    /// If `users.automerge` doesn't exist, returns an empty store.
    /// If the file is corrupt, logs a warning and returns an empty store.
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("users.automerge");

        match Self::load_from_file(&path) {
            Ok((users, key_hash_to_email)) => {
                tracing::info!(
                    "Loaded {} user(s), {} API key(s)",
                    users.len(),
                    key_hash_to_email.len()
                );
                Self {
                    path,
                    users,
                    key_hash_to_email,
                }
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
                    key_hash_to_email: HashMap::new(),
                }
            }
        }
    }

    /// Load users and key mappings from the Automerge file.
    fn load_from_file(path: &Path) -> Result<UserLoadResult, UserStoreError> {
        let bytes = std::fs::read(path).map_err(UserStoreError::IoError)?;

        let doc =
            AutoCommit::load(&bytes).map_err(|e| UserStoreError::AutomergeError(e.to_string()))?;

        let mut users = HashMap::new();
        let mut key_hash_to_email = HashMap::new();

        for email in doc.keys(ROOT) {
            if let Some(user) = Self::parse_user(&doc, &email) {
                // Build key hash -> email lookup
                for hash in &user.api_key_hashes {
                    key_hash_to_email.insert(hash.clone(), email.clone());
                }
                users.insert(email.to_string(), user);
            }
        }

        Ok((users, key_hash_to_email))
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

        // Load API key hashes
        let mut api_key_hashes = HashSet::new();
        if let Ok(Some((_, hashes_obj))) = doc.get(&user_obj, "api_key_hashes") {
            let len = doc.length(&hashes_obj);
            for i in 0..len {
                if let Ok(Some((val, _))) = doc.get(&hashes_obj, i) {
                    if let Ok(hash) = val.into_string() {
                        api_key_hashes.insert(hash);
                    }
                }
            }
        }

        Some(User {
            email: email.to_string(),
            group_id,
            name,
            api_key_hashes,
        })
    }

    /// Get a user by email.
    pub fn get_user(&self, email: &str) -> Option<&User> {
        self.users.get(email)
    }

    /// Validate an API key and return the authenticated user info.
    ///
    /// Returns `None` if the key is invalid or not associated with any user.
    pub fn validate_api_key(&self, key: &str) -> Option<AuthUser> {
        let hash = hash_api_key(key);

        let email = self.key_hash_to_email.get(&hash)?;
        let user = self.users.get(email)?;

        Some(AuthUser {
            user_id: user.email.clone(),
            group_id: user.group_id.clone(),
        })
    }

    /// Add an API key for a user.
    ///
    /// The key is hashed before storage. Returns an error if the user doesn't exist
    /// or if saving fails.
    pub fn add_api_key(&mut self, email: &str, key: &str) -> Result<(), UserStoreError> {
        let hash = hash_api_key(key);

        // Update in-memory state
        let user = self
            .users
            .get_mut(email)
            .ok_or_else(|| UserStoreError::AutomergeError(format!("User {} not found", email)))?;

        user.api_key_hashes.insert(hash.clone());
        self.key_hash_to_email
            .insert(hash.clone(), email.to_string());

        // Persist to file
        self.save_api_key_hash(email, &hash)?;

        tracing::info!("Added API key for {}", email);
        Ok(())
    }

    /// Save a new API key hash to the Automerge document.
    fn save_api_key_hash(&self, email: &str, hash: &str) -> Result<(), UserStoreError> {
        // Load current document
        let bytes = std::fs::read(&self.path).map_err(UserStoreError::IoError)?;
        let mut doc =
            AutoCommit::load(&bytes).map_err(|e| UserStoreError::AutomergeError(e.to_string()))?;

        // Get user object
        let (_, user_obj) = doc
            .get(ROOT, email)
            .map_err(|e| UserStoreError::AutomergeError(e.to_string()))?
            .ok_or_else(|| UserStoreError::AutomergeError(format!("User {} not found", email)))?;

        // Get or create api_key_hashes array
        let hashes_obj = match doc.get(&user_obj, "api_key_hashes") {
            Ok(Some((_, obj))) => obj,
            _ => doc
                .put_object(&user_obj, "api_key_hashes", ObjType::List)
                .map_err(|e| UserStoreError::AutomergeError(e.to_string()))?,
        };

        // Check if hash already exists
        let len = doc.length(&hashes_obj);
        for i in 0..len {
            if let Ok(Some((val, _))) = doc.get(&hashes_obj, i) {
                if let Ok(existing) = val.into_string() {
                    if existing == hash {
                        // Already exists, nothing to do
                        return Ok(());
                    }
                }
            }
        }

        // Append new hash
        doc.insert(&hashes_obj, len, hash)
            .map_err(|e| UserStoreError::AutomergeError(e.to_string()))?;

        // Save document
        let bytes = doc.save();
        std::fs::write(&self.path, bytes).map_err(UserStoreError::IoError)?;

        Ok(())
    }

    /// Revoke an API key for a user.
    ///
    /// Returns `true` if the key was found and revoked, `false` if not found.
    pub fn revoke_api_key(&mut self, email: &str, key: &str) -> Result<bool, UserStoreError> {
        let hash = hash_api_key(key);

        // Check if key exists
        let user = match self.users.get_mut(email) {
            Some(u) => u,
            None => return Ok(false),
        };

        if !user.api_key_hashes.remove(&hash) {
            return Ok(false);
        }

        self.key_hash_to_email.remove(&hash);

        // Persist removal
        self.remove_api_key_hash(email, &hash)?;

        tracing::info!("Revoked API key for {}", email);
        Ok(true)
    }

    /// Remove an API key hash from the Automerge document.
    fn remove_api_key_hash(&self, email: &str, hash: &str) -> Result<(), UserStoreError> {
        let bytes = std::fs::read(&self.path).map_err(UserStoreError::IoError)?;
        let mut doc =
            AutoCommit::load(&bytes).map_err(|e| UserStoreError::AutomergeError(e.to_string()))?;

        let (_, user_obj) = doc
            .get(ROOT, email)
            .map_err(|e| UserStoreError::AutomergeError(e.to_string()))?
            .ok_or_else(|| UserStoreError::AutomergeError(format!("User {} not found", email)))?;

        if let Ok(Some((_, hashes_obj))) = doc.get(&user_obj, "api_key_hashes") {
            let len = doc.length(&hashes_obj);

            for i in (0..len).rev() {
                if let Ok(Some((val, _))) = doc.get(&hashes_obj, i) {
                    if let Ok(existing) = val.into_string() {
                        if existing == hash {
                            doc.delete(&hashes_obj, i)
                                .map_err(|e| UserStoreError::AutomergeError(e.to_string()))?;
                            break;
                        }
                    }
                }
            }
        }

        let bytes = doc.save();
        std::fs::write(&self.path, bytes).map_err(UserStoreError::IoError)?;

        Ok(())
    }

    /// Reload users from disk.
    ///
    /// Returns the new user count, or an error if loading failed.
    pub fn reload(&mut self) -> Result<usize, UserStoreError> {
        let (users, key_hash_to_email) = Self::load_from_file(&self.path)?;
        let count = users.len();
        self.users = users;
        self.key_hash_to_email = key_hash_to_email;
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

    /// Returns the number of API keys across all users.
    pub fn api_key_count(&self) -> usize {
        self.key_hash_to_email.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_hash_api_key_deterministic() {
        let key = "test-api-key-12345";
        let hash1 = hash_api_key(key);
        let hash2 = hash_api_key(key);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_api_key_different_keys() {
        let hash1 = hash_api_key("key1");
        let hash2 = hash_api_key("key2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_load_empty_when_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let store = UserStore::load(temp_dir.path());

        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.api_key_count(), 0);
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
    fn test_add_and_validate_api_key() {
        let temp_dir = TempDir::new().unwrap();
        create_users_file(temp_dir.path(), &[("alice@example.com", "family1", None)]);

        let mut store = UserStore::load(temp_dir.path());

        // Initially no API keys
        assert_eq!(store.api_key_count(), 0);
        assert!(store.validate_api_key("my-secret-key").is_none());

        // Add API key
        store
            .add_api_key("alice@example.com", "my-secret-key")
            .unwrap();

        // Validate works
        let auth = store.validate_api_key("my-secret-key").unwrap();
        assert_eq!(auth.user_id, "alice@example.com");
        assert_eq!(auth.group_id, "family1");
        assert_eq!(store.api_key_count(), 1);

        // Wrong key doesn't work
        assert!(store.validate_api_key("wrong-key").is_none());
    }

    #[test]
    fn test_api_key_persists_after_reload() {
        let temp_dir = TempDir::new().unwrap();
        create_users_file(temp_dir.path(), &[("alice@example.com", "family1", None)]);

        // Add key
        {
            let mut store = UserStore::load(temp_dir.path());
            store
                .add_api_key("alice@example.com", "persistent-key")
                .unwrap();
        }

        // Load fresh store
        let store = UserStore::load(temp_dir.path());

        // Key should still work
        let auth = store.validate_api_key("persistent-key").unwrap();
        assert_eq!(auth.user_id, "alice@example.com");
    }

    #[test]
    fn test_revoke_api_key() {
        let temp_dir = TempDir::new().unwrap();
        create_users_file(temp_dir.path(), &[("alice@example.com", "family1", None)]);

        let mut store = UserStore::load(temp_dir.path());
        store.add_api_key("alice@example.com", "my-key").unwrap();

        // Key works
        assert!(store.validate_api_key("my-key").is_some());

        // Revoke
        let revoked = store.revoke_api_key("alice@example.com", "my-key").unwrap();
        assert!(revoked);

        // Key no longer works
        assert!(store.validate_api_key("my-key").is_none());

        // Revoking again returns false
        let revoked_again = store.revoke_api_key("alice@example.com", "my-key").unwrap();
        assert!(!revoked_again);
    }

    #[test]
    fn test_multiple_api_keys_per_user() {
        let temp_dir = TempDir::new().unwrap();
        create_users_file(temp_dir.path(), &[("alice@example.com", "family1", None)]);

        let mut store = UserStore::load(temp_dir.path());
        store.add_api_key("alice@example.com", "key1").unwrap();
        store.add_api_key("alice@example.com", "key2").unwrap();

        assert_eq!(store.api_key_count(), 2);
        assert!(store.validate_api_key("key1").is_some());
        assert!(store.validate_api_key("key2").is_some());
    }

    #[test]
    fn test_add_api_key_nonexistent_user() {
        let temp_dir = TempDir::new().unwrap();
        create_users_file(temp_dir.path(), &[("alice@example.com", "family1", None)]);

        let mut store = UserStore::load(temp_dir.path());
        let result = store.add_api_key("nonexistent@example.com", "key");

        assert!(result.is_err());
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
