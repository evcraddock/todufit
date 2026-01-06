//! Sync CLI commands for synchronizing with the server.

use automerge::AutoCommit;
use clap::{Args, Subcommand};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::db::{DishRepository, MealLogRepository, MealPlanRepository};
use crate::sync::storage::DocumentStorage;
use crate::sync::writer::{write_dish, write_meallog, write_mealplan};
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
    /// Rebuild automerge documents from SQLite database
    ///
    /// This recreates all local automerge documents from the data stored in SQLite.
    /// Use this after schema changes that invalidate existing documents (e.g., document ID encoding changes).
    Rebuild,
}

impl SyncCommand {
    pub async fn run(&self, pool: &SqlitePool, config: &Config) -> Result<(), SyncCommandError> {
        match &self.command {
            None => self.sync(pool, config).await,
            Some(SyncSubcommand::Status) => self.status(config).await,
            Some(SyncSubcommand::Rebuild) => self.rebuild(pool).await,
        }
    }

    async fn sync(&self, pool: &SqlitePool, config: &Config) -> Result<(), SyncCommandError> {
        let mut client = SyncClient::from_config(&config.sync)?;

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
            println!("  FIT_SYNC_URL");
            println!("  FIT_SYNC_API_KEY");
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
        let mut client = SyncClient::from_config(&config.sync)?;
        match client.sync_document(DocType::Dishes).await {
            Ok(_) => println!("✓ connected"),
            Err(SyncClientError::SyncError(_)) => println!("✗ unreachable"),
            Err(e) => println!("✗ error: {}", e),
        }

        Ok(())
    }

    async fn rebuild(&self, pool: &SqlitePool) -> Result<(), SyncCommandError> {
        let storage = DocumentStorage::new();

        println!("Rebuilding automerge documents from SQLite...");
        println!();

        // Delete existing automerge files
        for doc_type in [DocType::Dishes, DocType::MealPlans, DocType::MealLogs] {
            let path = storage.path(doc_type);
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| {
                    SyncCommandError::RebuildError(format!(
                        "Failed to delete {}: {}",
                        path.display(),
                        e
                    ))
                })?;
                println!("  Deleted old {}", doc_type_name(doc_type));
            }
        }

        // Rebuild dishes document
        let dish_repo = DishRepository::new(pool.clone());
        let dishes = dish_repo
            .list()
            .await
            .map_err(|e| SyncCommandError::RebuildError(format!("Failed to list dishes: {}", e)))?;

        let mut dishes_doc = AutoCommit::new();
        for dish in &dishes {
            write_dish(&mut dishes_doc, dish);
        }
        storage
            .save(DocType::Dishes, &mut dishes_doc)
            .map_err(|e| {
                SyncCommandError::RebuildError(format!("Failed to save dishes doc: {}", e))
            })?;
        println!("  ✓ Rebuilt dishes ({} items)", dishes.len());

        // Rebuild mealplans document
        let mealplan_repo = MealPlanRepository::new(pool.clone());
        let mealplans = mealplan_repo.list().await.map_err(|e| {
            SyncCommandError::RebuildError(format!("Failed to list mealplans: {}", e))
        })?;

        let mut mealplans_doc = AutoCommit::new();
        for mealplan in &mealplans {
            write_mealplan(&mut mealplans_doc, mealplan);
        }
        storage
            .save(DocType::MealPlans, &mut mealplans_doc)
            .map_err(|e| {
                SyncCommandError::RebuildError(format!("Failed to save mealplans doc: {}", e))
            })?;
        println!("  ✓ Rebuilt mealplans ({} items)", mealplans.len());

        // Rebuild meallogs document
        let meallog_repo = MealLogRepository::new(pool.clone());
        let meallogs = meallog_repo.list().await.map_err(|e| {
            SyncCommandError::RebuildError(format!("Failed to list meallogs: {}", e))
        })?;

        let mut meallogs_doc = AutoCommit::new();
        for meallog in &meallogs {
            write_meallog(&mut meallogs_doc, meallog);
        }
        storage
            .save(DocType::MealLogs, &mut meallogs_doc)
            .map_err(|e| {
                SyncCommandError::RebuildError(format!("Failed to save meallogs doc: {}", e))
            })?;
        println!("  ✓ Rebuilt meallogs ({} items)", meallogs.len());

        println!();
        println!("Rebuild complete. Run 'fit sync' to sync with server.");

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
    RebuildError(String),
}

impl std::fmt::Display for SyncCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncCommandError::SyncError(e) => write!(f, "{}", e),
            SyncCommandError::RebuildError(e) => write!(f, "Rebuild error: {}", e),
        }
    }
}

impl std::error::Error for SyncCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SyncCommandError::SyncError(e) => Some(e),
            SyncCommandError::RebuildError(_) => None,
        }
    }
}

impl From<SyncClientError> for SyncCommandError {
    fn from(e: SyncClientError) -> Self {
        SyncCommandError::SyncError(e)
    }
}
