//! WebSocket sync client for connecting to the Todu Fit sync server.
//!
//! Uses the automerge-repo WebSocket protocol with CBOR-encoded messages
//! to synchronize documents with the server.

use std::collections::HashMap;
use std::time::Duration;

use automerge::sync::{Message as SyncMessage, State as SyncState, SyncDoc};
use automerge::AutoCommit;
use futures::{SinkExt, StreamExt};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::error::SyncError;
use super::protocol::{
    generate_doc_id, generate_peer_id, MeResponse, PeerMetadata, ProtocolMessage,
};
use crate::automerge::DocType;

/// Timeout for handshake completion.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// How long to wait for any activity on a document before checking exit conditions.
const DOC_IDLE_TIMEOUT: Duration = Duration::from_secs(2);

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

/// Identity information obtained from the /me endpoint.
#[derive(Debug, Clone)]
pub struct Identity {
    pub user_id: String,
    pub group_id: String,
}

/// Sync client for connecting to the Todu Fit sync server.
///
/// This client uses the automerge-repo WebSocket protocol:
/// 1. Fetches identity via /me endpoint
/// 2. Opens a single WebSocket connection
/// 3. Performs handshake (join/peer messages)
/// 4. Syncs all documents over the single connection
#[derive(Debug)]
pub struct SyncClient {
    server_url: String,
    api_key: String,
    /// Cached identity from /me endpoint
    identity: Option<Identity>,
}

impl SyncClient {
    /// Creates a new sync client with explicit parameters.
    pub fn new(server_url: String, api_key: String) -> Self {
        Self {
            server_url,
            api_key,
            identity: None,
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

    /// Fetches identity (user_id, group_id) from the /me endpoint.
    ///
    /// Results are cached for subsequent calls.
    pub async fn fetch_identity(&mut self) -> Result<&Identity, SyncError> {
        if self.identity.is_some() {
            return Ok(self.identity.as_ref().unwrap());
        }

        let http_url = self.build_http_url("/me");
        let client = reqwest::Client::new();

        let response = client
            .get(&http_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| SyncError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SyncError::HttpError(format!(
                "Server returned status {}",
                response.status()
            )));
        }

        let me: MeResponse = response
            .json()
            .await
            .map_err(|e| SyncError::HttpError(e.to_string()))?;

        self.identity = Some(Identity {
            user_id: me.user_id,
            group_id: me.group_id,
        });

        Ok(self.identity.as_ref().unwrap())
    }

    /// Syncs all document types with the server over a single connection.
    ///
    /// This is the primary sync method. It:
    /// 1. Fetches identity if not cached
    /// 2. Opens WebSocket connection
    /// 3. Performs handshake
    /// 4. Syncs each document type
    /// 5. Closes connection
    pub async fn sync_all(
        &mut self,
        docs: &mut HashMap<DocType, AutoCommit>,
    ) -> Result<Vec<SyncResult>, SyncError> {
        // Ensure we have identity
        self.fetch_identity().await?;
        let identity = self.identity.as_ref().unwrap();

        // Compute document IDs
        let doc_ids: HashMap<DocType, String> = [
            (
                DocType::Dishes,
                generate_doc_id(&identity.group_id, "dishes"),
            ),
            (
                DocType::MealPlans,
                generate_doc_id(&identity.group_id, "mealplans"),
            ),
            (
                DocType::MealLogs,
                generate_doc_id(&identity.user_id, "meallogs"),
            ),
        ]
        .into_iter()
        .collect();

        // Connect to WebSocket
        let ws_url = self.build_ws_url();
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| SyncError::ConnectionError(e.to_string()))?;

        let (mut sender, mut receiver) = ws_stream.split();

        // Generate peer ID for this connection
        let peer_id = generate_peer_id();

        // Perform handshake
        let server_peer_id = self
            .perform_handshake(&mut sender, &mut receiver, &peer_id)
            .await?;

        // Sync each document
        let mut results = Vec::new();

        for doc_type in [DocType::Dishes, DocType::MealPlans, DocType::MealLogs] {
            let doc_id = doc_ids.get(&doc_type).unwrap();

            // Get or create document
            let doc = docs.entry(doc_type).or_default();

            let result = self
                .sync_document_over_connection(
                    &mut sender,
                    &mut receiver,
                    doc_type,
                    doc_id,
                    &peer_id,
                    &server_peer_id,
                    doc,
                )
                .await?;

            results.push(result);
        }

        // Send Leave message before closing
        let leave_msg = ProtocolMessage::Leave {
            sender_id: peer_id.clone(),
        };
        if let Ok(encoded) = leave_msg.encode() {
            let _ = sender.send(Message::Binary(encoded.into())).await;
        }

        // Close WebSocket gracefully
        let _ = sender.send(Message::Close(None)).await;

        Ok(results)
    }

    /// Syncs a single document with the server (legacy method for compatibility).
    ///
    /// Note: This opens a new connection for each call. For efficiency,
    /// prefer using `sync_all` to sync all documents over a single connection.
    pub async fn sync_document(
        &mut self,
        doc_type: DocType,
        doc: &mut AutoCommit,
    ) -> Result<SyncResult, SyncError> {
        let mut docs: HashMap<DocType, AutoCommit> = HashMap::new();

        // Take ownership temporarily
        let temp_doc = std::mem::replace(doc, AutoCommit::new());
        docs.insert(doc_type, temp_doc);

        let results = self.sync_all(&mut docs).await?;

        // Put the document back
        if let Some(synced_doc) = docs.remove(&doc_type) {
            *doc = synced_doc;
        }

        // Return result for the requested doc type
        results
            .into_iter()
            .find(|r| r.doc_type == doc_type)
            .ok_or_else(|| SyncError::ProtocolError("Document not synced".to_string()))
    }

    /// Performs the handshake with the server.
    ///
    /// Sends a `join` message and waits for a `peer` response.
    async fn perform_handshake<S, R>(
        &self,
        sender: &mut S,
        receiver: &mut R,
        peer_id: &str,
    ) -> Result<String, SyncError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
        R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        // Send join message
        let join_msg = ProtocolMessage::Join {
            sender_id: peer_id.to_string(),
            supported_protocol_versions: vec!["1".to_string()],
            metadata: Some(PeerMetadata {
                storage_id: None,
                is_ephemeral: true,
            }),
        };

        let encoded = join_msg
            .encode()
            .map_err(|e| SyncError::CborError(e.to_string()))?;

        sender
            .send(Message::Binary(encoded.into()))
            .await
            .map_err(|e| SyncError::WebSocketError(e.to_string()))?;

        // Wait for peer response with timeout
        let peer_response = timeout(HANDSHAKE_TIMEOUT, async {
            while let Some(msg_result) = receiver.next().await {
                match msg_result {
                    Ok(Message::Binary(data)) => {
                        let msg = ProtocolMessage::decode(&data)
                            .map_err(|e| SyncError::CborError(e.to_string()))?;

                        match msg {
                            ProtocolMessage::Peer {
                                sender_id,
                                target_id,
                                selected_protocol_version: _,
                            } => {
                                if target_id != peer_id {
                                    return Err(SyncError::HandshakeError(
                                        "Peer response target_id mismatch".to_string(),
                                    ));
                                }
                                return Ok(sender_id);
                            }
                            ProtocolMessage::Error { message } => {
                                return Err(SyncError::HandshakeError(message));
                            }
                            _ => {
                                return Err(SyncError::HandshakeError(format!(
                                    "Unexpected message during handshake: {:?}",
                                    msg
                                )));
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        // Ping during handshake - we can't respond here easily
                        // The server shouldn't ping during handshake
                        let _ = data;
                    }
                    Ok(Message::Close(_)) => {
                        return Err(SyncError::HandshakeError(
                            "Server closed connection during handshake".to_string(),
                        ));
                    }
                    Ok(_) => {
                        // Ignore other message types
                    }
                    Err(e) => {
                        return Err(SyncError::WebSocketError(e.to_string()));
                    }
                }
            }
            Err(SyncError::HandshakeError(
                "Connection closed before handshake completed".to_string(),
            ))
        })
        .await;

        match peer_response {
            Ok(result) => result,
            Err(_) => Err(SyncError::HandshakeTimeout),
        }
    }

    /// Syncs a single document over an established connection.
    #[allow(clippy::too_many_arguments)]
    async fn sync_document_over_connection<S, R>(
        &self,
        sender: &mut S,
        receiver: &mut R,
        doc_type: DocType,
        doc_id: &str,
        peer_id: &str,
        server_peer_id: &str,
        doc: &mut AutoCommit,
    ) -> Result<SyncResult, SyncError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
        R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        let initial_heads = doc.get_heads().to_vec();

        // Initialize sync state
        let mut sync_state = SyncState::new();
        let mut rounds = 0;

        // Get doc_type string for protocol
        let doc_type_str = match doc_type {
            DocType::Dishes => "dishes",
            DocType::MealPlans => "mealplans",
            DocType::MealLogs => "meallogs",
        };

        // Generate initial sync message - always send Request even if no sync data
        let sync_data = doc
            .sync()
            .generate_sync_message(&mut sync_state)
            .map(|m| m.encode())
            .unwrap_or_default();

        let protocol_msg = ProtocolMessage::Request {
            document_id: doc_id.to_string(),
            sender_id: peer_id.to_string(),
            target_id: server_peer_id.to_string(),
            doc_type: doc_type_str.to_string(),
            data: sync_data,
        };

        let encoded = protocol_msg
            .encode()
            .map_err(|e| SyncError::CborError(e.to_string()))?;

        sender
            .send(Message::Binary(encoded.into()))
            .await
            .map_err(|e| SyncError::WebSocketError(e.to_string()))?;

        rounds += 1;

        // Sync loop
        loop {
            match timeout(DOC_IDLE_TIMEOUT, receiver.next()).await {
                Ok(Some(Ok(Message::Binary(data)))) => {
                    let msg = ProtocolMessage::decode(&data)
                        .map_err(|e| SyncError::CborError(e.to_string()))?;

                    match msg {
                        ProtocolMessage::Sync {
                            document_id,
                            data,
                            sender_id: _,
                            target_id: _,
                        }
                        | ProtocolMessage::Request {
                            document_id,
                            data,
                            sender_id: _,
                            target_id: _,
                            doc_type: _,
                        } => {
                            if document_id != doc_id {
                                // Message for a different document - shouldn't happen
                                // in sequential sync, but skip if it does
                                continue;
                            }

                            // Decode and apply server's sync message
                            let sync_msg = SyncMessage::decode(&data)
                                .map_err(|e| SyncError::ProtocolError(e.to_string()))?;

                            doc.sync()
                                .receive_sync_message(&mut sync_state, sync_msg)
                                .map_err(|e| SyncError::ProtocolError(e.to_string()))?;

                            // Generate response if needed
                            if let Some(response) =
                                doc.sync().generate_sync_message(&mut sync_state)
                            {
                                let protocol_msg = ProtocolMessage::Sync {
                                    document_id: doc_id.to_string(),
                                    sender_id: peer_id.to_string(),
                                    target_id: server_peer_id.to_string(),
                                    data: response.encode(),
                                };

                                let encoded = protocol_msg
                                    .encode()
                                    .map_err(|e| SyncError::CborError(e.to_string()))?;

                                sender
                                    .send(Message::Binary(encoded.into()))
                                    .await
                                    .map_err(|e| SyncError::WebSocketError(e.to_string()))?;

                                rounds += 1;
                            } else {
                                // No more messages needed, sync complete for this document
                                break;
                            }
                        }
                        ProtocolMessage::DocUnavailable { document_id, .. } => {
                            return Err(SyncError::DocumentUnavailable(document_id));
                        }
                        ProtocolMessage::Error { message } => {
                            return Err(SyncError::ProtocolError(message));
                        }
                        _ => {
                            // Ignore other message types (Peer, Join - shouldn't happen here)
                        }
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => {
                    // Server closed connection
                    break;
                }
                Ok(Some(Ok(Message::Ping(data)))) => {
                    sender
                        .send(Message::Pong(data))
                        .await
                        .map_err(|e| SyncError::WebSocketError(e.to_string()))?;
                }
                Ok(Some(Ok(_))) => {
                    // Ignore other message types
                }
                Ok(Some(Err(e))) => {
                    return Err(SyncError::WebSocketError(e.to_string()));
                }
                Ok(None) => {
                    // Connection closed
                    break;
                }
                Err(_) => {
                    // No activity during idle window - assume sync complete
                    break;
                }
            }
        }

        // Check if document was updated
        let final_heads = doc.get_heads().to_vec();
        let updated = initial_heads != final_heads;

        Ok(SyncResult {
            doc_type,
            updated,
            rounds,
        })
    }

    /// Builds the WebSocket URL for the sync endpoint.
    fn build_ws_url(&self) -> String {
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

        format!("{}/sync?key={}", base_url, self.api_key)
    }

    /// Builds an HTTP URL for a given path.
    fn build_http_url(&self, path: &str) -> String {
        // Convert ws(s) to http(s) if needed
        let base_url = if self.server_url.starts_with("ws://") {
            self.server_url.replace("ws://", "http://")
        } else if self.server_url.starts_with("wss://") {
            self.server_url.replace("wss://", "https://")
        } else if !self.server_url.starts_with("http://")
            && !self.server_url.starts_with("https://")
        {
            format!("http://{}", self.server_url)
        } else {
            self.server_url.clone()
        };

        format!("{}{}", base_url.trim_end_matches('/'), path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ws_url() {
        let client = SyncClient::new("ws://localhost:8080".to_string(), "test-key".to_string());
        assert_eq!(
            client.build_ws_url(),
            "ws://localhost:8080/sync?key=test-key"
        );

        let client = SyncClient::new("http://localhost:8080".to_string(), "test-key".to_string());
        assert_eq!(
            client.build_ws_url(),
            "ws://localhost:8080/sync?key=test-key"
        );

        let client = SyncClient::new(
            "https://sync.example.com".to_string(),
            "test-key".to_string(),
        );
        assert_eq!(
            client.build_ws_url(),
            "wss://sync.example.com/sync?key=test-key"
        );

        let client = SyncClient::new("localhost:8080".to_string(), "test-key".to_string());
        assert_eq!(
            client.build_ws_url(),
            "ws://localhost:8080/sync?key=test-key"
        );
    }

    #[test]
    fn test_build_http_url() {
        let client = SyncClient::new("http://localhost:8080".to_string(), "test-key".to_string());
        assert_eq!(client.build_http_url("/me"), "http://localhost:8080/me");

        let client = SyncClient::new("ws://localhost:8080".to_string(), "test-key".to_string());
        assert_eq!(client.build_http_url("/me"), "http://localhost:8080/me");

        let client = SyncClient::new(
            "https://sync.example.com".to_string(),
            "test-key".to_string(),
        );
        assert_eq!(client.build_http_url("/me"), "https://sync.example.com/me");

        let client = SyncClient::new("wss://sync.example.com".to_string(), "test-key".to_string());
        assert_eq!(client.build_http_url("/me"), "https://sync.example.com/me");
    }

    #[test]
    fn test_client_accessors() {
        let client = SyncClient::new("ws://localhost:8080".to_string(), "my-key".to_string());
        assert_eq!(client.server_url(), "ws://localhost:8080");
        assert_eq!(client.api_key(), "my-key");
    }
}
