//! Writers for serializing entities into Automerge documents.
//!
//! Re-exports from todu-fit-core.

// Re-export core writer functions
pub use todu_fit_core::automerge::{
    delete_dish, delete_meallog, delete_mealplan, write_dish, write_meallog, write_mealplan,
};

use automerge::{transaction::Transactable, AutoCommit, ObjId, ReadDoc};
use uuid::Uuid;

/// Adds a dish ID to a mealplan's dishes list.
pub fn add_dish_to_mealplan(doc: &mut AutoCommit, mealplan_id: &ObjId, dish_id: Uuid) {
    // Get current dishes list
    if let Ok(Some((_, dishes_obj))) = doc.get(mealplan_id, "dishes") {
        let len = doc.length(&dishes_obj);
        doc.insert(&dishes_obj, len, dish_id.to_string().as_str())
            .unwrap();
    }
}

/// Removes a dish ID from a mealplan's dishes list.
pub fn remove_dish_from_mealplan(doc: &mut AutoCommit, mealplan_id: &ObjId, dish_id: Uuid) {
    if let Ok(Some((_, dishes_obj))) = doc.get(mealplan_id, "dishes") {
        let dish_id_str = dish_id.to_string();
        let len = doc.length(&dishes_obj);

        // Find and remove the dish ID
        for i in (0..len).rev() {
            if let Ok(Some((val, _))) = doc.get(&dishes_obj, i) {
                if let Ok(s) = val.into_string() {
                    if s == dish_id_str {
                        let _ = doc.delete(&dishes_obj, i);
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Dish, Ingredient, MealLog, MealPlan, MealType, Nutrient};
    use automerge::{ReadDoc, ROOT};
    use chrono::NaiveDate;

    #[test]
    fn test_write_dish_basic() {
        let mut doc = AutoCommit::new();
        let dish = Dish::new("Test Pasta", "chef");

        write_dish(&mut doc, &dish);

        // Verify dish exists
        let id_str = dish.id.to_string();
        assert!(doc.get(ROOT, &id_str).unwrap().is_some());
    }

    #[test]
    fn test_write_dish_with_ingredients() {
        let mut doc = AutoCommit::new();
        let dish = Dish::new("Pasta", "chef").with_ingredients(vec![
            Ingredient::new("pasta", 200.0, "g"),
            Ingredient::new("sauce", 1.0, "cup"),
        ]);

        write_dish(&mut doc, &dish);

        // Verify ingredients exist
        let id_str = dish.id.to_string();
        let (_, dish_obj) = doc.get(ROOT, &id_str).unwrap().unwrap();
        let (_, ingredients_obj) = doc.get(&dish_obj, "ingredients").unwrap().unwrap();
        assert_eq!(doc.length(&ingredients_obj), 2);
    }

    #[test]
    fn test_write_dish_with_nutrients() {
        let mut doc = AutoCommit::new();
        let dish = Dish::new("Healthy Meal", "chef").with_nutrients(vec![
            Nutrient::new("calories", 500.0, "kcal"),
            Nutrient::new("protein", 30.0, "g"),
        ]);

        write_dish(&mut doc, &dish);

        let id_str = dish.id.to_string();
        let (_, dish_obj) = doc.get(ROOT, &id_str).unwrap().unwrap();
        let (_, nutrients_obj) = doc.get(&dish_obj, "nutrients").unwrap().unwrap();
        assert_eq!(doc.length(&nutrients_obj), 2);
    }

    #[test]
    fn test_delete_dish() {
        let mut doc = AutoCommit::new();
        let dish = Dish::new("To Delete", "chef");
        let id = dish.id;

        write_dish(&mut doc, &dish);
        assert!(doc.get(ROOT, &id.to_string()).unwrap().is_some());

        delete_dish(&mut doc, id);
        assert!(doc.get(ROOT, &id.to_string()).unwrap().is_none());
    }

    #[test]
    fn test_write_mealplan() {
        let mut doc = AutoCommit::new();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let mealplan = MealPlan::new(date, MealType::Dinner, "Sunday Dinner", "chef");

        write_mealplan(&mut doc, &mealplan);

        let id_str = mealplan.id.to_string();
        assert!(doc.get(ROOT, &id_str).unwrap().is_some());
    }

    #[test]
    fn test_write_meallog() {
        let mut doc = AutoCommit::new();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let meallog = MealLog::new(date, MealType::Lunch, "chef").with_notes("Delicious!");

        write_meallog(&mut doc, &meallog);

        let id_str = meallog.id.to_string();
        assert!(doc.get(ROOT, &id_str).unwrap().is_some());
    }

    #[test]
    fn test_write_meallog_with_mealplan_id() {
        let mut doc = AutoCommit::new();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let mealplan_id = Uuid::new_v4();
        let meallog = MealLog::new(date, MealType::Lunch, "chef").with_mealplan_id(mealplan_id);

        write_meallog(&mut doc, &meallog);

        let id_str = meallog.id.to_string();
        let (_, log_obj) = doc.get(ROOT, &id_str).unwrap().unwrap();
        let (val, _) = doc.get(&log_obj, "mealplan_id").unwrap().unwrap();
        assert_eq!(val.into_string().unwrap(), mealplan_id.to_string());
    }
}
