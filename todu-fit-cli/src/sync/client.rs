//! WebSocket sync client for connecting to the ToduFit sync server.
//!
//! Uses the Automerge sync protocol over WebSocket to synchronize
//! documents with the server.

use automerge::sync::{Message as SyncMessage, State as SyncState, SyncDoc};
use automerge::AutoCommit;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::storage::{DocType, DocumentStorage};
use super::{DishProjection, MealLogProjection, MealPlanProjection};
use crate::config::SyncConfig;

/// Errors that can occur during sync client operations.
#[derive(Debug)]
pub enum SyncClientError {
    /// Sync is not configured
    NotConfigured,
    /// Failed to connect to server
    ConnectionError(String),
    /// WebSocket error
    WebSocketError(String),
    /// Sync protocol error
    SyncError(String),
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
            SyncClientError::ConnectionError(e) => write!(f, "Connection error: {}", e),
            SyncClientError::WebSocketError(e) => write!(f, "WebSocket error: {}", e),
            SyncClientError::SyncError(e) => write!(f, "Sync error: {}", e),
            SyncClientError::StorageError(e) => write!(f, "Storage error: {}", e),
            SyncClientError::ProjectionError(e) => write!(f, "Projection error: {}", e),
        }
    }
}

impl std::error::Error for SyncClientError {}

/// Result of a sync operation for a single document type.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Document type that was synced
    pub doc_type: DocType,
    /// Whether the document was updated
    pub updated: bool,
    /// Number of sync round-trips
    pub rounds: usize,
}

/// Sync client for connecting to the ToduFit sync server.
pub struct SyncClient {
    server_url: String,
    api_key: String,
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
            server_url,
            api_key,
            storage: DocumentStorage::new(),
        })
    }

    /// Creates a new sync client with explicit parameters.
    pub fn new(server_url: String, api_key: String) -> Self {
        Self {
            server_url,
            api_key,
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
        // Build WebSocket URL
        let ws_url = self.build_ws_url(doc_type);

        // Connect to server
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| SyncClientError::ConnectionError(e.to_string()))?;

        let (mut sender, mut receiver) = ws_stream.split();

        // Load or create local document
        let mut doc = self
            .storage
            .load(doc_type)
            .map_err(|e| SyncClientError::StorageError(e.to_string()))?
            .unwrap_or_else(AutoCommit::new);

        let initial_heads = doc.get_heads().to_vec();

        // Initialize sync state
        let mut sync_state = SyncState::new();
        let mut rounds = 0;

        // Generate initial sync message
        if let Some(msg) = doc.sync().generate_sync_message(&mut sync_state) {
            sender
                .send(Message::Binary(msg.encode().into()))
                .await
                .map_err(|e| SyncClientError::WebSocketError(e.to_string()))?;
            rounds += 1;
        }

        // Sync loop
        loop {
            match receiver.next().await {
                Some(Ok(Message::Binary(data))) => {
                    // Decode and apply server's sync message
                    let msg = SyncMessage::decode(&data)
                        .map_err(|e| SyncClientError::SyncError(e.to_string()))?;

                    doc.sync()
                        .receive_sync_message(&mut sync_state, msg)
                        .map_err(|e| SyncClientError::SyncError(e.to_string()))?;

                    // Generate response if needed
                    if let Some(response) = doc.sync().generate_sync_message(&mut sync_state) {
                        sender
                            .send(Message::Binary(response.encode().into()))
                            .await
                            .map_err(|e| SyncClientError::WebSocketError(e.to_string()))?;
                        rounds += 1;
                    } else {
                        // No more messages needed, sync complete
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    // Server closed connection
                    break;
                }
                Some(Ok(Message::Ping(data))) => {
                    sender
                        .send(Message::Pong(data))
                        .await
                        .map_err(|e| SyncClientError::WebSocketError(e.to_string()))?;
                }
                Some(Ok(_)) => {
                    // Ignore other message types
                }
                Some(Err(e)) => {
                    return Err(SyncClientError::WebSocketError(e.to_string()));
                }
                None => {
                    // Connection closed
                    break;
                }
            }
        }

        // Close WebSocket gracefully
        let _ = sender.send(Message::Close(None)).await;

        // Check if document was updated
        let final_heads = doc.get_heads().to_vec();
        let updated = initial_heads != final_heads;

        // Save updated document
        self.storage
            .save(doc_type, &mut doc)
            .map_err(|e| SyncClientError::StorageError(e.to_string()))?;

        Ok(SyncResult {
            doc_type,
            updated,
            rounds,
        })
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

    /// Builds the WebSocket URL for a document type.
    fn build_ws_url(&self, doc_type: DocType) -> String {
        let doc_type_str = match doc_type {
            DocType::Dishes => "dishes",
            DocType::MealPlans => "mealplans",
            DocType::MealLogs => "meallogs",
        };

        // Convert http(s) to ws(s) if needed
        let base_url = if self.server_url.starts_with("http://") {
            self.server_url.replace("http://", "ws://")
        } else if self.server_url.starts_with("https://") {
            self.server_url.replace("https://", "wss://")
        } else if !self.server_url.starts_with("ws://") && !self.server_url.starts_with("wss://") {
            format!("ws://{}", self.server_url)
        } else {
            self.server_url.clone()
        };

        format!("{}/sync/{}?key={}", base_url, doc_type_str, self.api_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ws_url_with_ws() {
        let client = SyncClient::new("ws://localhost:8080".to_string(), "test-key".to_string());
        let url = client.build_ws_url(DocType::Dishes);
        assert_eq!(url, "ws://localhost:8080/sync/dishes?key=test-key");
    }

    #[test]
    fn test_build_ws_url_with_http() {
        let client = SyncClient::new("http://localhost:8080".to_string(), "test-key".to_string());
        let url = client.build_ws_url(DocType::Dishes);
        assert_eq!(url, "ws://localhost:8080/sync/dishes?key=test-key");
    }

    #[test]
    fn test_build_ws_url_with_https() {
        let client = SyncClient::new(
            "https://sync.example.com".to_string(),
            "test-key".to_string(),
        );
        let url = client.build_ws_url(DocType::MealPlans);
        assert_eq!(url, "wss://sync.example.com/sync/mealplans?key=test-key");
    }

    #[test]
    fn test_build_ws_url_bare_host() {
        let client = SyncClient::new("localhost:8080".to_string(), "test-key".to_string());
        let url = client.build_ws_url(DocType::MealLogs);
        assert_eq!(url, "ws://localhost:8080/sync/meallogs?key=test-key");
    }
}
