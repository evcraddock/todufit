//! Protocol types for automerge-repo WebSocket sync.
//!
//! These types match the CBOR message format expected by the todu-sync server.
//! Field names use camelCase to match the automerge-repo protocol.

use serde::{Deserialize, Serialize};

/// Message types for the automerge-repo protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProtocolMessage {
    /// Join message - sent by client to initiate handshake
    #[serde(rename = "join")]
    Join {
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "supportedProtocolVersions")]
        supported_protocol_versions: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<PeerMetadata>,
    },
    /// Leave message - sent by client before disconnecting
    #[serde(rename = "leave")]
    Leave {
        #[serde(rename = "senderId")]
        sender_id: String,
    },
    /// Peer message - sent by server to confirm handshake
    #[serde(rename = "peer")]
    Peer {
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "targetId")]
        target_id: String,
        #[serde(rename = "selectedProtocolVersion")]
        selected_protocol_version: String,
    },
    /// Request message - initial sync request for a document
    #[serde(rename = "request")]
    Request {
        #[serde(rename = "documentId")]
        document_id: String,
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "targetId")]
        target_id: String,
        #[serde(rename = "docType")]
        doc_type: String,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    /// Sync message - ongoing sync messages
    #[serde(rename = "sync")]
    Sync {
        #[serde(rename = "documentId")]
        document_id: String,
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "targetId")]
        target_id: String,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    /// Error message from server
    #[serde(rename = "error")]
    Error { message: String },
    /// Document unavailable response
    #[serde(rename = "doc-unavailable")]
    DocUnavailable {
        #[serde(rename = "documentId")]
        document_id: String,
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "targetId")]
        target_id: String,
    },
}

/// Metadata sent with join message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerMetadata {
    #[serde(rename = "storageId", skip_serializing_if = "Option::is_none")]
    pub storage_id: Option<String>,
    #[serde(rename = "isEphemeral")]
    pub is_ephemeral: bool,
}

/// Response from the /me endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct MeResponse {
    pub user_id: String,
    pub group_id: String,
}

impl ProtocolMessage {
    /// Encode message as CBOR bytes.
    pub fn encode(&self) -> Result<Vec<u8>, ciborium::ser::Error<std::io::Error>> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf)?;
        Ok(buf)
    }

    /// Decode message from CBOR bytes.
    pub fn decode(data: &[u8]) -> Result<Self, ciborium::de::Error<std::io::Error>> {
        ciborium::from_reader(data)
    }
}

/// Generate a document ID using the same algorithm as todu-sync.
///
/// The document ID is computed as: base58(sha256(owner_id + ":" + doc_type)[0:16])
pub fn generate_doc_id(owner_id: &str, doc_type: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(owner_id.as_bytes());
    hasher.update(b":");
    hasher.update(doc_type.as_bytes());
    let hash = hasher.finalize();
    // Use bs58check encoding (base58 with checksum) to match automerge-repo
    bs58::encode(&hash[..16]).with_check().into_string()
}

/// Generate a random peer ID for this connection.
pub fn generate_peer_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_doc_id() {
        // Test deterministic generation
        let id1 = generate_doc_id("group123", "dishes");
        let id2 = generate_doc_id("group123", "dishes");
        assert_eq!(id1, id2);

        // Different inputs produce different IDs
        let id3 = generate_doc_id("group123", "mealplans");
        assert_ne!(id1, id3);

        let id4 = generate_doc_id("group456", "dishes");
        assert_ne!(id1, id4);
    }

    #[test]
    fn test_generate_peer_id() {
        let id1 = generate_peer_id();
        let id2 = generate_peer_id();
        assert_ne!(id1, id2);
        // Should be valid UUID format
        assert!(uuid::Uuid::parse_str(&id1).is_ok());
    }

    #[test]
    fn test_join_message_encode_decode() {
        let msg = ProtocolMessage::Join {
            sender_id: "peer123".to_string(),
            supported_protocol_versions: vec!["1".to_string()],
            metadata: Some(PeerMetadata {
                storage_id: None,
                is_ephemeral: true,
            }),
        };

        let encoded = msg.encode().unwrap();
        let decoded = ProtocolMessage::decode(&encoded).unwrap();

        match decoded {
            ProtocolMessage::Join {
                sender_id,
                supported_protocol_versions,
                metadata,
            } => {
                assert_eq!(sender_id, "peer123");
                assert_eq!(supported_protocol_versions, vec!["1".to_string()]);
                assert!(metadata.is_some());
                let meta = metadata.unwrap();
                assert!(meta.storage_id.is_none());
                assert!(meta.is_ephemeral);
            }
            _ => panic!("Expected Join message"),
        }
    }

    #[test]
    fn test_request_message_encode_decode() {
        let msg = ProtocolMessage::Request {
            document_id: "doc123".to_string(),
            sender_id: "peer1".to_string(),
            target_id: "peer2".to_string(),
            doc_type: "dishes".to_string(),
            data: vec![1, 2, 3, 4, 5],
        };

        let encoded = msg.encode().unwrap();
        let decoded = ProtocolMessage::decode(&encoded).unwrap();

        match decoded {
            ProtocolMessage::Request {
                document_id,
                sender_id,
                target_id,
                doc_type,
                data,
            } => {
                assert_eq!(document_id, "doc123");
                assert_eq!(sender_id, "peer1");
                assert_eq!(target_id, "peer2");
                assert_eq!(doc_type, "dishes");
                assert_eq!(data, vec![1, 2, 3, 4, 5]);
            }
            _ => panic!("Expected Request message"),
        }
    }

    #[test]
    fn test_leave_message_encode_decode() {
        let msg = ProtocolMessage::Leave {
            sender_id: "peer123".to_string(),
        };

        let encoded = msg.encode().unwrap();
        let decoded = ProtocolMessage::decode(&encoded).unwrap();

        match decoded {
            ProtocolMessage::Leave { sender_id } => {
                assert_eq!(sender_id, "peer123");
            }
            _ => panic!("Expected Leave message"),
        }
    }
}
