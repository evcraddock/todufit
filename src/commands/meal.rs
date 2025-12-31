use chrono::{Local, NaiveDate};
use clap::{Args, Subcommand, ValueEnum};
use uuid::Uuid;

use crate::config::Config;
use crate::db::{DishRepository, MealLogRepository, MealPlanRepository};
use crate::models::{MealLog, MealType};

#[derive(Clone, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Repositories needed for meal commands
pub struct MealRepos<'a> {
    pub meallog: &'a MealLogRepository,
    pub mealplan: &'a MealPlanRepository,
    pub dish: &'a DishRepository,
}

#[derive(Args)]
pub struct MealCommand {
    #[command(subcommand)]
    pub command: MealSubcommand,
}

#[derive(Subcommand)]
pub enum MealSubcommand {
    /// Log a meal (from plan or unplanned)
    Log {
        /// Meal plan ID (UUID) - if provided, copies from plan
        mealplan_id: Option<String>,

        /// Date (YYYY-MM-DD) - required for unplanned meals
        #[arg(long, short)]
        date: Option<String>,

        /// Meal type (breakfast, lunch, dinner, snack) - required for unplanned meals
        #[arg(long = "type", short = 't', value_name = "TYPE")]
        meal_type: Option<String>,

        /// Add dish by ID or name (can be repeated) - for unplanned meals
        #[arg(long = "dish", value_name = "DISH")]
        dishes: Vec<String>,

        /// Add notes to the log
        #[arg(long)]
        notes: Option<String>,
    },

    /// View meal history
    History {
        /// Output format
        #[arg(long, short, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Start date (YYYY-MM-DD), defaults to 7 days ago
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD), defaults to today
        #[arg(long)]
        to: Option<String>,
    },
}

impl MealCommand {
    pub async fn run(
        &self,
        repos: MealRepos<'_>,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &self.command {
            MealSubcommand::Log {
                mealplan_id,
                date,
                meal_type,
                dishes,
                notes,
            } => {
                // Determine if this is a planned or unplanned meal
                if let Some(plan_id) = mealplan_id {
                    // Logging from a plan
                    self.log_from_plan(plan_id, notes, &repos, config).await
                } else {
                    // Unplanned meal - require date and type
                    self.log_unplanned(date, meal_type, dishes, notes, &repos, config)
                        .await
                }
            }
            MealSubcommand::History { format, from, to } => {
                self.show_history(format, from, to, &repos).await
            }
        }
    }

    async fn log_from_plan(
        &self,
        mealplan_id: &str,
        notes: &Option<String>,
        repos: &MealRepos<'_>,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Parse mealplan UUID
        let plan_uuid = Uuid::parse_str(mealplan_id)
            .map_err(|_| format!("Invalid mealplan UUID: {}", mealplan_id))?;

        // Get the meal plan
        let plan = repos
            .mealplan
            .get_by_id(plan_uuid)
            .await?
            .ok_or_else(|| format!("Meal plan not found: {}", mealplan_id))?;

        // Create meal log from plan
        let mut log = MealLog::new(plan.date, plan.meal_type, &config.created_by.value)
            .with_mealplan_id(plan.id)
            .with_dishes(plan.dishes.clone());

        if let Some(n) = notes {
            log = log.with_notes(n);
        }

        let created = repos.meallog.create(&log).await?;

        println!("Logged meal from plan '{}':", plan.title);
        println!();
        print_log_details(&created);

        Ok(())
    }

    async fn log_unplanned(
        &self,
        date: &Option<String>,
        meal_type: &Option<String>,
        dishes: &[String],
        notes: &Option<String>,
        repos: &MealRepos<'_>,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Validate required fields
        let date_str = date
            .as_ref()
            .ok_or("--date is required for unplanned meals")?;
        let meal_type_str = meal_type
            .as_ref()
            .ok_or("--type is required for unplanned meals")?;

        // Parse date
        let parsed_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| format!("Invalid date format '{}'. Use YYYY-MM-DD.", date_str))?;

        // Parse meal type
        let parsed_meal_type: MealType = meal_type_str.parse().map_err(|e: String| e)?;

        // Resolve dishes
        let mut resolved_dishes = Vec::new();
        for dish_ref in dishes {
            let dish = if let Ok(uuid) = Uuid::parse_str(dish_ref) {
                repos.dish.get_by_id(uuid).await?
            } else {
                repos.dish.get_by_name(dish_ref).await?
            };

            match dish {
                Some(d) => resolved_dishes.push(d),
                None => return Err(format!("Dish not found: {}", dish_ref).into()),
            }
        }

        // Create meal log
        let mut log = MealLog::new(parsed_date, parsed_meal_type, &config.created_by.value)
            .with_dishes(resolved_dishes);

        if let Some(n) = notes {
            log = log.with_notes(n);
        }

        let created = repos.meallog.create(&log).await?;

        println!("Logged unplanned meal:");
        println!();
        print_log_details(&created);

        Ok(())
    }

    async fn show_history(
        &self,
        format: &OutputFormat,
        from: &Option<String>,
        to: &Option<String>,
        repos: &MealRepos<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Parse date range
        let today = Local::now().date_naive();
        let to_date = match to {
            Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|_| format!("Invalid date format '{}'. Use YYYY-MM-DD.", d))?,
            None => today,
        };
        let from_date = match from {
            Some(d) => NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|_| format!("Invalid date format '{}'. Use YYYY-MM-DD.", d))?,
            None => to_date - chrono::Duration::days(7),
        };

        // Fetch meal logs
        let logs = repos.meallog.list_range(from_date, to_date).await?;

        if logs.is_empty() {
            println!("No meal history found for {} to {}", from_date, to_date);
            return Ok(());
        }

        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&logs)?);
            }
            OutputFormat::Text => {
                let mut current_date: Option<NaiveDate> = None;

                for log in &logs {
                    // Print date header when it changes
                    if current_date != Some(log.date) {
                        if current_date.is_some() {
                            println!();
                        }
                        println!("{}", log.date);
                        println!("{}", "-".repeat(10));
                        current_date = Some(log.date);
                    }

                    // Determine if planned or unplanned
                    let plan_indicator = if log.mealplan_id.is_some() {
                        "(planned)"
                    } else {
                        "(unplanned)"
                    };

                    // Build dish summary
                    let dishes_str = if log.dishes.is_empty() {
                        String::new()
                    } else {
                        let names: Vec<&str> = log.dishes.iter().map(|d| d.name.as_str()).collect();
                        format!(": {}", names.join(", "))
                    };

                    println!("  {:10} {}{}", log.meal_type, plan_indicator, dishes_str);

                    if let Some(notes) = &log.notes {
                        println!("             Notes: {}", notes);
                    }
                }

                println!("\nTotal: {} meal(s)", logs.len());
            }
        }

        Ok(())
    }
}

fn print_log_details(log: &MealLog) {
    println!("  Date: {}", log.date);
    println!("  Meal: {}", log.meal_type);
    if !log.dishes.is_empty() {
        println!("  Dishes:");
        for dish in &log.dishes {
            println!("    - {}", dish.name);
        }
    }
    if let Some(n) = &log.notes {
        println!("  Notes: {}", n);
    }
    println!();
    println!("Log ID: {}", log.id);
}
