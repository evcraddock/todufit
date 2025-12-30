use clap::{Parser, Subcommand};

mod commands;
mod db;
mod models;

#[derive(Parser)]
#[command(name = "todufit")]
#[command(about = "A fitness tracking CLI application", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Placeholder command
    #[command(hide = true)]
    Init,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            println!("Initializing...");
        }
        None => {
            println!("Use --help to see available commands");
        }
    }
}
