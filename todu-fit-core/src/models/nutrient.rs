use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Nutrient {
    pub name: String,
    pub amount: f64,
    pub unit: String,
}

impl Nutrient {
    pub fn new(name: impl Into<String>, amount: f64, unit: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            amount,
            unit: unit.into(),
        }
    }
}

impl fmt::Display for Nutrient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} {}", self.name, self.amount, self.unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nutrient_new() {
        let nutrient = Nutrient::new("calories", 250.0, "kcal");
        assert_eq!(nutrient.name, "calories");
        assert_eq!(nutrient.amount, 250.0);
        assert_eq!(nutrient.unit, "kcal");
    }

    #[test]
    fn test_nutrient_display() {
        let nutrient = Nutrient::new("protein", 15.5, "g");
        assert_eq!(format!("{}", nutrient), "protein: 15.5 g");
    }

    #[test]
    fn test_nutrient_json_roundtrip() {
        let nutrient = Nutrient::new("carbs", 30.0, "g");
        let json = serde_json::to_string(&nutrient).unwrap();
        let parsed: Nutrient = serde_json::from_str(&json).unwrap();
        assert_eq!(nutrient, parsed);
    }
}
