//! WebSocket sync client for automerge-repo-sync-server.
//!
//! Uses the automerge-repo WebSocket protocol with CBOR-encoded messages
//! to synchronize documents with the server. No authentication required.

use std::time::Duration;

use automerge::sync::{Message as SyncMessage, State as SyncState, SyncDoc};
use automerge::AutoCommit;
use futures::{SinkExt, StreamExt};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::error::SyncError;
use super::protocol::{generate_peer_id, PeerMetadata, ProtocolMessage};
use crate::document_id::DocumentId;

/// Timeout for handshake completion.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// How long to wait for any activity on a document before checking exit conditions.
const DOC_IDLE_TIMEOUT: Duration = Duration::from_secs(2);

/// Result of a sync operation for a single document.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Document ID that was synced
    pub doc_id: DocumentId,
    /// Whether the document was updated
    pub updated: bool,
    /// Number of sync round-trips
    pub rounds: usize,
}

/// Sync client for connecting to automerge-repo-sync-server.
///
/// This client uses the automerge-repo WebSocket protocol:
/// 1. Opens WebSocket connection (no authentication)
/// 2. Performs handshake (join/peer messages)
/// 3. Syncs documents by DocumentId
#[derive(Debug)]
pub struct SyncClient {
    server_url: String,
}

impl SyncClient {
    /// Creates a new sync client.
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
        }
    }

    /// Returns the server URL.
    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    /// Syncs multiple documents with the server over a single connection.
    ///
    /// This is the primary sync method. It:
    /// 1. Opens WebSocket connection
    /// 2. Performs handshake
    /// 3. Syncs each document
    /// 4. Closes connection
    ///
    /// Documents are passed as (DocumentId, AutoCommit) pairs. The AutoCommit
    /// documents are modified in place with synced changes.
    pub async fn sync_documents(
        &self,
        docs: &mut [(DocumentId, AutoCommit)],
    ) -> Result<Vec<SyncResult>, SyncError> {
        if docs.is_empty() {
            return Ok(Vec::new());
        }

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

        for (doc_id, doc) in docs.iter_mut() {
            let result = self
                .sync_document_over_connection(
                    &mut sender,
                    &mut receiver,
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

    /// Syncs a single document with the server.
    ///
    /// This is a convenience method that opens a connection for a single document.
    /// For efficiency when syncing multiple documents, use `sync_documents`.
    pub async fn sync_document(
        &self,
        doc_id: &DocumentId,
        doc: &mut AutoCommit,
    ) -> Result<SyncResult, SyncError> {
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

        // Sync the document
        let result = self
            .sync_document_over_connection(
                &mut sender,
                &mut receiver,
                doc_id,
                &peer_id,
                &server_peer_id,
                doc,
            )
            .await?;

        // Send Leave message before closing
        let leave_msg = ProtocolMessage::Leave {
            sender_id: peer_id.clone(),
        };
        if let Ok(encoded) = leave_msg.encode() {
            let _ = sender.send(Message::Binary(encoded.into())).await;
        }

        // Close WebSocket gracefully
        let _ = sender.send(Message::Close(None)).await;

        Ok(result)
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
                        // Ping during handshake - ignore
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
    async fn sync_document_over_connection<S, R>(
        &self,
        sender: &mut S,
        receiver: &mut R,
        doc_id: &DocumentId,
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

        // Use automerge URL format for document ID
        let doc_id_str = doc_id.to_url();

        // Initialize sync state
        let mut sync_state = SyncState::new();
        let mut rounds = 0;

        // Generate initial sync message - always send Request even if no sync data
        let sync_data = doc
            .sync()
            .generate_sync_message(&mut sync_state)
            .map(|m| m.encode())
            .unwrap_or_default();

        let protocol_msg = ProtocolMessage::Request {
            document_id: doc_id_str.clone(),
            sender_id: peer_id.to_string(),
            target_id: server_peer_id.to_string(),
            doc_type: String::new(), // Not used in no-auth mode
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
                            if document_id != doc_id_str {
                                // Message for a different document - skip
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
                                    document_id: doc_id_str.clone(),
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
                                // No more messages needed, sync complete
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
                            // Ignore other message types
                        }
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => {
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
            doc_id: *doc_id,
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

        base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ws_url() {
        let client = SyncClient::new("ws://localhost:8080");
        assert_eq!(client.build_ws_url(), "ws://localhost:8080");

        let client = SyncClient::new("http://localhost:8080");
        assert_eq!(client.build_ws_url(), "ws://localhost:8080");

        let client = SyncClient::new("https://sync.example.com");
        assert_eq!(client.build_ws_url(), "wss://sync.example.com");

        let client = SyncClient::new("localhost:8080");
        assert_eq!(client.build_ws_url(), "ws://localhost:8080");
    }

    #[test]
    fn test_client_accessors() {
        let client = SyncClient::new("ws://localhost:8080");
        assert_eq!(client.server_url(), "ws://localhost:8080");
    }
}
