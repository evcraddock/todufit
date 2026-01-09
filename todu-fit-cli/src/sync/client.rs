//! WebSocket sync client for connecting to the ToduFit sync server.
//!
//! This module wraps the core sync client and adds CLI-specific functionality
//! like storage management.

use std::collections::HashMap;

use automerge::AutoCommit;

use super::storage::{DocType, DocumentStorage};
use crate::config::SyncConfig;

// Re-export core types
pub use todu_fit_core::sync::{
    SyncClient as CoreSyncClient, SyncError as CoreSyncError, SyncResult,
};

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
            SyncClientError::NotConfigured => write!(
                f,
                "Sync not configured. Add server_url and api_key to config."
            ),
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
        let api_key = config
            .api_key
            .clone()
            .ok_or(SyncClientError::NotConfigured)?;

        Ok(Self {
            core: CoreSyncClient::new(server_url, api_key),
            storage: DocumentStorage::new(),
        })
    }

    /// Creates a new sync client with explicit parameters.
    #[cfg(test)]
    pub fn new(server_url: String, api_key: String) -> Self {
        Self {
            core: CoreSyncClient::new(server_url, api_key),
            storage: DocumentStorage::new(),
        }
    }

    /// Syncs all document types with the server over a single connection.
    ///
    /// Returns results for each document type.
    pub async fn sync_all(&mut self) -> Result<Vec<SyncResult>, SyncClientError> {
        // Load all documents
        let mut docs: HashMap<DocType, AutoCommit> = HashMap::new();

        for doc_type in [DocType::Dishes, DocType::MealPlans, DocType::MealLogs] {
            let doc = self
                .storage
                .load(doc_type)
                .map_err(|e| SyncClientError::StorageError(e.to_string()))?
                .unwrap_or_else(AutoCommit::new);
            docs.insert(doc_type, doc);
        }

        // Sync all documents over single connection
        let results = self.core.sync_all(&mut docs).await?;

        // Save all documents
        for (doc_type, mut doc) in docs {
            self.storage
                .save(doc_type, &mut doc)
                .map_err(|e| SyncClientError::StorageError(e.to_string()))?;
        }

        Ok(results)
    }

    /// Syncs a single document type with the server.
    ///
    /// Note: This still opens a connection for each call. For efficiency,
    /// prefer using `sync_all` to sync all documents over a single connection.
    pub async fn sync_document(
        &mut self,
        doc_type: DocType,
    ) -> Result<SyncResult, SyncClientError> {
        // Load or create local document
        let mut doc = self
            .storage
            .load(doc_type)
            .map_err(|e| SyncClientError::StorageError(e.to_string()))?
            .unwrap_or_else(AutoCommit::new);

        // Sync with server using core client
        let result = self.core.sync_document(doc_type, &mut doc).await?;

        // Save updated document
        self.storage
            .save(doc_type, &mut doc)
            .map_err(|e| SyncClientError::StorageError(e.to_string()))?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_client_new() {
        let client = SyncClient::new("ws://localhost:8080".to_string(), "test-key".to_string());
        assert_eq!(client.core.server_url(), "ws://localhost:8080");
        assert_eq!(client.core.api_key(), "test-key");
    }

    #[test]
    fn test_sync_client_from_config() {
        let config = SyncConfig {
            server_url: Some("https://sync.example.com".to_string()),
            api_key: Some("my-key".to_string()),
            auto_sync: false,
        };
        let client = SyncClient::from_config(&config).unwrap();
        assert_eq!(client.core.server_url(), "https://sync.example.com");
        assert_eq!(client.core.api_key(), "my-key");
    }

    #[test]
    fn test_sync_client_not_configured() {
        let config = SyncConfig {
            server_url: None,
            api_key: None,
            auto_sync: false,
        };
        let result = SyncClient::from_config(&config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SyncClientError::NotConfigured
        ));
    }
}
