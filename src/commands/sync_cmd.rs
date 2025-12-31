//! Sync CLI commands for synchronizing with the server.

use clap::{Args, Subcommand};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::sync::{DocType, SyncClient, SyncClientError};

/// Sync with remote server
#[derive(Debug, Args)]
pub struct SyncCommand {
    #[command(subcommand)]
    command: Option<SyncSubcommand>,
}

#[derive(Debug, Subcommand)]
enum SyncSubcommand {
    /// Show sync configuration and server status
    Status,
}

impl SyncCommand {
    pub async fn run(&self, pool: &SqlitePool, config: &Config) -> Result<(), SyncCommandError> {
        match &self.command {
            None => self.sync(pool, config).await,
            Some(SyncSubcommand::Status) => self.status(config).await,
        }
    }

    async fn sync(&self, pool: &SqlitePool, config: &Config) -> Result<(), SyncCommandError> {
        let client = SyncClient::from_config(&config.sync)?;

        println!("Syncing with server...");
        println!();

        let results = client.sync_and_project(pool).await?;

        let mut any_updated = false;
        for result in &results {
            let status = if result.updated {
                any_updated = true;
                "✓ updated"
            } else {
                "✓ up to date"
            };
            println!(
                "  {} {} ({} round{})",
                status,
                doc_type_name(result.doc_type),
                result.rounds,
                if result.rounds == 1 { "" } else { "s" }
            );
        }

        println!();
        if any_updated {
            println!("Sync complete.");
        } else {
            println!("Already up to date.");
        }

        Ok(())
    }

    async fn status(&self, config: &Config) -> Result<(), SyncCommandError> {
        println!("Sync Configuration");
        println!("==================");
        println!();

        if !config.sync.is_configured() {
            println!("Status: Not configured");
            println!();
            println!("To enable sync, add to your config file:");
            println!();
            println!("  sync:");
            println!("    server_url: \"ws://localhost:8080\"");
            println!("    api_key: \"your-api-key\"");
            println!("    auto_sync: false");
            println!();
            println!("Or set environment variables:");
            println!("  TODUFIT_SYNC_URL");
            println!("  TODUFIT_SYNC_API_KEY");
            return Ok(());
        }

        let server_url = config.sync.server_url.as_ref().unwrap();
        let api_key = config.sync.api_key.as_ref().unwrap();

        println!("Server:    {}", server_url);
        println!("API Key:   {}...", &api_key[..api_key.len().min(8)]);
        println!(
            "Auto-sync: {}",
            if config.sync.auto_sync {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!();

        // Try to connect to check server status
        print!("Server status: ");

        // Try WebSocket connection to verify connectivity
        let client = SyncClient::from_config(&config.sync)?;
        match client.sync_document(DocType::Dishes).await {
            Ok(_) => println!("✓ connected"),
            Err(SyncClientError::ConnectionError(_)) => println!("✗ unreachable"),
            Err(e) => println!("✗ error: {}", e),
        }

        Ok(())
    }
}

fn doc_type_name(doc_type: DocType) -> &'static str {
    match doc_type {
        DocType::Dishes => "dishes",
        DocType::MealPlans => "mealplans",
        DocType::MealLogs => "meallogs",
    }
}

/// Errors from sync commands
#[derive(Debug)]
pub enum SyncCommandError {
    SyncError(SyncClientError),
}

impl std::fmt::Display for SyncCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncCommandError::SyncError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for SyncCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SyncCommandError::SyncError(e) => Some(e),
        }
    }
}

impl From<SyncClientError> for SyncCommandError {
    fn from(e: SyncClientError) -> Self {
        SyncCommandError::SyncError(e)
    }
}
