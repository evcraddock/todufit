use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ingredient {
    pub name: String,
    pub quantity: f64,
    pub unit: String,
}

impl Ingredient {
    pub fn new(name: impl Into<String>, quantity: f64, unit: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            quantity,
            unit: unit.into(),
        }
    }
}

impl fmt::Display for Ingredient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.unit.is_empty() {
            write!(f, "{} {}", self.quantity, self.name)
        } else {
            write!(f, "{} {} {}", self.quantity, self.unit, self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingredient_new() {
        let ingredient = Ingredient::new("flour", 2.5, "cups");
        assert_eq!(ingredient.name, "flour");
        assert_eq!(ingredient.quantity, 2.5);
        assert_eq!(ingredient.unit, "cups");
    }

    #[test]
    fn test_ingredient_display() {
        let ingredient = Ingredient::new("flour", 2.5, "cups");
        assert_eq!(format!("{}", ingredient), "2.5 cups flour");
    }

    #[test]
    fn test_ingredient_display_no_unit() {
        let ingredient = Ingredient::new("eggs", 3.0, "");
        assert_eq!(format!("{}", ingredient), "3 eggs");
    }

    #[test]
    fn test_ingredient_json_roundtrip() {
        let ingredient = Ingredient::new("sugar", 1.0, "tbsp");
        let json = serde_json::to_string(&ingredient).unwrap();
        let parsed: Ingredient = serde_json::from_str(&json).unwrap();
        assert_eq!(ingredient, parsed);
    }
}
