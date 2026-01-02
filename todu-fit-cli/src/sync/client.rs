//! WebSocket sync client for connecting to the ToduFit sync server.
//!
//! This module wraps the core sync client and adds CLI-specific functionality
//! like storage management and SQLite projection.

use automerge::AutoCommit;

use super::storage::{DocType, DocumentStorage};
use super::{DishProjection, MealLogProjection, MealPlanProjection};
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
    /// Projection error
    ProjectionError(String),
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
            SyncClientError::ProjectionError(e) => write!(f, "Projection error: {}", e),
        }
    }
}

impl std::error::Error for SyncClientError {}

impl From<CoreSyncError> for SyncClientError {
    fn from(e: CoreSyncError) -> Self {
        SyncClientError::SyncError(e)
    }
}

/// Sync client for the CLI that manages storage and projection.
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
    pub fn new(server_url: String, api_key: String) -> Self {
        Self {
            core: CoreSyncClient::new(server_url, api_key),
            storage: DocumentStorage::new(),
        }
    }

    /// Syncs all document types with the server.
    ///
    /// Returns results for each document type.
    pub async fn sync_all(&self) -> Result<Vec<SyncResult>, SyncClientError> {
        let mut results = Vec::new();

        for doc_type in [DocType::Dishes, DocType::MealPlans, DocType::MealLogs] {
            let result = self.sync_document(doc_type).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Syncs a single document type with the server.
    pub async fn sync_document(&self, doc_type: DocType) -> Result<SyncResult, SyncClientError> {
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

    /// Syncs all documents and projects changes to SQLite.
    pub async fn sync_and_project(
        &self,
        pool: &sqlx::SqlitePool,
    ) -> Result<Vec<SyncResult>, SyncClientError> {
        let results = self.sync_all().await?;

        // Project updated documents to SQLite
        for result in &results {
            if result.updated {
                self.project_document(result.doc_type, pool).await?;
            }
        }

        Ok(results)
    }

    /// Projects a document to SQLite.
    async fn project_document(
        &self,
        doc_type: DocType,
        pool: &sqlx::SqlitePool,
    ) -> Result<(), SyncClientError> {
        let doc = self
            .storage
            .load(doc_type)
            .map_err(|e| SyncClientError::StorageError(e.to_string()))?
            .ok_or_else(|| SyncClientError::StorageError("Document not found".to_string()))?;

        match doc_type {
            DocType::Dishes => {
                DishProjection::project_all(&doc, pool)
                    .await
                    .map_err(|e| SyncClientError::ProjectionError(e.to_string()))?;
            }
            DocType::MealPlans => {
                MealPlanProjection::project_all(&doc, pool)
                    .await
                    .map_err(|e| SyncClientError::ProjectionError(e.to_string()))?;
            }
            DocType::MealLogs => {
                MealLogProjection::project_all(&doc, pool)
                    .await
                    .map_err(|e| SyncClientError::ProjectionError(e.to_string()))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ws_url_with_ws() {
        let client = SyncClient::new("ws://localhost:8080".to_string(), "test-key".to_string());
        let url = client.core.build_ws_url(DocType::Dishes);
        assert_eq!(url, "ws://localhost:8080/sync/dishes?key=test-key");
    }

    #[test]
    fn test_build_ws_url_with_http() {
        let client = SyncClient::new("http://localhost:8080".to_string(), "test-key".to_string());
        let url = client.core.build_ws_url(DocType::Dishes);
        assert_eq!(url, "ws://localhost:8080/sync/dishes?key=test-key");
    }

    #[test]
    fn test_build_ws_url_with_https() {
        let client = SyncClient::new(
            "https://sync.example.com".to_string(),
            "test-key".to_string(),
        );
        let url = client.core.build_ws_url(DocType::MealPlans);
        assert_eq!(url, "wss://sync.example.com/sync/mealplans?key=test-key");
    }

    #[test]
    fn test_build_ws_url_bare_host() {
        let client = SyncClient::new("localhost:8080".to_string(), "test-key".to_string());
        let url = client.core.build_ws_url(DocType::MealLogs);
        assert_eq!(url, "ws://localhost:8080/sync/meallogs?key=test-key");
    }
}
