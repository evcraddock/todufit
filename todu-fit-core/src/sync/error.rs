//! Sync error types.

/// Errors that can occur during sync client operations.
#[derive(Debug)]
pub enum SyncError {
    /// Sync is not configured
    NotConfigured,
    /// Failed to connect to server
    ConnectionError(String),
    /// WebSocket error
    WebSocketError(String),
    /// Sync protocol error
    ProtocolError(String),
    /// Storage error
    StorageError(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::NotConfigured => write!(
                f,
                "Sync not configured. Add server_url and api_key to config."
            ),
            SyncError::ConnectionError(e) => write!(f, "Connection error: {}", e),
            SyncError::WebSocketError(e) => write!(f, "WebSocket error: {}", e),
            SyncError::ProtocolError(e) => write!(f, "Sync protocol error: {}", e),
            SyncError::StorageError(e) => write!(f, "Storage error: {}", e),
        }
    }
}

impl std::error::Error for SyncError {}
