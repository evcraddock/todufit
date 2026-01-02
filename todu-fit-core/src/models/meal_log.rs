use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::dish::Dish;
use super::meal_type::MealType;

/// A meal log represents what was actually eaten (vs MealPlan which is planned)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealLog {
    pub id: Uuid,
    pub date: NaiveDate,
    pub meal_type: MealType,
    pub mealplan_id: Option<Uuid>,
    pub dishes: Vec<Dish>,
    pub notes: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl MealLog {
    pub fn new(date: NaiveDate, meal_type: MealType, created_by: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            date,
            meal_type,
            mealplan_id: None,
            dishes: Vec::new(),
            notes: None,
            created_by: created_by.into(),
            created_at: Utc::now(),
        }
    }

    pub fn with_mealplan_id(mut self, mealplan_id: Uuid) -> Self {
        self.mealplan_id = Some(mealplan_id);
        self
    }

    pub fn with_dishes(mut self, dishes: Vec<Dish>) -> Self {
        self.dishes = dishes;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

impl fmt::Display for MealLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Meal Log: {} - {}", self.date, self.meal_type)?;
        writeln!(f, "{}", "=".repeat(30))?;

        if !self.dishes.is_empty() {
            writeln!(f, "Dishes:")?;
            for dish in &self.dishes {
                writeln!(f, "  - {}", dish.name)?;
            }
        }

        if let Some(notes) = &self.notes {
            writeln!(f, "\nNotes: {}", notes)?;
        }

        if let Some(plan_id) = &self.mealplan_id {
            writeln!(f, "From meal plan: {}", plan_id)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meal_log_new() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let log = MealLog::new(date, MealType::Dinner, "user1");

        assert_eq!(log.date, date);
        assert_eq!(log.meal_type, MealType::Dinner);
        assert!(log.mealplan_id.is_none());
        assert!(log.dishes.is_empty());
        assert!(log.notes.is_none());
        assert_eq!(log.created_by, "user1");
    }

    #[test]
    fn test_meal_log_with_mealplan() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let mealplan_id = Uuid::new_v4();
        let log = MealLog::new(date, MealType::Lunch, "user1").with_mealplan_id(mealplan_id);

        assert_eq!(log.mealplan_id, Some(mealplan_id));
    }

    #[test]
    fn test_meal_log_with_notes() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let log =
            MealLog::new(date, MealType::Breakfast, "user1").with_notes("Ate at a restaurant");

        assert_eq!(log.notes, Some("Ate at a restaurant".to_string()));
    }

    #[test]
    fn test_meal_log_display() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let log = MealLog::new(date, MealType::Dinner, "user1").with_notes("Delicious!");

        let output = format!("{}", log);
        assert!(output.contains("2025-01-01"));
        assert!(output.contains("dinner"));
        assert!(output.contains("Delicious!"));
    }

    #[test]
    fn test_meal_log_json_roundtrip() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let log = MealLog::new(date, MealType::Dinner, "user1").with_notes("Test note");

        let json = serde_json::to_string(&log).unwrap();
        let parsed: MealLog = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.date, log.date);
        assert_eq!(parsed.meal_type, log.meal_type);
        assert_eq!(parsed.notes, log.notes);
    }
}
