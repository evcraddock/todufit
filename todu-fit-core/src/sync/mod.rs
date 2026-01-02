//! Sync module for connecting to the Todu Fit sync server.
//!
//! This module provides the WebSocket sync client that uses the Automerge
//! sync protocol to synchronize documents with the server.

mod client;
mod error;

pub use client::{SyncClient, SyncResult};
pub use error::SyncError;
