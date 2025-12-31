use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod db;
mod models;

use commands::{ConfigCommand, DishCommand, MealPlanCommand};
use config::Config;
use db::{init_db, DishRepository, MealPlanRepository};

#[derive(Parser)]
#[command(name = "todufit")]
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
    /// Manage dishes (recipes)
    Dish(DishCommand),

    /// Manage meal plans
    Mealplan(MealPlanCommand),

    /// Manage configuration
    Config(ConfigCommand),
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

    match cli.command {
        Some(Commands::Dish(cmd)) => {
            let pool = init_db(Some(config.database_path.value.clone())).await?;
            let repo = DishRepository::new(pool);
            cmd.run(&repo, &config).await?;
        }
        Some(Commands::Mealplan(cmd)) => {
            let pool = init_db(Some(config.database_path.value.clone())).await?;
            let mealplan_repo = MealPlanRepository::new(pool.clone());
            let dish_repo = DishRepository::new(pool);
            cmd.run(&mealplan_repo, &dish_repo, &config).await?;
        }
        Some(Commands::Config(cmd)) => {
            cmd.run(&config)?;
        }
        None => {
            println!("Use --help to see available commands");
        }
    }

    Ok(())
}
