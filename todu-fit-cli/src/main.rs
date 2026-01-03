use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod db;
mod models;
mod sync;

use commands::{
    meal::MealRepos, AuthCommand, ConfigCommand, DishCommand, MealCommand, MealPlanCommand,
    SyncCommand,
};
use config::Config;
use db::{init_db, DishRepository};
use sync::{SyncClient, SyncDishRepository, SyncMealLogRepository, SyncMealPlanRepository};

#[derive(Parser)]
#[command(name = "fit")]
#[command(version)]
#[command(about = "A fitness tracking CLI application", long_about = None)]
struct Cli {
    /// Path to config file
    #[arg(long, short, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with the sync server
    Auth(AuthCommand),

    /// Manage dishes (recipes)
    Dish(DishCommand),

    /// Log and track meals
    Meal(MealCommand),

    /// Manage meal plans
    Mealplan(MealPlanCommand),

    /// Manage configuration
    Config(ConfigCommand),

    /// Sync with remote server
    Sync(SyncCommand),
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load configuration
    let config = Config::load(cli.config)?;

    // Track if we should auto-sync after the command
    let mut should_auto_sync = false;
    let mut pool_for_sync = None;

    match cli.command {
        Some(Commands::Auth(cmd)) => {
            cmd.run(&config).await?;
        }
        Some(Commands::Dish(cmd)) => {
            let pool = init_db(Some(config.database_path.value.clone())).await?;
            // Sync before command if auto_sync enabled
            if config.sync.auto_sync && config.sync.is_configured() {
                auto_sync(&pool, &config).await;
            }
            let repo = SyncDishRepository::new(pool.clone());
            cmd.run(&repo, &config).await?;
            should_auto_sync = true;
            pool_for_sync = Some(pool);
        }
        Some(Commands::Meal(cmd)) => {
            let pool = init_db(Some(config.database_path.value.clone())).await?;
            // Sync before command if auto_sync enabled
            if config.sync.auto_sync && config.sync.is_configured() {
                auto_sync(&pool, &config).await;
            }
            let meallog_repo = SyncMealLogRepository::new(pool.clone());
            let mealplan_repo = SyncMealPlanRepository::new(pool.clone());
            let dish_repo = DishRepository::new(pool.clone());
            let repos = MealRepos {
                meallog: &meallog_repo,
                mealplan: &mealplan_repo,
                dish: &dish_repo,
            };
            cmd.run(repos, &config).await?;
            should_auto_sync = true;
            pool_for_sync = Some(pool);
        }
        Some(Commands::Mealplan(cmd)) => {
            let pool = init_db(Some(config.database_path.value.clone())).await?;
            // Sync before command if auto_sync enabled
            if config.sync.auto_sync && config.sync.is_configured() {
                auto_sync(&pool, &config).await;
            }
            let mealplan_repo = SyncMealPlanRepository::new(pool.clone());
            let dish_repo = DishRepository::new(pool.clone());
            cmd.run(&mealplan_repo, &dish_repo, &config).await?;
            should_auto_sync = true;
            pool_for_sync = Some(pool);
        }
        Some(Commands::Config(cmd)) => {
            cmd.run(&config)?;
        }
        Some(Commands::Sync(cmd)) => {
            let pool = init_db(Some(config.database_path.value.clone())).await?;
            cmd.run(&pool, &config).await?;
        }
        None => {
            println!("Use --help to see available commands");
        }
    }

    // Auto-sync if enabled and we ran a data command
    if should_auto_sync && config.sync.auto_sync && config.sync.is_configured() {
        if let Some(pool) = pool_for_sync {
            auto_sync(&pool, &config).await;
        }
    }

    Ok(())
}

/// Perform auto-sync in background, failing silently
async fn auto_sync(pool: &sqlx::SqlitePool, config: &Config) {
    match SyncClient::from_config(&config.sync) {
        Ok(client) => {
            match client.sync_and_project(pool).await {
                Ok(_) => {
                    // Sync succeeded silently
                }
                Err(_) => {
                    // Sync failed silently - server might be unreachable
                }
            }
        }
        Err(_) => {
            // Config error - shouldn't happen since we checked is_configured
        }
    }
}
