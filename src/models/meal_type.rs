use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MealType {
    Breakfast,
    Lunch,
    Dinner,
    Snack,
}

impl fmt::Display for MealType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MealType::Breakfast => write!(f, "breakfast"),
            MealType::Lunch => write!(f, "lunch"),
            MealType::Dinner => write!(f, "dinner"),
            MealType::Snack => write!(f, "snack"),
        }
    }
}

impl FromStr for MealType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "breakfast" => Ok(MealType::Breakfast),
            "lunch" => Ok(MealType::Lunch),
            "dinner" => Ok(MealType::Dinner),
            "snack" => Ok(MealType::Snack),
            _ => Err(format!(
                "Invalid meal type '{}'. Valid options: breakfast, lunch, dinner, snack",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meal_type_display() {
        assert_eq!(format!("{}", MealType::Breakfast), "breakfast");
        assert_eq!(format!("{}", MealType::Lunch), "lunch");
        assert_eq!(format!("{}", MealType::Dinner), "dinner");
        assert_eq!(format!("{}", MealType::Snack), "snack");
    }

    #[test]
    fn test_meal_type_from_str() {
        assert_eq!(
            MealType::from_str("breakfast").unwrap(),
            MealType::Breakfast
        );
        assert_eq!(MealType::from_str("LUNCH").unwrap(), MealType::Lunch);
        assert_eq!(MealType::from_str("Dinner").unwrap(), MealType::Dinner);
        assert_eq!(MealType::from_str("snack").unwrap(), MealType::Snack);
    }

    #[test]
    fn test_meal_type_from_str_invalid() {
        assert!(MealType::from_str("brunch").is_err());
        assert!(MealType::from_str("").is_err());
    }

    #[test]
    fn test_meal_type_json_roundtrip() {
        let meal_type = MealType::Breakfast;
        let json = serde_json::to_string(&meal_type).unwrap();
        assert_eq!(json, "\"breakfast\"");

        let parsed: MealType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, meal_type);
    }
}
