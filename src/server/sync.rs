//! WebSocket sync handler for Automerge documents.
//!
//! Handles the Automerge sync protocol over WebSocket connections.
//! Multiple clients in the same group can sync documents in real-time.

use automerge::sync::SyncDoc;
use automerge::{sync, AutoCommit};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use super::storage::{DocType, ServerStorage};

/// A sync session for a specific document.
///
/// Tracks the sync state for a client syncing a particular document.
pub struct SyncSession {
    /// The sync state for this client
    sync_state: sync::State,
}

impl SyncSession {
    /// Creates a new sync session.
    pub fn new() -> Self {
        Self {
            sync_state: sync::State::new(),
        }
    }

    /// Generates a sync message to send to the client.
    ///
    /// Returns None if no sync is needed.
    pub fn generate_sync_message(&mut self, doc: &mut AutoCommit) -> Option<Vec<u8>> {
        doc.sync()
            .generate_sync_message(&mut self.sync_state)
            .map(|msg| msg.encode())
    }

    /// Receives a sync message from the client and applies it to the document.
    ///
    /// Returns the patches applied (if any).
    pub fn receive_sync_message(
        &mut self,
        doc: &mut AutoCommit,
        message: &[u8],
    ) -> Result<(), SyncError> {
        let msg =
            sync::Message::decode(message).map_err(|e| SyncError::DecodeError(e.to_string()))?;

        doc.sync()
            .receive_sync_message(&mut self.sync_state, msg)
            .map_err(|e| SyncError::SyncError(e.to_string()))?;

        Ok(())
    }
}

impl Default for SyncSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during sync.
#[derive(Debug)]
pub enum SyncError {
    /// Error decoding a sync message.
    DecodeError(String),
    /// Error during sync protocol.
    SyncError(String),
    /// Storage error.
    StorageError(String),
    /// Invalid document type.
    InvalidDocType(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::DecodeError(e) => write!(f, "Failed to decode sync message: {}", e),
            SyncError::SyncError(e) => write!(f, "Sync error: {}", e),
            SyncError::StorageError(e) => write!(f, "Storage error: {}", e),
            SyncError::InvalidDocType(t) => write!(f, "Invalid document type: {}", t),
        }
    }
}

impl std::error::Error for SyncError {}

/// Message types for WebSocket communication.
#[derive(Debug, Clone)]
pub enum SyncMessage {
    /// Automerge sync protocol message
    Sync(Vec<u8>),
    /// Document updated notification (for broadcasting)
    Updated,
}

/// Tracks all connected clients for broadcasting updates.
pub struct SyncHub {
    /// Broadcast channels per group+doc_type
    /// Key: (group_id, doc_type)
    channels: RwLock<HashMap<(String, DocType), broadcast::Sender<SyncMessage>>>,
}

impl SyncHub {
    /// Creates a new sync hub.
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribes to updates for a group+doc_type.
    pub async fn subscribe(
        &self,
        group_id: &str,
        doc_type: DocType,
    ) -> broadcast::Receiver<SyncMessage> {
        let key = (group_id.to_string(), doc_type);

        let mut channels = self.channels.write().await;

        if let Some(sender) = channels.get(&key) {
            sender.subscribe()
        } else {
            // Create new channel with buffer of 16 messages
            let (sender, receiver) = broadcast::channel(16);
            channels.insert(key, sender);
            receiver
        }
    }

    /// Broadcasts an update to all subscribers for a group+doc_type.
    pub async fn broadcast(&self, group_id: &str, doc_type: DocType, message: SyncMessage) {
        let key = (group_id.to_string(), doc_type);

        let channels = self.channels.read().await;

        if let Some(sender) = channels.get(&key) {
            // Ignore send errors (no subscribers)
            let _ = sender.send(message);
        }
    }
}

impl Default for SyncHub {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages sync operations for a connected client.
///
/// Maintains sync state across the WebSocket session.
pub struct ClientSync {
    storage: Arc<RwLock<ServerStorage>>,
    hub: Arc<SyncHub>,
    group_id: String,
    user_id: String,
    /// Sync state maintained across the session
    sync_state: sync::State,
}

impl ClientSync {
    /// Creates a new client sync manager.
    pub fn new(
        storage: Arc<RwLock<ServerStorage>>,
        hub: Arc<SyncHub>,
        group_id: String,
        user_id: String,
    ) -> Self {
        Self {
            storage,
            hub,
            group_id,
            user_id,
            sync_state: sync::State::new(),
        }
    }

    /// Performs a sync round for a document type.
    ///
    /// Returns sync message to send to the client, if any.
    pub async fn sync_document(
        &mut self,
        doc_type: DocType,
        client_message: Option<&[u8]>,
    ) -> Result<Option<Vec<u8>>, SyncError> {
        let storage = self.storage.write().await;

        // Load or create the document
        let mut doc = storage
            .load(&self.group_id, doc_type)
            .map_err(|e| SyncError::StorageError(e.to_string()))?
            .unwrap_or_else(AutoCommit::new);

        let doc_heads = doc.get_heads().len();
        tracing::debug!(
            "sync_document for {}/{:?}: doc has {} heads, client_message={}",
            self.group_id,
            doc_type,
            doc_heads,
            client_message.is_some()
        );

        // If client sent a message, apply it
        if let Some(msg_bytes) = client_message {
            tracing::debug!("Received {} bytes from client", msg_bytes.len());

            let msg = sync::Message::decode(msg_bytes)
                .map_err(|e| SyncError::DecodeError(e.to_string()))?;

            doc.sync()
                .receive_sync_message(&mut self.sync_state, msg)
                .map_err(|e| SyncError::SyncError(e.to_string()))?;

            // Save the updated document
            storage
                .save(&self.group_id, doc_type, &mut doc)
                .map_err(|e| SyncError::StorageError(e.to_string()))?;

            tracing::debug!(
                "Applied sync from {} for {}/{:?}, doc now has {} heads",
                self.user_id,
                self.group_id,
                doc_type,
                doc.get_heads().len()
            );

            // Broadcast update to other clients
            self.hub
                .broadcast(&self.group_id, doc_type, SyncMessage::Updated)
                .await;
        }

        // Generate response message
        let response = doc
            .sync()
            .generate_sync_message(&mut self.sync_state)
            .map(|msg| msg.encode());

        tracing::debug!(
            "Generated response for {}/{:?}: {} bytes",
            self.group_id,
            doc_type,
            response.as_ref().map(|r| r.len()).unwrap_or(0)
        );

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::transaction::Transactable;
    use automerge::ROOT;

    #[test]
    fn test_sync_session_new() {
        let session = SyncSession::new();
        // Just verify we can create a session
        assert!(session.sync_state.their_heads.is_none());
    }

    #[test]
    fn test_sync_session_generate_message_empty_doc() {
        let mut session = SyncSession::new();
        let mut doc = AutoCommit::new();

        // First message should be a sync request
        let msg = session.generate_sync_message(&mut doc);
        assert!(msg.is_some());
    }

    #[test]
    fn test_sync_two_docs() {
        // Create two documents with different content
        let mut doc1 = AutoCommit::new();
        doc1.put(ROOT, "key1", "value1").unwrap();

        let mut doc2 = AutoCommit::new();
        doc2.put(ROOT, "key2", "value2").unwrap();

        // Create sync sessions
        let mut session1 = SyncSession::new();
        let mut session2 = SyncSession::new();

        // Sync doc1 -> doc2
        if let Some(msg) = session1.generate_sync_message(&mut doc1) {
            session2.receive_sync_message(&mut doc2, &msg).unwrap();
        }

        // Sync doc2 -> doc1
        if let Some(msg) = session2.generate_sync_message(&mut doc2) {
            session1.receive_sync_message(&mut doc1, &msg).unwrap();
        }

        // Continue syncing until no more messages
        loop {
            let msg1 = session1.generate_sync_message(&mut doc1);
            let msg2 = session2.generate_sync_message(&mut doc2);

            if msg1.is_none() && msg2.is_none() {
                break;
            }

            if let Some(msg) = msg1 {
                session2.receive_sync_message(&mut doc2, &msg).unwrap();
            }
            if let Some(msg) = msg2 {
                session1.receive_sync_message(&mut doc1, &msg).unwrap();
            }
        }

        // Both docs should now have both keys
        use automerge::ReadDoc;
        assert!(doc1.get(ROOT, "key1").unwrap().is_some());
        assert!(doc1.get(ROOT, "key2").unwrap().is_some());
        assert!(doc2.get(ROOT, "key1").unwrap().is_some());
        assert!(doc2.get(ROOT, "key2").unwrap().is_some());
    }

    #[tokio::test]
    async fn test_sync_hub_subscribe_and_broadcast() {
        let hub = SyncHub::new();

        // Subscribe
        let mut rx = hub.subscribe("group1", DocType::Dishes).await;

        // Broadcast
        hub.broadcast("group1", DocType::Dishes, SyncMessage::Updated)
            .await;

        // Should receive the message
        let msg = rx.try_recv().unwrap();
        assert!(matches!(msg, SyncMessage::Updated));
    }

    #[tokio::test]
    async fn test_sync_hub_isolated_groups() {
        let hub = SyncHub::new();

        // Subscribe to different groups
        let mut rx1 = hub.subscribe("group1", DocType::Dishes).await;
        let mut rx2 = hub.subscribe("group2", DocType::Dishes).await;

        // Broadcast to group1 only
        hub.broadcast("group1", DocType::Dishes, SyncMessage::Updated)
            .await;

        // group1 should receive
        assert!(rx1.try_recv().is_ok());

        // group2 should not receive
        assert!(rx2.try_recv().is_err());
    }
}
