//! ToduFit Admin CLI
//!
//! Administration tool for managing users on the sync server.
//!
//! # Usage
//!
//! ```bash
//! todufit-admin user add erik@example.com --group family --name Erik
//! todufit-admin user list
//! todufit-admin user remove erik@example.com
//! ```
//!
//! # Environment Variables
//!
//! - `TODUFIT_DATA_DIR`: Directory where server stores data (default: ~/.local/share/todufit-server)

use automerge::transaction::Transactable;
use automerge::{AutoCommit, ObjType, ReadDoc, ROOT};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

// ============================================================================
// CLI Structure
// ============================================================================

#[derive(Parser)]
#[command(name = "todufit-admin")]
#[command(version)]
#[command(about = "ToduFit server administration tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage users
    User(UserCommand),
}

#[derive(Args)]
struct UserCommand {
    #[command(subcommand)]
    command: UserSubcommand,
}

#[derive(Subcommand)]
enum UserSubcommand {
    /// Add a new user
    Add {
        /// User's email address
        email: String,
        /// Group ID for data access
        #[arg(long, short)]
        group: String,
        /// User's display name
        #[arg(long, short)]
        name: Option<String>,
    },
    /// List all users
    List,
    /// Remove a user
    Remove {
        /// User's email address
        email: String,
    },
}

// ============================================================================
// Storage
// ============================================================================

/// Get the data directory for the server
fn data_dir() -> PathBuf {
    std::env::var("TODUFIT_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("todufit-server")
        })
}

/// Path to users.automerge
fn users_path() -> PathBuf {
    data_dir().join("users.automerge")
}

/// Load or create the users document
fn load_users() -> Result<AutoCommit, Box<dyn std::error::Error>> {
    let path = users_path();

    if path.exists() {
        let bytes = std::fs::read(&path)?;
        let doc = AutoCommit::load(&bytes)?;
        Ok(doc)
    } else {
        Ok(AutoCommit::new())
    }
}

/// Save the users document
fn save_users(doc: &mut AutoCommit) -> Result<(), Box<dyn std::error::Error>> {
    let path = users_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let bytes = doc.save();
    std::fs::write(&path, bytes)?;
    Ok(())
}

// ============================================================================
// Commands
// ============================================================================

fn add_user(
    email: String,
    group: String,
    name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = load_users()?;

    // Check if user already exists
    if doc.get(ROOT, &email)?.is_some() {
        eprintln!("Error: User '{}' already exists", email);
        std::process::exit(1);
    }

    // Create user object
    let user_obj = doc.put_object(ROOT, &email, ObjType::Map)?;
    doc.put(&user_obj, "group_id", group.clone())?;
    doc.put(&user_obj, "created_at", Utc::now().to_rfc3339())?;

    if let Some(n) = &name {
        doc.put(&user_obj, "name", n.clone())?;
    }

    save_users(&mut doc)?;

    println!("Added user: {}", email);
    println!("  Group: {}", group);
    if let Some(n) = name {
        println!("  Name: {}", n);
    }

    Ok(())
}

fn list_users() -> Result<(), Box<dyn std::error::Error>> {
    let doc = load_users()?;

    let keys: Vec<_> = doc.keys(ROOT).collect();

    if keys.is_empty() {
        println!("No users registered.");
        return Ok(());
    }

    println!("{:<40} {:<20} {:<20}", "EMAIL", "GROUP", "NAME");
    println!("{}", "-".repeat(80));

    for email in &keys {
        if let Some((_, user_obj)) = doc.get(ROOT, email.as_str())? {
            let group_id = doc
                .get(&user_obj, "group_id")?
                .and_then(|(v, _)| v.into_string().ok())
                .unwrap_or_default();

            let name = doc
                .get(&user_obj, "name")?
                .and_then(|(v, _)| v.into_string().ok())
                .unwrap_or_default();

            println!("{:<40} {:<20} {:<20}", email, group_id, name);
        }
    }

    println!();
    println!("Total: {} user(s)", keys.len());

    Ok(())
}

fn remove_user(email: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = load_users()?;

    // Check if user exists
    if doc.get(ROOT, &email)?.is_none() {
        eprintln!("Error: User '{}' not found", email);
        std::process::exit(1);
    }

    doc.delete(ROOT, &email)?;
    save_users(&mut doc)?;

    println!("Removed user: {}", email);

    Ok(())
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::User(user_cmd) => match user_cmd.command {
            UserSubcommand::Add { email, group, name } => add_user(email, group, name),
            UserSubcommand::List => list_users(),
            UserSubcommand::Remove { email } => remove_user(email),
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
