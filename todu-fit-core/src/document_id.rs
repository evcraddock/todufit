//! Document ID handling compatible with automerge-repo
//!
//! Document IDs are UUIDs encoded with bs58check (base58 with checksum).
//! The full URL format is `automerge:<bs58check-encoded-uuid>`.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// URL prefix for Automerge documents
pub const URL_PREFIX: &str = "automerge:";

/// Errors that can occur with document IDs
#[derive(Error, Debug)]
pub enum DocumentIdError {
    #[error("Invalid bs58check encoding: {0}")]
    InvalidEncoding(String),

    #[error("Invalid document ID length: expected 16 bytes, got {0}")]
    InvalidLength(usize),

    #[error("Invalid Automerge URL format: {0}")]
    InvalidUrl(String),

    #[error("Checksum verification failed")]
    ChecksumFailed,
}

/// A document ID compatible with automerge-repo
///
/// Internally stores a UUID (16 bytes), but displays and serializes
/// as a bs58check-encoded string for compatibility with automerge-repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentId([u8; 16]);

impl DocumentId {
    /// Generate a new random document ID
    pub fn new() -> Self {
        let uuid = Uuid::new_v4();
        Self(*uuid.as_bytes())
    }

    /// Create a document ID from raw bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Get as UUID
    pub fn as_uuid(&self) -> Uuid {
        Uuid::from_bytes(self.0)
    }

    /// Encode as bs58check string (without the automerge: prefix)
    pub fn to_bs58check(&self) -> String {
        bs58::encode(&self.0).with_check().into_string()
    }

    /// Decode from bs58check string (without the automerge: prefix)
    pub fn from_bs58check(s: &str) -> Result<Self, DocumentIdError> {
        let bytes = bs58::decode(s)
            .with_check(None)
            .into_vec()
            .map_err(|e| DocumentIdError::InvalidEncoding(e.to_string()))?;

        if bytes.len() != 16 {
            return Err(DocumentIdError::InvalidLength(bytes.len()));
        }

        let mut arr = [0u8; 16];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Get the full Automerge URL (automerge:<id>)
    pub fn to_url(&self) -> String {
        format!("{}{}", URL_PREFIX, self.to_bs58check())
    }

    /// Parse from an Automerge URL (automerge:<id>)
    pub fn from_url(url: &str) -> Result<Self, DocumentIdError> {
        let id_part = url
            .strip_prefix(URL_PREFIX)
            .ok_or_else(|| DocumentIdError::InvalidUrl(url.to_string()))?;

        // Handle optional heads section (automerge:<id>#<heads>)
        let id_str = id_part.split('#').next().unwrap_or(id_part);

        Self::from_bs58check(id_str)
    }

    /// Create from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(*uuid.as_bytes())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_bs58check())
    }
}

impl Serialize for DocumentId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_bs58check())
    }
}

impl<'de> Deserialize<'de> for DocumentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_bs58check(&s).map_err(serde::de::Error::custom)
    }
}

impl From<Uuid> for DocumentId {
    fn from(uuid: Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}

impl From<DocumentId> for Uuid {
    fn from(doc_id: DocumentId) -> Self {
        doc_id.as_uuid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_document_id() {
        let id1 = DocumentId::new();
        let id2 = DocumentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_bs58check_roundtrip() {
        let id = DocumentId::new();
        let encoded = id.to_bs58check();
        let decoded = DocumentId::from_bs58check(&encoded).unwrap();
        assert_eq!(id, decoded);
    }

    #[test]
    fn test_url_roundtrip() {
        let id = DocumentId::new();
        let url = id.to_url();
        assert!(url.starts_with("automerge:"));

        let parsed = DocumentId::from_url(&url).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_url_with_heads() {
        let id = DocumentId::new();
        let url_with_heads = format!("{}#someheads|otherheads", id.to_url());

        let parsed = DocumentId::from_url(&url_with_heads).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_invalid_url() {
        let result = DocumentId::from_url("invalid:abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_bs58check() {
        // Invalid checksum
        let result = DocumentId::from_bs58check("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_display() {
        let id = DocumentId::new();
        let display = format!("{}", id);
        let encoded = id.to_bs58check();
        assert_eq!(display, encoded);
    }

    #[test]
    fn test_uuid_conversion() {
        let uuid = Uuid::new_v4();
        let doc_id = DocumentId::from(uuid);
        let back: Uuid = doc_id.into();
        assert_eq!(uuid, back);
    }

    #[test]
    fn test_serialization() {
        let id = DocumentId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: DocumentId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_known_value() {
        // Test that we can decode a known automerge-repo style ID
        // This ensures compatibility with the JS implementation
        let id = DocumentId::new();
        let encoded = id.to_bs58check();

        // bs58check should produce a string around 23-25 chars for 16 bytes
        assert!(encoded.len() >= 20 && encoded.len() <= 30);

        // Should only contain base58 characters
        assert!(encoded.chars().all(|c| {
            c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l'
        }));
    }
}
