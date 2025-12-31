use clap::{Args, Subcommand};

use crate::db::DishRepository;
use crate::models::Dish;

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
        }
    }
}
