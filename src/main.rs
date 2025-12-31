use clap::{Parser, Subcommand};
use std::path::PathBuf;

use todufit::commands::{
    meal::MealRepos, ConfigCommand, DishCommand, MealCommand, MealPlanCommand,
};
use todufit::config::Config;
use todufit::db::{init_db, DishRepository};
use todufit::sync::{SyncDishRepository, SyncMealLogRepository, SyncMealPlanRepository};

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

    /// Log and track meals
    Meal(MealCommand),

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
            let repo = SyncDishRepository::new(pool);
            cmd.run(&repo, &config).await?;
        }
        Some(Commands::Meal(cmd)) => {
            let pool = init_db(Some(config.database_path.value.clone())).await?;
            let meallog_repo = SyncMealLogRepository::new(pool.clone());
            let mealplan_repo = SyncMealPlanRepository::new(pool.clone());
            let dish_repo = DishRepository::new(pool);
            let repos = MealRepos {
                meallog: &meallog_repo,
                mealplan: &mealplan_repo,
                dish: &dish_repo,
            };
            cmd.run(repos, &config).await?;
        }
        Some(Commands::Mealplan(cmd)) => {
            let pool = init_db(Some(config.database_path.value.clone())).await?;
            let mealplan_repo = SyncMealPlanRepository::new(pool.clone());
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
