use clap::{Parser, Subcommand};

mod commands;
mod db;
mod models;

use commands::DishCommand;
use db::{init_db, DishRepository};

#[derive(Parser)]
#[command(name = "todufit")]
#[command(about = "A fitness tracking CLI application", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage dishes (recipes)
    Dish(DishCommand),
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

    match cli.command {
        Some(Commands::Dish(cmd)) => {
            let pool = init_db(None).await?;
            let repo = DishRepository::new(pool);
            cmd.run(&repo).await?;
        }
        None => {
            println!("Use --help to see available commands");
        }
    }

    Ok(())
}
