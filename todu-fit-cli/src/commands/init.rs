//! Initialize identity for multi-user support.

use clap::Args;

use todu_fit_core::{DocumentId, Identity, IdentityState, MultiDocStorage};

use crate::config::Config;

/// Initialize a new identity or join an existing one
#[derive(Args)]
pub struct InitCommand {
    /// Create a new identity
    #[arg(long, conflicts_with = "join")]
    new: bool,

    /// Join an existing identity by document ID
    #[arg(long, conflicts_with = "new", value_name = "DOC_ID")]
    join: Option<String>,
}

impl InitCommand {
    pub fn run(&self, _config: &Config) -> Result<(), InitError> {
        let storage = MultiDocStorage::new(Config::default_data_dir());
        let identity = Identity::new(storage);

        match identity.state() {
            IdentityState::Initialized => {
                let root_id = identity.root_doc_id()?.unwrap();
                println!("Identity already initialized.");
                println!();
                println!("Identity ID: {}", root_id.to_bs58check());
                println!();
                println!("To share this identity with another device, use:");
                println!("  fit device show");
                return Ok(());
            }
            IdentityState::PendingSync => {
                let root_id = identity.root_doc_id()?.unwrap();
                println!("Identity is pending sync.");
                println!();
                println!("Identity ID: {}", root_id.to_bs58check());
                println!();
                println!("Run 'fit sync' to complete the join.");
                return Ok(());
            }
            IdentityState::Uninitialized => {
                // Continue with initialization
            }
        }

        if self.new {
            self.create_new(&identity)
        } else if let Some(doc_id_str) = &self.join {
            self.join_existing(&identity, doc_id_str)
        } else {
            println!("Initialize your identity:");
            println!();
            println!("  fit init --new         Create a new identity");
            println!("  fit init --join <ID>   Join an existing identity");
            println!();
            println!("If you're setting up a new account, use --new.");
            println!("If you're adding a device to an existing account, get the");
            println!("identity ID from your other device with 'fit device show'.");
            Ok(())
        }
    }

    fn create_new(&self, identity: &Identity) -> Result<(), InitError> {
        let root_id = identity.initialize_new()?;

        println!("✓ Identity created successfully!");
        println!();
        println!("Identity ID: {}", root_id.to_bs58check());
        println!();
        println!("Next steps:");
        println!("  1. Create a group:  fit group create <name>");
        println!("  2. Add dishes:      fit dish create <name>");
        println!("  3. Plan meals:      fit mealplan create --date YYYY-MM-DD --type dinner");
        println!();
        println!(
            "To add another device, run 'fit init --join {}' on that device.",
            root_id.to_bs58check()
        );

        Ok(())
    }

    fn join_existing(&self, identity: &Identity, doc_id_str: &str) -> Result<(), InitError> {
        let doc_id = DocumentId::from_bs58check(doc_id_str)
            .map_err(|e| InitError::InvalidDocId(doc_id_str.to_string(), e.to_string()))?;

        identity.initialize_join(doc_id)?;

        println!("✓ Identity joined successfully!");
        println!();
        println!("Identity ID: {}", doc_id.to_bs58check());
        println!();
        println!("Status: Pending sync");
        println!();
        println!("Next step: Run 'fit sync' to fetch your data from the sync server.");

        Ok(())
    }
}

/// Errors from init command
#[derive(Debug)]
pub enum InitError {
    IdentityError(todu_fit_core::IdentityError),
    InvalidDocId(String, String),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::IdentityError(e) => write!(f, "{}", e),
            InitError::InvalidDocId(id, e) => write!(f, "Invalid document ID '{}': {}", id, e),
        }
    }
}

impl std::error::Error for InitError {}

impl From<todu_fit_core::IdentityError> for InitError {
    fn from(e: todu_fit_core::IdentityError) -> Self {
        InitError::IdentityError(e)
    }
}
