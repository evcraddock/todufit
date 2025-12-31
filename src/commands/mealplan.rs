use chrono::{Local, NaiveDate};
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

    /// List meal plans
    List {
        /// Output format
        #[arg(long, short, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Start date (YYYY-MM-DD), defaults to today
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD), defaults to 7 days from start
        #[arg(long)]
        to: Option<String>,

        /// Filter by meal type
        #[arg(long = "type", short = 't', value_name = "TYPE")]
        meal_type: Option<String>,
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

            MealPlanSubcommand::List {
                format,
                from,
                to,
                meal_type,
            } => {
                // Parse date range
                let today = Local::now().date_naive();
                let from_date = match from {
                    Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d")
                        .map_err(|_| format!("Invalid date format '{}'. Use YYYY-MM-DD.", d))?,
                    None => today,
                };
                let to_date = match to {
                    Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d")
                        .map_err(|_| format!("Invalid date format '{}'. Use YYYY-MM-DD.", d))?,
                    None => from_date + chrono::Duration::days(7),
                };

                // Parse meal type filter
                let meal_type_filter: Option<MealType> = match meal_type {
                    Some(mt) => Some(mt.parse().map_err(|e: String| e)?),
                    None => None,
                };

                // Fetch meal plans
                let mut plans = mealplan_repo.list_range(from_date, to_date).await?;

                // Apply meal type filter
                if let Some(mt) = meal_type_filter {
                    plans.retain(|p| p.meal_type == mt);
                }

                if plans.is_empty() {
                    println!("No meal plans found");
                    return Ok(());
                }

                match format {
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&plans)?);
                    }
                    OutputFormat::Text => {
                        let mut current_date: Option<NaiveDate> = None;
                        for plan in &plans {
                            if current_date != Some(plan.date) {
                                if current_date.is_some() {
                                    println!();
                                }
                                println!("{}", plan.date);
                                println!("{}", "-".repeat(10));
                                current_date = Some(plan.date);
                            }
                            let dish_count = plan.dishes.len();
                            let dishes_str = if dish_count == 0 {
                                "no dishes".to_string()
                            } else if dish_count == 1 {
                                "1 dish".to_string()
                            } else {
                                format!("{} dishes", dish_count)
                            };
                            println!("  {:10} {} ({})", plan.meal_type, plan.title, dishes_str);
                        }
                        println!("\nTotal: {} meal plan(s)", plans.len());
                    }
                }
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
