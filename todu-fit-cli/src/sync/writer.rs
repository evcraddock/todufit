//! Writers for serializing entities into Automerge documents.
//!
//! These functions handle converting Rust structs into Automerge document structure.

use automerge::{transaction::Transactable, AutoCommit, ObjId, ObjType, ReadDoc, ROOT};
use uuid::Uuid;

use crate::models::{Dish, MealLog, MealPlan};

/// Writes a dish to an Automerge document.
///
/// The dish is stored at root[dish.id.to_string()].
pub fn write_dish(doc: &mut AutoCommit, dish: &Dish) {
    let id_str = dish.id.to_string();

    // Create or overwrite the dish object
    let dish_id = doc
        .put_object(ROOT, &id_str, ObjType::Map)
        .expect("Failed to create dish object");

    doc.put(&dish_id, "name", dish.name.as_str()).unwrap();
    doc.put(&dish_id, "instructions", dish.instructions.as_str())
        .unwrap();
    doc.put(&dish_id, "created_by", dish.created_by.as_str())
        .unwrap();
    doc.put(
        &dish_id,
        "created_at",
        dish.created_at.to_rfc3339().as_str(),
    )
    .unwrap();
    doc.put(
        &dish_id,
        "updated_at",
        dish.updated_at.to_rfc3339().as_str(),
    )
    .unwrap();

    // Optional fields
    if let Some(prep_time) = dish.prep_time {
        doc.put(&dish_id, "prep_time", prep_time as i64).unwrap();
    }
    if let Some(cook_time) = dish.cook_time {
        doc.put(&dish_id, "cook_time", cook_time as i64).unwrap();
    }
    if let Some(servings) = dish.servings {
        doc.put(&dish_id, "servings", servings as i64).unwrap();
    }
    if let Some(ref url) = dish.image_url {
        doc.put(&dish_id, "image_url", url.as_str()).unwrap();
    }
    if let Some(ref url) = dish.source_url {
        doc.put(&dish_id, "source_url", url.as_str()).unwrap();
    }

    // Tags
    let tags_id = doc.put_object(&dish_id, "tags", ObjType::List).unwrap();
    for (i, tag) in dish.tags.iter().enumerate() {
        doc.insert(&tags_id, i, tag.as_str()).unwrap();
    }

    // Ingredients
    let ingredients_id = doc
        .put_object(&dish_id, "ingredients", ObjType::List)
        .unwrap();
    for (i, ingredient) in dish.ingredients.iter().enumerate() {
        let ing_id = doc.insert_object(&ingredients_id, i, ObjType::Map).unwrap();
        doc.put(&ing_id, "name", ingredient.name.as_str()).unwrap();
        doc.put(&ing_id, "quantity", ingredient.quantity).unwrap();
        doc.put(&ing_id, "unit", ingredient.unit.as_str()).unwrap();
    }

    // Nutrients
    if let Some(ref nutrients) = dish.nutrients {
        let nutrients_id = doc
            .put_object(&dish_id, "nutrients", ObjType::List)
            .unwrap();
        for (i, nutrient) in nutrients.iter().enumerate() {
            let nut_id = doc.insert_object(&nutrients_id, i, ObjType::Map).unwrap();
            doc.put(&nut_id, "name", nutrient.name.as_str()).unwrap();
            doc.put(&nut_id, "amount", nutrient.amount).unwrap();
            doc.put(&nut_id, "unit", nutrient.unit.as_str()).unwrap();
        }
    }
}

/// Deletes a dish from an Automerge document.
pub fn delete_dish(doc: &mut AutoCommit, id: Uuid) {
    let id_str = id.to_string();
    let _ = doc.delete(ROOT, &id_str);
}

/// Writes a meal plan to an Automerge document.
///
/// The meal plan is stored at root[mealplan.id.to_string()].
/// Dishes are stored as a list of UUIDs (references, not embedded).
pub fn write_mealplan(doc: &mut AutoCommit, mealplan: &MealPlan) {
    let id_str = mealplan.id.to_string();

    let plan_id = doc
        .put_object(ROOT, &id_str, ObjType::Map)
        .expect("Failed to create mealplan object");

    doc.put(&plan_id, "date", mealplan.date.to_string().as_str())
        .unwrap();
    doc.put(
        &plan_id,
        "meal_type",
        mealplan.meal_type.to_string().as_str(),
    )
    .unwrap();
    doc.put(&plan_id, "title", mealplan.title.as_str()).unwrap();
    doc.put(&plan_id, "cook", mealplan.cook.as_str()).unwrap();
    doc.put(&plan_id, "created_by", mealplan.created_by.as_str())
        .unwrap();
    doc.put(
        &plan_id,
        "created_at",
        mealplan.created_at.to_rfc3339().as_str(),
    )
    .unwrap();
    doc.put(
        &plan_id,
        "updated_at",
        mealplan.updated_at.to_rfc3339().as_str(),
    )
    .unwrap();

    // Dishes as list of UUIDs
    let dishes_id = doc.put_object(&plan_id, "dishes", ObjType::List).unwrap();
    for (i, dish) in mealplan.dishes.iter().enumerate() {
        doc.insert(&dishes_id, i, dish.id.to_string().as_str())
            .unwrap();
    }
}

/// Deletes a meal plan from an Automerge document.
pub fn delete_mealplan(doc: &mut AutoCommit, id: Uuid) {
    let id_str = id.to_string();
    let _ = doc.delete(ROOT, &id_str);
}

/// Writes a meal log to an Automerge document.
///
/// The meal log is stored at root[meallog.id.to_string()].
/// Dishes are stored as a list of UUIDs (references, not embedded).
pub fn write_meallog(doc: &mut AutoCommit, meallog: &MealLog) {
    let id_str = meallog.id.to_string();

    let log_id = doc
        .put_object(ROOT, &id_str, ObjType::Map)
        .expect("Failed to create meallog object");

    doc.put(&log_id, "date", meallog.date.to_string().as_str())
        .unwrap();
    doc.put(&log_id, "meal_type", meallog.meal_type.to_string().as_str())
        .unwrap();
    doc.put(&log_id, "created_by", meallog.created_by.as_str())
        .unwrap();
    doc.put(
        &log_id,
        "created_at",
        meallog.created_at.to_rfc3339().as_str(),
    )
    .unwrap();

    if let Some(ref plan_id) = meallog.mealplan_id {
        doc.put(&log_id, "mealplan_id", plan_id.to_string().as_str())
            .unwrap();
    }

    if let Some(ref notes) = meallog.notes {
        doc.put(&log_id, "notes", notes.as_str()).unwrap();
    }

    // Dishes as list of UUIDs
    let dishes_id = doc.put_object(&log_id, "dishes", ObjType::List).unwrap();
    for (i, dish) in meallog.dishes.iter().enumerate() {
        doc.insert(&dishes_id, i, dish.id.to_string().as_str())
            .unwrap();
    }
}

/// Deletes a meal log from an Automerge document.
pub fn delete_meallog(doc: &mut AutoCommit, id: Uuid) {
    let id_str = id.to_string();
    let _ = doc.delete(ROOT, &id_str);
}

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
    use automerge::ReadDoc;

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
    use crate::models::{Ingredient, MealType, Nutrient};
    use automerge::ReadDoc;
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
