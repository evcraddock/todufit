//! WebSocket sync client wrapper for the CLI.
//!
//! This module wraps the core sync client and adds CLI-specific functionality
//! like storage management and identity-based document discovery.

use automerge::AutoCommit;

use super::storage::{DocType, DocumentStorage};
use crate::config::SyncConfig;

// Re-export core types
pub use todu_fit_core::sync::{SyncClient as CoreSyncClient, SyncError as CoreSyncError};

/// Errors that can occur during sync client operations.
#[derive(Debug)]
pub enum SyncClientError {
    /// Sync is not configured
    NotConfigured,
    /// Core sync error
    SyncError(CoreSyncError),
    /// Storage error
    StorageError(String),
}

impl std::fmt::Display for SyncClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncClientError::NotConfigured => {
                write!(f, "Sync not configured. Add server_url to config.")
            }
            SyncClientError::SyncError(e) => write!(f, "{}", e),
            SyncClientError::StorageError(e) => write!(f, "Storage error: {}", e),
        }
    }
}

impl std::error::Error for SyncClientError {}

impl From<CoreSyncError> for SyncClientError {
    fn from(e: CoreSyncError) -> Self {
        SyncClientError::SyncError(e)
    }
}

/// Sync client for the CLI that manages storage.
///
/// This is a transitional implementation that syncs documents by DocType
/// for backward compatibility. Future versions will use Identity-based
/// document discovery.
#[derive(Debug)]
pub struct SyncClient {
    core: CoreSyncClient,
    storage: DocumentStorage,
}

impl SyncClient {
    /// Creates a new sync client from config.
    ///
    /// Returns an error if sync is not configured.
    pub fn from_config(config: &SyncConfig) -> Result<Self, SyncClientError> {
        let server_url = config
            .server_url
            .clone()
            .ok_or(SyncClientError::NotConfigured)?;

        Ok(Self {
            core: CoreSyncClient::new(server_url),
            storage: DocumentStorage::new(),
        })
    }

    /// Creates a new sync client with explicit server URL.
    #[cfg(test)]
    pub fn new(server_url: String) -> Self {
        Self {
            core: CoreSyncClient::new(server_url),
            storage: DocumentStorage::new(),
        }
    }

    /// Syncs a single document type with the server.
    ///
    /// Note: This is a transitional method. It uses a fixed DocumentId
    /// based on the DocType for backward compatibility.
    pub async fn sync_document(
        &mut self,
        doc_type: DocType,
    ) -> Result<LegacySyncResult, SyncClientError> {
        // Load or create local document
        let mut doc = self
            .storage
            .load(doc_type)
            .map_err(|e| SyncClientError::StorageError(e.to_string()))?
            .unwrap_or_else(AutoCommit::new);

        // Generate a deterministic DocumentId from the doc type
        // This is a transitional approach - production should use Identity
        let doc_id = doc_type_to_doc_id(doc_type);

        // Sync with server using core client
        let result = self.core.sync_document(&doc_id, &mut doc).await?;

        // Save updated document
        self.storage
            .save(doc_type, &mut doc)
            .map_err(|e| SyncClientError::StorageError(e.to_string()))?;

        Ok(LegacySyncResult {
            doc_type,
            updated: result.updated,
            rounds: result.rounds,
        })
    }
}

/// Result of a sync operation for a document type (legacy format).
#[derive(Debug, Clone)]
pub struct LegacySyncResult {
    /// Document type that was synced
    pub doc_type: DocType,
    /// Whether the document was updated
    pub updated: bool,
    /// Number of sync round-trips
    pub rounds: usize,
}

/// Generate a deterministic DocumentId from a DocType.
///
/// This is a transitional function for backward compatibility.
/// Production code should use Identity-based document IDs.
fn doc_type_to_doc_id(doc_type: DocType) -> todu_fit_core::DocumentId {
    // Use fixed UUIDs for each doc type (deterministic for backward compatibility)
    // These are arbitrary but consistent values
    let bytes: [u8; 16] = match doc_type {
        DocType::Dishes => [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ],
        DocType::MealPlans => [
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
            0x1f, 0x20,
        ],
        DocType::MealLogs => [
            0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
            0x2f, 0x30,
        ],
    };

    todu_fit_core::DocumentId::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_client_new() {
        let client = SyncClient::new("ws://localhost:8080".to_string());
        assert_eq!(client.core.server_url(), "ws://localhost:8080");
    }

    #[test]
    fn test_sync_client_from_config() {
        let config = SyncConfig {
            server_url: Some("https://sync.example.com".to_string()),
            auto_sync: false,
        };
        let client = SyncClient::from_config(&config).unwrap();
        assert_eq!(client.core.server_url(), "https://sync.example.com");
    }

    #[test]
    fn test_sync_client_not_configured() {
        let config = SyncConfig {
            server_url: None,
            auto_sync: false,
        };
        let result = SyncClient::from_config(&config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SyncClientError::NotConfigured
        ));
    }

    #[test]
    fn test_doc_type_to_doc_id_deterministic() {
        let id1 = doc_type_to_doc_id(DocType::Dishes);
        let id2 = doc_type_to_doc_id(DocType::Dishes);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_doc_type_to_doc_id_unique() {
        let dishes_id = doc_type_to_doc_id(DocType::Dishes);
        let plans_id = doc_type_to_doc_id(DocType::MealPlans);
        let logs_id = doc_type_to_doc_id(DocType::MealLogs);

        assert_ne!(dishes_id, plans_id);
        assert_ne!(dishes_id, logs_id);
        assert_ne!(plans_id, logs_id);
    }
}
