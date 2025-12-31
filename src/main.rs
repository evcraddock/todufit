use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;
mod db;
mod models;

use commands::{ConfigCommand, DishCommand};
use config::Config;
use db::{init_db, DishRepository};

#[derive(Parser)]
#[command(name = "todufit")]
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
        Some(Commands::Config(cmd)) => {
            cmd.run(&config)?;
        }
        None => {
            println!("Use --help to see available commands");
        }
    }

    Ok(())
}
