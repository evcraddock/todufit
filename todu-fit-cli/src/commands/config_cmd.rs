use clap::{Args, Subcommand, ValueEnum};
use std::fs;
use std::io::Write;

use crate::config::Config;

#[derive(Clone, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Subcommand)]
pub enum ConfigSubcommand {
    /// Show current configuration values
    Show {
        /// Output format
        #[arg(long, short, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Initialize configuration file
    Init,
}

impl ConfigCommand {
    pub fn run(
        &self,
        config: &Config,
        cli_config_path: Option<std::path::PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &self.command {
            ConfigSubcommand::Show { format } => {
                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(config)?);
                    }
                    OutputFormat::Text => {
                        println!("Configuration");
                        println!("=============\n");

                        if let Some(path) = &config.config_file {
                            println!("Config file: {}", path.display());
                        } else {
                            println!(
                                "Config file: {} (not found)",
                                Config::default_config_path().display()
                            );
                        }
                        println!();

                        println!("database_path: {}", config.database_path.value.display());
                        println!("  source: {}", config.database_path.source);
                        println!();

                        println!("created_by: {}", config.created_by.value);
                        println!("  source: {}", config.created_by.source);
                    }
                }
                Ok(())
            }

            ConfigSubcommand::Init => {
                // Use CLI path if provided, otherwise use loaded config path or default
                let config_path = cli_config_path
                    .or_else(|| config.config_file.clone())
                    .unwrap_or_else(Config::default_config_path);

                // Check if config already exists
                if config_path.exists() {
                    println!("Config file already exists: {}", config_path.display());
                    println!("Use 'fit config show' to view current configuration.");
                    return Ok(());
                }

                // Create parent directory
                if let Some(parent) = config_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Get absolute database path
                let db_path = std::fs::canonicalize(&config.database_path.value)
                    .unwrap_or_else(|_| config.database_path.value.clone());

                // Write config with absolute database path
                let config_content = format!(
                    r#"# fit configuration

# Path to SQLite database
database_path: {}

# Default user name for new dishes
created_by: {}

# Sync configuration (uncomment and fill in to enable)
# sync:
#   server_url: ws://localhost:8080
#   api_key: your-api-key-here
#   auto_sync: false
"#,
                    db_path.display(),
                    config.created_by.value
                );

                let mut file = fs::File::create(&config_path)?;
                file.write_all(config_content.as_bytes())?;

                println!("Created config file: {}", config_path.display());
                println!("\nConfiguration:");
                println!("  database_path: {}", db_path.display());
                println!("  created_by: {}", config.created_by.value);
                Ok(())
            }
        }
    }
}
