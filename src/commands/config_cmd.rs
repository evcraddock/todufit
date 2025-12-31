use clap::{Args, Subcommand, ValueEnum};

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

                        println!(
                            "database_path: {}",
                            config.database_path.value.display()
                        );
                        println!("  source: {}", config.database_path.source);
                        println!();

                        println!("created_by: {}", config.created_by.value);
                        println!("  source: {}", config.created_by.source);
                    }
                }
                Ok(())
            }
        }
    }
}
