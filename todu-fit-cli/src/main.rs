use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod models;
mod sync;

use commands::{
    meal::MealRepos, ConfigCommand, DeviceCommand, DishCommand, GroupCommand, InitCommand,
    MealCommand, MealPlanCommand, SyncCommand,
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
    /// Initialize identity (new or join existing)
    Init(InitCommand),

    /// Show device identity for sharing
    Device(DeviceCommand),

    /// Manage groups for shared dishes and meal plans
    Group(GroupCommand),

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
        Some(Commands::Init(cmd)) => {
            cmd.run(&config)?;
        }
        Some(Commands::Device(cmd)) => {
            cmd.run(&config)?;
        }
        Some(Commands::Group(cmd)) => {
            cmd.run(&config)?;
        }
        Some(Commands::Dish(cmd)) => {
            let repo = SyncDishRepository::new(config.data_dir.value.clone());
            cmd.run(&repo, &config)?;
        }
        Some(Commands::Meal(cmd)) => {
            let data_dir = config.data_dir.value.clone();
            let meallog_repo = SyncMealLogRepository::new(data_dir.clone());
            let mealplan_repo = SyncMealPlanRepository::new(data_dir.clone());
            let dish_repo = SyncDishRepository::new(data_dir);
            let repos = MealRepos {
                meallog: &meallog_repo,
                mealplan: &mealplan_repo,
                dish: &dish_repo,
            };
            cmd.run(repos, &config)?;
        }
        Some(Commands::Mealplan(cmd)) => {
            let data_dir = config.data_dir.value.clone();
            let mealplan_repo = SyncMealPlanRepository::new(data_dir.clone());
            let dish_repo = SyncDishRepository::new(data_dir);
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
