use clap::{Args, Subcommand, ValueEnum};
use uuid::Uuid;

use crate::db::DishRepository;
use crate::models::Dish;

#[derive(Clone, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Args)]
pub struct DishCommand {
    #[command(subcommand)]
    pub command: DishSubcommand,
}

#[derive(Subcommand)]
pub enum DishSubcommand {
    /// Create a new dish
    Create {
        /// Name of the dish
        name: String,

        /// Cooking instructions
        #[arg(long)]
        instructions: Option<String>,

        /// Prep time in minutes
        #[arg(long)]
        prep_time: Option<i32>,

        /// Cook time in minutes
        #[arg(long)]
        cook_time: Option<i32>,

        /// Number of servings
        #[arg(long)]
        servings: Option<i32>,

        /// Tags (can be repeated)
        #[arg(long = "tag", value_name = "TAG")]
        tags: Vec<String>,

        /// Image URL
        #[arg(long)]
        image_url: Option<String>,

        /// Recipe source URL
        #[arg(long)]
        source_url: Option<String>,
    },

    /// List all dishes
    List {
        /// Output format
        #[arg(long, short, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Filter by tag
        #[arg(long = "tag", value_name = "TAG")]
        tag: Option<String>,
    },

    /// Show a dish's details
    Show {
        /// Dish ID (UUID) or name
        identifier: String,

        /// Output format
        #[arg(long, short, value_enum, default_value = "text")]
        format: OutputFormat,
    },
}

impl DishCommand {
    pub async fn run(&self, repo: &DishRepository) -> Result<(), Box<dyn std::error::Error>> {
        match &self.command {
            DishSubcommand::Create {
                name,
                instructions,
                prep_time,
                cook_time,
                servings,
                tags,
                image_url,
                source_url,
            } => {
                if name.trim().is_empty() {
                    return Err("Dish name cannot be empty".into());
                }

                let mut dish = Dish::new(name.trim(), "default");

                if let Some(instructions) = instructions {
                    dish = dish.with_instructions(instructions);
                }
                if let Some(prep_time) = prep_time {
                    dish = dish.with_prep_time(*prep_time);
                }
                if let Some(cook_time) = cook_time {
                    dish = dish.with_cook_time(*cook_time);
                }
                if let Some(servings) = servings {
                    dish = dish.with_servings(*servings);
                }
                if !tags.is_empty() {
                    dish = dish.with_tags(tags.clone());
                }
                if image_url.is_some() {
                    dish.image_url = image_url.clone();
                }
                if source_url.is_some() {
                    dish.source_url = source_url.clone();
                }

                let created = repo.create(&dish).await?;
                println!("Created dish:");
                println!("{}", created);
                Ok(())
            }

            DishSubcommand::List { format, tag } => {
                let dishes = repo.list().await?;

                // Filter by tag if specified
                let dishes: Vec<_> = if let Some(tag) = tag {
                    let tag_lower = tag.to_lowercase();
                    dishes
                        .into_iter()
                        .filter(|d| d.tags.iter().any(|t| t.to_lowercase() == tag_lower))
                        .collect()
                } else {
                    dishes
                };

                if dishes.is_empty() {
                    println!("No dishes found");
                    return Ok(());
                }

                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&dishes)?);
                    }
                    OutputFormat::Text => {
                        println!(
                            "{:<36}  {:<30}  {}",
                            "ID", "NAME", "TAGS"
                        );
                        println!("{}", "-".repeat(80));
                        for dish in &dishes {
                            let tags = dish.tags.join(", ");
                            let name = if dish.name.len() > 30 {
                                format!("{}...", &dish.name[..27])
                            } else {
                                dish.name.clone()
                            };
                            println!("{:<36}  {:<30}  {}", dish.id, name, tags);
                        }
                        println!("\nTotal: {} dish(es)", dishes.len());
                    }
                }
                Ok(())
            }

            DishSubcommand::Show { identifier, format } => {
                // Try to parse as UUID first, then fall back to name lookup
                let dish = if let Ok(uuid) = Uuid::parse_str(identifier) {
                    repo.get_by_id(uuid).await?
                } else {
                    repo.get_by_name(identifier).await?
                };

                match dish {
                    Some(dish) => {
                        match format {
                            OutputFormat::Json => {
                                println!("{}", serde_json::to_string_pretty(&dish)?);
                            }
                            OutputFormat::Text => {
                                println!("{}", dish);
                            }
                        }
                        Ok(())
                    }
                    None => Err(format!("Dish not found: {}", identifier).into()),
                }
            }
        }
    }
}
