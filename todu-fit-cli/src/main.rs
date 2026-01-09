use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod models;
mod sync;

use commands::{
    meal::MealRepos, AuthCommand, ConfigCommand, DishCommand, MealCommand, MealPlanCommand,
    SyncCommand,
};
use config::Config;
use sync::{SyncDishRepository, SyncMealLogRepository, SyncMealPlanRepository};

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

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Save config path for init command
    let cli_config_path = cli.config.clone();

    // Load configuration
    let config = Config::load(cli.config)?;

    match cli.command {
        Some(Commands::Auth(cmd)) => {
            cmd.run(&config)?;
        }
        Some(Commands::Dish(cmd)) => {
            let repo = SyncDishRepository::new();
            cmd.run(&repo, &config)?;
        }
        Some(Commands::Meal(cmd)) => {
            let meallog_repo = SyncMealLogRepository::new();
            let mealplan_repo = SyncMealPlanRepository::new();
            let dish_repo = SyncDishRepository::new();
            let repos = MealRepos {
                meallog: &meallog_repo,
                mealplan: &mealplan_repo,
                dish: &dish_repo,
            };
            cmd.run(repos, &config)?;
        }
        Some(Commands::Mealplan(cmd)) => {
            let mealplan_repo = SyncMealPlanRepository::new();
            let dish_repo = SyncDishRepository::new();
            cmd.run(&mealplan_repo, &dish_repo, &config)?;
        }
        Some(Commands::Config(cmd)) => {
            cmd.run(&config, cli_config_path)?;
        }
        Some(Commands::Sync(cmd)) => {
            cmd.run(&config)?;
        }
        None => {
            println!("Use --help to see available commands");
        }
    }

    Ok(())
}
