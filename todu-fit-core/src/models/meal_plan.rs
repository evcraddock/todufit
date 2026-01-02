use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::dish::Dish;
use super::meal_type::MealType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealPlan {
    pub id: Uuid,
    pub date: NaiveDate,
    pub meal_type: MealType,
    pub title: String,
    pub cook: String,
    pub dishes: Vec<Dish>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MealPlan {
    pub fn new(
        date: NaiveDate,
        meal_type: MealType,
        title: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            date,
            meal_type,
            title: title.into(),
            cook: "Unknown".to_string(),
            dishes: Vec::new(),
            created_by: created_by.into(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_cook(mut self, cook: impl Into<String>) -> Self {
        self.cook = cook.into();
        self
    }

    pub fn with_dishes(mut self, dishes: Vec<Dish>) -> Self {
        self.dishes = dishes;
        self
    }
}

impl fmt::Display for MealPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.title)?;
        writeln!(f, "{}", "=".repeat(self.title.len()))?;
        writeln!(f, "Date: {}", self.date)?;
        writeln!(f, "Meal: {}", self.meal_type)?;
        writeln!(f, "Cook: {}", self.cook)?;

        if !self.dishes.is_empty() {
            writeln!(f, "\nDishes:")?;
            for dish in &self.dishes {
                writeln!(f, "  - {}", dish.name)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meal_plan_new() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "New Year Dinner", "user1");

        assert_eq!(plan.date, date);
        assert_eq!(plan.meal_type, MealType::Dinner);
        assert_eq!(plan.title, "New Year Dinner");
        assert_eq!(plan.cook, "Unknown");
        assert!(plan.dishes.is_empty());
    }

    #[test]
    fn test_meal_plan_with_cook() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Lunch, "Lunch", "user1").with_cook("Chef Bob");

        assert_eq!(plan.cook, "Chef Bob");
    }

    #[test]
    fn test_meal_plan_display() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Breakfast, "Morning Meal", "user1");

        let output = format!("{}", plan);
        assert!(output.contains("Morning Meal"));
        assert!(output.contains("2025-01-01"));
        assert!(output.contains("breakfast"));
    }

    #[test]
    fn test_meal_plan_json_roundtrip() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let plan = MealPlan::new(date, MealType::Dinner, "Test Dinner", "user1");

        let json = serde_json::to_string(&plan).unwrap();
        let parsed: MealPlan = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.title, plan.title);
        assert_eq!(parsed.meal_type, plan.meal_type);
        assert_eq!(parsed.date, plan.date);
    }
}
