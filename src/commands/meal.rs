use clap::{Args, Subcommand};
use uuid::Uuid;

use crate::config::Config;
use crate::db::{MealLogRepository, MealPlanRepository};
use crate::models::MealLog;

#[derive(Args)]
pub struct MealCommand {
    #[command(subcommand)]
    pub command: MealSubcommand,
}

#[derive(Subcommand)]
pub enum MealSubcommand {
    /// Log a meal from an existing meal plan
    Log {
        /// Meal plan ID (UUID)
        mealplan_id: String,

        /// Add notes to the log
        #[arg(long)]
        notes: Option<String>,
    },
}

impl MealCommand {
    pub async fn run(
        &self,
        meallog_repo: &MealLogRepository,
        mealplan_repo: &MealPlanRepository,
        config: &Config,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &self.command {
            MealSubcommand::Log { mealplan_id, notes } => {
                // Parse mealplan UUID
                let plan_uuid = Uuid::parse_str(mealplan_id)
                    .map_err(|_| format!("Invalid mealplan UUID: {}", mealplan_id))?;

                // Get the meal plan
                let plan = mealplan_repo
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

                let created = meallog_repo.create(&log).await?;

                println!("Logged meal from plan '{}':", plan.title);
                println!();
                println!("  Date: {}", created.date);
                println!("  Meal: {}", created.meal_type);
                if !created.dishes.is_empty() {
                    println!("  Dishes:");
                    for dish in &created.dishes {
                        println!("    - {}", dish.name);
                    }
                }
                if let Some(n) = &created.notes {
                    println!("  Notes: {}", n);
                }
                println!();
                println!("Log ID: {}", created.id);

                Ok(())
            }
        }
    }
}
