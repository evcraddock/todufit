use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::ingredient::Ingredient;
use super::nutrient::Nutrient;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dish {
    pub id: Uuid,
    pub name: String,
    pub ingredients: Vec<Ingredient>,
    pub instructions: String,
    pub nutrients: Option<Vec<Nutrient>>,
    pub prep_time: Option<i32>, // minutes
    pub cook_time: Option<i32>, // minutes
    pub servings: Option<i32>,
    pub tags: Vec<String>,
    pub image_url: Option<String>,
    pub source_url: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Dish {
    pub fn new(name: impl Into<String>, created_by: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            ingredients: Vec::new(),
            instructions: String::new(),
            nutrients: None,
            prep_time: None,
            cook_time: None,
            servings: None,
            tags: Vec::new(),
            image_url: None,
            source_url: None,
            created_by: created_by.into(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_ingredients(mut self, ingredients: Vec<Ingredient>) -> Self {
        self.ingredients = ingredients;
        self
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = instructions.into();
        self
    }

    pub fn with_nutrients(mut self, nutrients: Vec<Nutrient>) -> Self {
        self.nutrients = Some(nutrients);
        self
    }

    pub fn with_prep_time(mut self, minutes: i32) -> Self {
        self.prep_time = Some(minutes);
        self
    }

    pub fn with_cook_time(mut self, minutes: i32) -> Self {
        self.cook_time = Some(minutes);
        self
    }

    pub fn with_servings(mut self, servings: i32) -> Self {
        self.servings = Some(servings);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn total_time(&self) -> Option<i32> {
        match (self.prep_time, self.cook_time) {
            (Some(prep), Some(cook)) => Some(prep + cook),
            (Some(prep), None) => Some(prep),
            (None, Some(cook)) => Some(cook),
            (None, None) => None,
        }
    }
}

impl fmt::Display for Dish {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.name)?;
        writeln!(f, "{}", "=".repeat(self.name.len()))?;

        if let Some(servings) = self.servings {
            writeln!(f, "Servings: {}", servings)?;
        }

        if let Some(total) = self.total_time() {
            let parts: Vec<String> = [
                self.prep_time.map(|t| format!("prep: {} min", t)),
                self.cook_time.map(|t| format!("cook: {} min", t)),
            ]
            .into_iter()
            .flatten()
            .collect();
            writeln!(f, "Time: {} min ({})", total, parts.join(", "))?;
        }

        if !self.tags.is_empty() {
            writeln!(f, "Tags: {}", self.tags.join(", "))?;
        }

        if !self.ingredients.is_empty() {
            writeln!(f, "\nIngredients:")?;
            for ingredient in &self.ingredients {
                writeln!(f, "  - {}", ingredient)?;
            }
        }

        if !self.instructions.is_empty() {
            writeln!(f, "\nInstructions:\n{}", self.instructions)?;
        }

        if let Some(nutrients) = &self.nutrients {
            if !nutrients.is_empty() {
                writeln!(f, "\nNutrition (per serving):")?;
                for nutrient in nutrients {
                    writeln!(f, "  - {}", nutrient)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dish_new() {
        let dish = Dish::new("Pasta", "user1");
        assert_eq!(dish.name, "Pasta");
        assert_eq!(dish.created_by, "user1");
        assert!(dish.ingredients.is_empty());
        assert!(dish.instructions.is_empty());
    }

    #[test]
    fn test_dish_builder() {
        let dish = Dish::new("Salad", "user1")
            .with_ingredients(vec![
                Ingredient::new("lettuce", 1.0, "head"),
                Ingredient::new("tomato", 2.0, ""),
            ])
            .with_instructions("Mix everything together.")
            .with_prep_time(10)
            .with_servings(2)
            .with_tags(vec!["healthy".into(), "quick".into()]);

        assert_eq!(dish.ingredients.len(), 2);
        assert_eq!(dish.prep_time, Some(10));
        assert_eq!(dish.servings, Some(2));
        assert_eq!(dish.tags.len(), 2);
    }

    #[test]
    fn test_total_time() {
        let dish = Dish::new("Test", "user1")
            .with_prep_time(15)
            .with_cook_time(30);
        assert_eq!(dish.total_time(), Some(45));

        let dish2 = Dish::new("Test2", "user1").with_prep_time(10);
        assert_eq!(dish2.total_time(), Some(10));

        let dish3 = Dish::new("Test3", "user1");
        assert_eq!(dish3.total_time(), None);
    }

    #[test]
    fn test_dish_json_roundtrip() {
        let dish = Dish::new("Soup", "user1")
            .with_ingredients(vec![Ingredient::new("water", 4.0, "cups")])
            .with_nutrients(vec![Nutrient::new("calories", 100.0, "kcal")])
            .with_prep_time(5)
            .with_cook_time(20);

        let json = serde_json::to_string(&dish).unwrap();
        let parsed: Dish = serde_json::from_str(&json).unwrap();
        assert_eq!(dish.name, parsed.name);
        assert_eq!(dish.ingredients, parsed.ingredients);
        assert_eq!(dish.nutrients, parsed.nutrients);
    }

    #[test]
    fn test_dish_display() {
        let dish = Dish::new("Test Dish", "user1")
            .with_ingredients(vec![Ingredient::new("item", 1.0, "unit")])
            .with_servings(4);

        let output = format!("{}", dish);
        assert!(output.contains("Test Dish"));
        assert!(output.contains("Servings: 4"));
        assert!(output.contains("1 unit item"));
    }
}
