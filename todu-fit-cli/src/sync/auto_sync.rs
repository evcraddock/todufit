//! Auto-sync functionality for CLI commands.
//!
//! Provides automatic synchronization before read operations and after write
//! operations when `auto_sync` is enabled in the configuration.

use todu_fit_core::check_server;

use crate::config::Config;
use crate::sync::{SyncClient, SyncClientError};

/// Performs auto-sync if enabled and server is reachable.
///
/// This function:
/// 1. Checks if auto_sync is enabled in config
/// 2. Checks if sync is configured (server_url present)
/// 3. Checks if the server is reachable
/// 4. Performs the sync operation
///
/// Any errors are silently ignored to provide graceful degradation -
/// the CLI should work offline when the server is unavailable.
pub fn try_auto_sync(config: &Config) {
    if !config.sync.auto_sync || !config.sync.is_configured() {
        return;
    }

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return,
    };

    rt.block_on(async {
        let url = match config.sync.server_url.as_ref() {
            Some(url) => url,
            None => return,
        };

        // Check server reachability first (fast fail)
        if !check_server(url).await {
            eprintln!("Auto-sync: server unreachable, skipping");
            return;
        }

        // Perform sync
        let mut client = match SyncClient::from_config(&config.sync, config.data_dir.value.clone())
        {
            Ok(c) => c,
            Err(SyncClientError::NotInitialized) => {
                // Identity not initialized yet - skip silently
                return;
            }
            Err(SyncClientError::NoGroups) => {
                // No groups yet - skip silently
                return;
            }
            Err(_) => return,
        };

        match client.sync_all().await {
            Ok(_) => {}
            Err(SyncClientError::NotInitialized | SyncClientError::NoGroups) => {
                // Expected errors when identity/groups not set up yet
            }
            Err(e) => {
                eprintln!("Auto-sync: {}", e);
            }
        }
    });
}
