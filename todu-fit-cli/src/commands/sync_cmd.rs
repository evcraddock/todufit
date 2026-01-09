//! Sync CLI commands for synchronizing with the server.

use clap::{Args, Subcommand};

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
    pub fn run(&self, config: &Config) -> Result<(), SyncCommandError> {
        // Use tokio runtime for async operations
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SyncCommandError::RuntimeError(e.to_string()))?;

        match &self.command {
            None => rt.block_on(self.sync(config)),
            Some(SyncSubcommand::Status) => rt.block_on(self.status(config)),
        }
    }

    async fn sync(&self, config: &Config) -> Result<(), SyncCommandError> {
        let mut client = SyncClient::from_config(&config.sync)?;

        println!("Syncing with server...");
        println!();

        let mut any_updated = false;

        // Sync each document type
        for doc_type in [DocType::Dishes, DocType::MealPlans, DocType::MealLogs] {
            match client.sync_document(doc_type).await {
                Ok(result) => {
                    let status = if result.updated {
                        any_updated = true;
                        "✓ updated"
                    } else {
                        "✓ up to date"
                    };
                    println!(
                        "  {} {} ({} round{})",
                        status,
                        doc_type_name(doc_type),
                        result.rounds,
                        if result.rounds == 1 { "" } else { "s" }
                    );
                }
                Err(e) => {
                    println!("  ✗ {} - {}", doc_type_name(doc_type), e);
                }
            }
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
            println!();
            println!("Or set environment variable:");
            println!("  FIT_SYNC_URL");
            return Ok(());
        }

        let server_url = config.sync.server_url.as_ref().unwrap();

        println!("Server:    {}", server_url);
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
        let mut client = SyncClient::from_config(&config.sync)?;
        match client.sync_document(DocType::Dishes).await {
            Ok(_) => println!("✓ connected"),
            Err(SyncClientError::SyncError(_)) => println!("✗ unreachable"),
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
    RuntimeError(String),
}

impl std::fmt::Display for SyncCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncCommandError::SyncError(e) => write!(f, "{}", e),
            SyncCommandError::RuntimeError(e) => write!(f, "Runtime error: {}", e),
        }
    }
}

impl std::error::Error for SyncCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SyncCommandError::SyncError(e) => Some(e),
            SyncCommandError::RuntimeError(_) => None,
        }
    }
}

impl From<SyncClientError> for SyncCommandError {
    fn from(e: SyncClientError) -> Self {
        SyncCommandError::SyncError(e)
    }
}
