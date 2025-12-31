use chrono::NaiveDate;
use clap::{Args, Subcommand, ValueEnum};
use uuid::Uuid;

use crate::config::Config;
use crate::db::{DishRepository, MealPlanRepository};
use crate::models::{MealPlan, MealType};

#[derive(Clone, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Args)]
pub struct MealPlanCommand {
    #[command(subcommand)]
    pub command: MealPlanSubcommand,
}

#[derive(Subcommand)]
pub enum MealPlanSubcommand {
    /// Create a new meal plan
    Create {
        /// Date (YYYY-MM-DD)
        #[arg(long, short)]
        date: String,

        /// Meal type (breakfast, lunch, dinner, snack)
        #[arg(long = "type", short = 't', value_name = "TYPE")]
        meal_type: String,

        /// Title (defaults to "<MealType> on <date>")
        #[arg(long)]
        title: Option<String>,

        /// Cook name
        #[arg(long)]
        cook: Option<String>,

        /// Add dish by ID or name (can be repeated)
        #[arg(long = "dish", value_name = "DISH")]
        dishes: Vec<String>,
    },
}

impl MealPlanCommand {
    pub async fn run(
        &self,
        mealplan_repo: &MealPlanRepository,
        dish_repo: &DishRepository,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &self.command {
            MealPlanSubcommand::Create {
                date,
                meal_type,
                title,
                cook,
                dishes,
            } => {
                // Parse date
                let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .map_err(|_| format!("Invalid date format '{}'. Use YYYY-MM-DD.", date))?;

                // Parse meal type
                let meal_type: MealType = meal_type.parse().map_err(|e: String| e)?;

                // Build title
                let title = title.clone().unwrap_or_else(|| {
                    format!("{} on {}", capitalize(&meal_type.to_string()), date)
                });

                // Build cook
                let cook = cook
                    .clone()
                    .unwrap_or_else(|| config.created_by.value.clone());

                // Create meal plan
                let mut plan = MealPlan::new(date, meal_type, &title, &config.created_by.value)
                    .with_cook(&cook);

                // Resolve and add dishes
                let mut resolved_dishes = Vec::new();
                for dish_ref in dishes {
                    let dish = if let Ok(uuid) = Uuid::parse_str(dish_ref) {
                        dish_repo.get_by_id(uuid).await?
                    } else {
                        dish_repo.get_by_name(dish_ref).await?
                    };

                    match dish {
                        Some(d) => resolved_dishes.push(d),
                        None => return Err(format!("Dish not found: {}", dish_ref).into()),
                    }
                }
                plan.dishes = resolved_dishes;

                let created = mealplan_repo.create(&plan).await?;
                println!("Created meal plan:");
                println!("{}", created);
                Ok(())
            }
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
