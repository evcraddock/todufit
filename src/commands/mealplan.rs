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

    /// Show meal plan details
    Show {
        /// Meal plan ID (UUID) or date (YYYY-MM-DD)
        identifier: String,

        /// Output format
        #[arg(long, short, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Meal type (required if date has multiple plans)
        #[arg(long = "type", short = 't', value_name = "TYPE")]
        meal_type: Option<String>,
    },

    /// Update a meal plan
    Update {
        /// Meal plan ID (UUID)
        id: String,

        /// New date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,

        /// New meal type
        #[arg(long = "type", short = 't', value_name = "TYPE")]
        meal_type: Option<String>,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New cook
        #[arg(long)]
        cook: Option<String>,
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

            MealPlanSubcommand::Show {
                identifier,
                format,
                meal_type,
            } => {
                // Try to parse as UUID first
                let plans: Vec<MealPlan> = if let Ok(uuid) = Uuid::parse_str(identifier) {
                    match mealplan_repo.get_by_id(uuid).await? {
                        Some(plan) => vec![plan],
                        None => vec![],
                    }
                } else if let Ok(date) = NaiveDate::parse_from_str(identifier, "%Y-%m-%d") {
                    // It's a date - get plans for that date
                    let mut plans = mealplan_repo.get_by_date(date).await?;

                    // Filter by meal type if provided
                    if let Some(mt) = meal_type {
                        let meal_type_filter: MealType = mt.parse().map_err(|e: String| e)?;
                        plans.retain(|p| p.meal_type == meal_type_filter);
                    }
                    plans
                } else {
                    return Err(format!(
                        "Invalid identifier '{}'. Use UUID or date (YYYY-MM-DD).",
                        identifier
                    )
                    .into());
                };

                if plans.is_empty() {
                    return Err(format!("Meal plan not found: {}", identifier).into());
                }

                match format {
                    OutputFormat::Json => {
                        if plans.len() == 1 {
                            println!("{}", serde_json::to_string_pretty(&plans[0])?);
                        } else {
                            println!("{}", serde_json::to_string_pretty(&plans)?);
                        }
                    }
                    OutputFormat::Text => {
                        for (i, plan) in plans.iter().enumerate() {
                            if i > 0 {
                                println!("\n{}\n", "=".repeat(40));
                            }
                            println!("{}", plan);

                            // Show dish details
                            if !plan.dishes.is_empty() {
                                for dish in &plan.dishes {
                                    println!("\n  {}", dish.name);
                                    println!("  {}", "-".repeat(dish.name.len()));
                                    if !dish.ingredients.is_empty() {
                                        println!("  Ingredients:");
                                        for ing in &dish.ingredients {
                                            println!("    - {}", ing);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(())
            }

            MealPlanSubcommand::Update {
                id,
                date,
                meal_type,
                title,
                cook,
            } => {
                // Check if any updates provided
                let has_updates =
                    date.is_some() || meal_type.is_some() || title.is_some() || cook.is_some();

                if !has_updates {
                    return Err("Nothing to update. Provide at least one option.".into());
                }

                // Parse UUID
                let uuid = Uuid::parse_str(id).map_err(|_| format!("Invalid UUID: {}", id))?;

                // Get existing plan
                let mut plan = mealplan_repo
                    .get_by_id(uuid)
                    .await?
                    .ok_or_else(|| format!("Meal plan not found: {}", id))?;

                // Apply updates
                if let Some(d) = date {
                    plan.date = NaiveDate::parse_from_str(d, "%Y-%m-%d")
                        .map_err(|_| format!("Invalid date format '{}'. Use YYYY-MM-DD.", d))?;
                }
                if let Some(mt) = meal_type {
                    plan.meal_type = mt.parse().map_err(|e: String| e)?;
                }
                if let Some(t) = title {
                    plan.title = t.clone();
                }
                if let Some(c) = cook {
                    plan.cook = c.clone();
                }

                let updated = mealplan_repo.update(&plan).await?;
                println!("Updated meal plan:");
                println!("{}", updated);
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
