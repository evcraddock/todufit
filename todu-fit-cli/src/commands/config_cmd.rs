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
    pub fn run(&self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
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
                let config_path = Config::default_config_path();

                // Check if config already exists
                if config_path.exists() {
                    println!("Config file already exists: {}", config_path.display());
                    println!("Use 'todufit config show' to view current configuration.");
                    return Ok(());
                }

                // Create parent directory
                if let Some(parent) = config_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Write default config
                let default_config = r#"# todufit configuration

# Path to SQLite database (default: ~/.config/todufit/todufit.db)
# database_path: ~/.config/todufit/todufit.db

# Default user name for new dishes
created_by: default
"#;

                let mut file = fs::File::create(&config_path)?;
                file.write_all(default_config.as_bytes())?;

                println!("Created config file: {}", config_path.display());
                println!("\nEdit this file to customize your settings.");
                Ok(())
            }
        }
    }
}
