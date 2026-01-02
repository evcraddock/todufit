//! WebSocket sync client for connecting to the Todu Fit sync server.
//!
//! Uses the Automerge sync protocol over WebSocket to synchronize
//! documents with the server.

use automerge::sync::{Message as SyncMessage, State as SyncState, SyncDoc};
use automerge::AutoCommit;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::SyncError;
use crate::automerge::DocType;

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

/// Sync client for connecting to the Todu Fit sync server.
///
/// This is a stateless client that can sync individual documents.
/// Storage and projection are handled externally.
pub struct SyncClient {
    server_url: String,
    api_key: String,
}

impl SyncClient {
    /// Creates a new sync client with explicit parameters.
    pub fn new(server_url: String, api_key: String) -> Self {
        Self {
            server_url,
            api_key,
        }
    }

    /// Returns the server URL.
    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    /// Returns the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Syncs a document with the server.
    ///
    /// The document is modified in place with any changes from the server.
    /// Returns a SyncResult indicating whether changes were made.
    pub async fn sync_document(
        &self,
        doc_type: DocType,
        doc: &mut AutoCommit,
    ) -> Result<SyncResult, SyncError> {
        // Build WebSocket URL
        let ws_url = self.build_ws_url(doc_type);

        // Connect to server
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| SyncError::ConnectionError(e.to_string()))?;

        let (mut sender, mut receiver) = ws_stream.split();

        let initial_heads = doc.get_heads().to_vec();

        // Initialize sync state
        let mut sync_state = SyncState::new();
        let mut rounds = 0;

        // Generate initial sync message
        if let Some(msg) = doc.sync().generate_sync_message(&mut sync_state) {
            sender
                .send(Message::Binary(msg.encode().into()))
                .await
                .map_err(|e| SyncError::WebSocketError(e.to_string()))?;
            rounds += 1;
        }

        // Sync loop
        loop {
            match receiver.next().await {
                Some(Ok(Message::Binary(data))) => {
                    // Decode and apply server's sync message
                    let msg = SyncMessage::decode(&data)
                        .map_err(|e| SyncError::ProtocolError(e.to_string()))?;

                    doc.sync()
                        .receive_sync_message(&mut sync_state, msg)
                        .map_err(|e| SyncError::ProtocolError(e.to_string()))?;

                    // Generate response if needed
                    if let Some(response) = doc.sync().generate_sync_message(&mut sync_state) {
                        sender
                            .send(Message::Binary(response.encode().into()))
                            .await
                            .map_err(|e| SyncError::WebSocketError(e.to_string()))?;
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
                        .map_err(|e| SyncError::WebSocketError(e.to_string()))?;
                }
                Some(Ok(_)) => {
                    // Ignore other message types
                }
                Some(Err(e)) => {
                    return Err(SyncError::WebSocketError(e.to_string()));
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

        Ok(SyncResult {
            doc_type,
            updated,
            rounds,
        })
    }

    /// Builds the WebSocket URL for a document type.
    pub fn build_ws_url(&self, doc_type: DocType) -> String {
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

    #[test]
    fn test_client_accessors() {
        let client = SyncClient::new("ws://localhost:8080".to_string(), "my-key".to_string());
        assert_eq!(client.server_url(), "ws://localhost:8080");
        assert_eq!(client.api_key(), "my-key");
    }
}
