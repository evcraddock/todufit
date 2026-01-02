//! Todu Fit Core Library
//!
//! Shared types and logic for Todu Fit applications.

pub mod models;

pub use models::{Dish, Ingredient, MealLog, MealPlan, MealType, Nutrient};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
