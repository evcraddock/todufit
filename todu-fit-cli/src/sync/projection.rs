//! Projection layer for syncing Automerge documents to SQLite.
//!
//! Projections read data from Automerge documents and write it to SQLite,
//! making the data queryable through standard SQL operations.

use automerge::{AutoCommit, ObjId, ReadDoc, ROOT};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{Dish, Ingredient, MealLog, MealPlan, MealType, Nutrient};

/// Error type for projection operations.
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum ProjectionError {
    /// Error reading from Automerge document.
    AutomergeError(String),
    /// Error writing to SQLite.
    SqliteError(sqlx::Error),
    /// Error parsing data (e.g., invalid UUID, date format).
    ParseError(String),
}

impl std::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectionError::AutomergeError(e) => write!(f, "Automerge error: {}", e),
            ProjectionError::SqliteError(e) => write!(f, "SQLite error: {}", e),
            ProjectionError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for ProjectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProjectionError::SqliteError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for ProjectionError {
    fn from(e: sqlx::Error) -> Self {
        ProjectionError::SqliteError(e)
    }
}

/// Projects dishes from an Automerge document to SQLite.
pub struct DishProjection;

impl DishProjection {
    /// Projects all dishes from an Automerge document to SQLite.
    ///
    /// This performs a full sync:
    /// 1. Clears all existing dishes from SQLite
    /// 2. Reads all dishes from the Automerge document
    /// 3. Inserts them into SQLite
    ///
    /// The entire operation is wrapped in a transaction for atomicity.
    pub async fn project_all(doc: &AutoCommit, pool: &SqlitePool) -> Result<(), ProjectionError> {
        let dishes = Self::extract_dishes(doc)?;

        let mut tx = pool.begin().await?;

        // Clear existing data (order matters due to foreign keys)
        sqlx::query("DELETE FROM nutrients")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM ingredients")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM dishes").execute(&mut *tx).await?;

        // Insert all dishes
        for dish in dishes {
            Self::insert_dish(&mut tx, &dish).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Extracts all dishes from an Automerge document.
    fn extract_dishes(doc: &AutoCommit) -> Result<Vec<Dish>, ProjectionError> {
        let mut dishes = Vec::new();

        // Iterate over all keys at the root (each key is a dish UUID)
        for key in doc.keys(ROOT) {
            if let Some((value, obj_id)) = doc.get(ROOT, &key).map_err(|e| {
                ProjectionError::AutomergeError(format!("Failed to get key {}: {}", key, e))
            })? {
                // Only process object values (maps)
                if value.is_object() {
                    let dish = Self::extract_dish(doc, &obj_id, &key)?;
                    dishes.push(dish);
                }
            }
        }

        Ok(dishes)
    }

    /// Extracts a single dish from an Automerge object.
    fn extract_dish(
        doc: &AutoCommit,
        obj_id: &ObjId,
        id_str: &str,
    ) -> Result<Dish, ProjectionError> {
        let id = Uuid::parse_str(id_str).map_err(|e| {
            ProjectionError::ParseError(format!("Invalid UUID '{}': {}", id_str, e))
        })?;

        let name = Self::get_string(doc, obj_id, "name")?.unwrap_or_default();
        let instructions = Self::get_string(doc, obj_id, "instructions")?.unwrap_or_default();
        let prep_time = Self::get_i32(doc, obj_id, "prep_time")?;
        let cook_time = Self::get_i32(doc, obj_id, "cook_time")?;
        let servings = Self::get_i32(doc, obj_id, "servings")?;
        let image_url = Self::get_string(doc, obj_id, "image_url")?;
        let source_url = Self::get_string(doc, obj_id, "source_url")?;
        let created_by = Self::get_string(doc, obj_id, "created_by")?.unwrap_or_default();

        let created_at = Self::get_string(doc, obj_id, "created_at")?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let updated_at = Self::get_string(doc, obj_id, "updated_at")?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let tags = Self::get_string_list(doc, obj_id, "tags")?;
        let ingredients = Self::extract_ingredients(doc, obj_id)?;
        let nutrients = Self::extract_nutrients(doc, obj_id)?;

        Ok(Dish {
            id,
            name,
            ingredients,
            instructions,
            nutrients,
            prep_time,
            cook_time,
            servings,
            tags,
            image_url,
            source_url,
            created_by,
            created_at,
            updated_at,
        })
    }

    /// Extracts ingredients from a dish object.
    fn extract_ingredients(
        doc: &AutoCommit,
        dish_obj: &ObjId,
    ) -> Result<Vec<Ingredient>, ProjectionError> {
        let mut ingredients = Vec::new();

        if let Some((value, list_id)) = doc
            .get(dish_obj, "ingredients")
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            if value.is_object() {
                let len = doc.length(&list_id);
                for i in 0..len {
                    if let Some((item_value, item_id)) = doc
                        .get(&list_id, i)
                        .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
                    {
                        if item_value.is_object() {
                            let name = Self::get_string(doc, &item_id, "name")?.unwrap_or_default();
                            let quantity = Self::get_f64(doc, &item_id, "quantity")?.unwrap_or(0.0);
                            let unit = Self::get_string(doc, &item_id, "unit")?.unwrap_or_default();
                            ingredients.push(Ingredient::new(name, quantity, unit));
                        }
                    }
                }
            }
        }

        Ok(ingredients)
    }

    /// Extracts nutrients from a dish object.
    fn extract_nutrients(
        doc: &AutoCommit,
        dish_obj: &ObjId,
    ) -> Result<Option<Vec<Nutrient>>, ProjectionError> {
        if let Some((value, list_id)) = doc
            .get(dish_obj, "nutrients")
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            if value.is_object() {
                let len = doc.length(&list_id);
                if len == 0 {
                    return Ok(None);
                }

                let mut nutrients = Vec::new();
                for i in 0..len {
                    if let Some((item_value, item_id)) = doc
                        .get(&list_id, i)
                        .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
                    {
                        if item_value.is_object() {
                            let name = Self::get_string(doc, &item_id, "name")?.unwrap_or_default();
                            let amount = Self::get_f64(doc, &item_id, "amount")?.unwrap_or(0.0);
                            let unit = Self::get_string(doc, &item_id, "unit")?.unwrap_or_default();
                            nutrients.push(Nutrient::new(name, amount, unit));
                        }
                    }
                }
                return Ok(Some(nutrients));
            }
        }

        Ok(None)
    }

    /// Helper to get a string value from an Automerge object.
    fn get_string(
        doc: &AutoCommit,
        obj_id: &ObjId,
        key: &str,
    ) -> Result<Option<String>, ProjectionError> {
        if let Some((value, _)) = doc
            .get(obj_id, key)
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            Ok(value.into_string().ok())
        } else {
            Ok(None)
        }
    }

    /// Helper to get an i32 value from an Automerge object.
    fn get_i32(
        doc: &AutoCommit,
        obj_id: &ObjId,
        key: &str,
    ) -> Result<Option<i32>, ProjectionError> {
        if let Some((value, _)) = doc
            .get(obj_id, key)
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            Ok(value.to_i64().map(|v| v as i32))
        } else {
            Ok(None)
        }
    }

    /// Helper to get an f64 value from an Automerge object.
    fn get_f64(
        doc: &AutoCommit,
        obj_id: &ObjId,
        key: &str,
    ) -> Result<Option<f64>, ProjectionError> {
        if let Some((value, _)) = doc
            .get(obj_id, key)
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            Ok(value.to_f64())
        } else {
            Ok(None)
        }
    }

    /// Helper to get a string list from an Automerge object.
    fn get_string_list(
        doc: &AutoCommit,
        obj_id: &ObjId,
        key: &str,
    ) -> Result<Vec<String>, ProjectionError> {
        let mut result = Vec::new();

        if let Some((value, list_id)) = doc
            .get(obj_id, key)
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            if value.is_object() {
                let len = doc.length(&list_id);
                for i in 0..len {
                    if let Some((item_value, _)) = doc
                        .get(&list_id, i)
                        .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
                    {
                        if let Ok(s) = item_value.into_string() {
                            result.push(s);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Inserts a dish into SQLite within a transaction.
    async fn insert_dish(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        dish: &Dish,
    ) -> Result<(), ProjectionError> {
        let id = dish.id.to_string();
        let tags = serde_json::to_string(&dish.tags).unwrap_or_else(|_| "[]".to_string());
        let created_at = dish.created_at.to_rfc3339();
        let updated_at = dish.updated_at.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO dishes (id, name, instructions, prep_time, cook_time, servings, tags, image_url, source_url, created_by, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&dish.name)
        .bind(&dish.instructions)
        .bind(dish.prep_time)
        .bind(dish.cook_time)
        .bind(dish.servings)
        .bind(&tags)
        .bind(&dish.image_url)
        .bind(&dish.source_url)
        .bind(&dish.created_by)
        .bind(&created_at)
        .bind(&updated_at)
        .execute(&mut **tx)
        .await?;

        // Insert ingredients
        for ingredient in &dish.ingredients {
            sqlx::query(
                "INSERT INTO ingredients (dish_id, name, quantity, unit) VALUES (?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&ingredient.name)
            .bind(ingredient.quantity)
            .bind(&ingredient.unit)
            .execute(&mut **tx)
            .await?;
        }

        // Insert nutrients
        if let Some(nutrients) = &dish.nutrients {
            for nutrient in nutrients {
                sqlx::query(
                    "INSERT INTO nutrients (dish_id, name, amount, unit) VALUES (?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(&nutrient.name)
                .bind(nutrient.amount)
                .bind(&nutrient.unit)
                .execute(&mut **tx)
                .await?;
            }
        }

        Ok(())
    }
}

/// Projects meal plans from an Automerge document to SQLite.
pub struct MealPlanProjection;

impl MealPlanProjection {
    /// Projects all meal plans from an Automerge document to SQLite.
    ///
    /// This performs a full sync:
    /// 1. Clears all existing meal plans from SQLite
    /// 2. Reads all meal plans from the Automerge document
    /// 3. Inserts them into SQLite
    ///
    /// Note: This only clears mealplans and mealplan_dishes tables, not dishes.
    /// Dishes must be projected separately first.
    pub async fn project_all(doc: &AutoCommit, pool: &SqlitePool) -> Result<(), ProjectionError> {
        let mealplans = Self::extract_mealplans(doc)?;

        let mut tx = pool.begin().await?;

        // Clear existing data
        sqlx::query("DELETE FROM mealplan_dishes")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM mealplans")
            .execute(&mut *tx)
            .await?;

        // Insert all meal plans
        for mealplan in mealplans {
            Self::insert_mealplan(&mut tx, &mealplan).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Extracts all meal plans from an Automerge document.
    fn extract_mealplans(doc: &AutoCommit) -> Result<Vec<MealPlanData>, ProjectionError> {
        let mut mealplans = Vec::new();

        for key in doc.keys(ROOT) {
            if let Some((value, obj_id)) = doc.get(ROOT, &key).map_err(|e| {
                ProjectionError::AutomergeError(format!("Failed to get key {}: {}", key, e))
            })? {
                if value.is_object() {
                    let mealplan = Self::extract_mealplan(doc, &obj_id, &key)?;
                    mealplans.push(mealplan);
                }
            }
        }

        Ok(mealplans)
    }

    /// Extracts a single meal plan from an Automerge object.
    fn extract_mealplan(
        doc: &AutoCommit,
        obj_id: &ObjId,
        id_str: &str,
    ) -> Result<MealPlanData, ProjectionError> {
        let id = Uuid::parse_str(id_str).map_err(|e| {
            ProjectionError::ParseError(format!("Invalid UUID '{}': {}", id_str, e))
        })?;

        let date_str = Self::get_string(doc, obj_id, "date")?.unwrap_or_default();
        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| {
            ProjectionError::ParseError(format!("Invalid date '{}': {}", date_str, e))
        })?;

        let meal_type_str = Self::get_string(doc, obj_id, "meal_type")?.unwrap_or_default();
        let meal_type: MealType = meal_type_str.parse().unwrap_or(MealType::Dinner);

        let title = Self::get_string(doc, obj_id, "title")?.unwrap_or_default();
        let cook = Self::get_string(doc, obj_id, "cook")?.unwrap_or_else(|| "Unknown".to_string());
        let created_by = Self::get_string(doc, obj_id, "created_by")?.unwrap_or_default();

        let created_at = Self::get_string(doc, obj_id, "created_at")?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let updated_at = Self::get_string(doc, obj_id, "updated_at")?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let dish_ids = Self::extract_dish_ids(doc, obj_id)?;

        Ok(MealPlanData {
            mealplan: MealPlan {
                id,
                date,
                meal_type,
                title,
                cook,
                dishes: Vec::new(), // Dishes are referenced by ID in junction table
                created_by,
                created_at,
                updated_at,
            },
            dish_ids,
        })
    }

    /// Extracts dish IDs from a meal plan object.
    fn extract_dish_ids(doc: &AutoCommit, obj_id: &ObjId) -> Result<Vec<Uuid>, ProjectionError> {
        let mut dish_ids = Vec::new();

        if let Some((value, list_id)) = doc
            .get(obj_id, "dishes")
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            if value.is_object() {
                let len = doc.length(&list_id);
                for i in 0..len {
                    if let Some((item_value, _)) = doc
                        .get(&list_id, i)
                        .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
                    {
                        if let Ok(id_str) = item_value.into_string() {
                            if let Ok(id) = Uuid::parse_str(&id_str) {
                                dish_ids.push(id);
                            }
                        }
                    }
                }
            }
        }

        Ok(dish_ids)
    }

    fn get_string(
        doc: &AutoCommit,
        obj_id: &ObjId,
        key: &str,
    ) -> Result<Option<String>, ProjectionError> {
        if let Some((value, _)) = doc
            .get(obj_id, key)
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            Ok(value.into_string().ok())
        } else {
            Ok(None)
        }
    }

    async fn insert_mealplan(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        data: &MealPlanData,
    ) -> Result<(), ProjectionError> {
        let mealplan = &data.mealplan;
        let id = mealplan.id.to_string();
        let date = mealplan.date.to_string();
        let meal_type = mealplan.meal_type.to_string();
        let created_at = mealplan.created_at.to_rfc3339();
        let updated_at = mealplan.updated_at.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO mealplans (id, date, meal_type, title, cook, created_by, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&date)
        .bind(&meal_type)
        .bind(&mealplan.title)
        .bind(&mealplan.cook)
        .bind(&mealplan.created_by)
        .bind(&created_at)
        .bind(&updated_at)
        .execute(&mut **tx)
        .await?;

        // Insert dish associations
        for dish_id in &data.dish_ids {
            sqlx::query(
                "INSERT OR IGNORE INTO mealplan_dishes (mealplan_id, dish_id) VALUES (?, ?)",
            )
            .bind(&id)
            .bind(dish_id.to_string())
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }
}

/// Internal struct to hold meal plan data with dish IDs.
struct MealPlanData {
    mealplan: MealPlan,
    dish_ids: Vec<Uuid>,
}

/// Projects meal logs from an Automerge document to SQLite.
pub struct MealLogProjection;

impl MealLogProjection {
    /// Projects all meal logs from an Automerge document to SQLite.
    ///
    /// This performs a full sync:
    /// 1. Clears all existing meal logs from SQLite
    /// 2. Reads all meal logs from the Automerge document
    /// 3. Inserts them into SQLite
    ///
    /// Note: This only clears meallogs and meallog_dishes tables.
    /// Dishes and meal plans must be projected separately first.
    pub async fn project_all(doc: &AutoCommit, pool: &SqlitePool) -> Result<(), ProjectionError> {
        let meallogs = Self::extract_meallogs(doc)?;

        let mut tx = pool.begin().await?;

        // Clear existing data
        sqlx::query("DELETE FROM meallog_dishes")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM meallogs")
            .execute(&mut *tx)
            .await?;

        // Insert all meal logs
        for meallog in meallogs {
            Self::insert_meallog(&mut tx, &meallog).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Extracts all meal logs from an Automerge document.
    fn extract_meallogs(doc: &AutoCommit) -> Result<Vec<MealLogData>, ProjectionError> {
        let mut meallogs = Vec::new();

        for key in doc.keys(ROOT) {
            if let Some((value, obj_id)) = doc.get(ROOT, &key).map_err(|e| {
                ProjectionError::AutomergeError(format!("Failed to get key {}: {}", key, e))
            })? {
                if value.is_object() {
                    let meallog = Self::extract_meallog(doc, &obj_id, &key)?;
                    meallogs.push(meallog);
                }
            }
        }

        Ok(meallogs)
    }

    /// Extracts a single meal log from an Automerge object.
    fn extract_meallog(
        doc: &AutoCommit,
        obj_id: &ObjId,
        id_str: &str,
    ) -> Result<MealLogData, ProjectionError> {
        let id = Uuid::parse_str(id_str).map_err(|e| {
            ProjectionError::ParseError(format!("Invalid UUID '{}': {}", id_str, e))
        })?;

        let date_str = Self::get_string(doc, obj_id, "date")?.unwrap_or_default();
        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| {
            ProjectionError::ParseError(format!("Invalid date '{}': {}", date_str, e))
        })?;

        let meal_type_str = Self::get_string(doc, obj_id, "meal_type")?.unwrap_or_default();
        let meal_type: MealType = meal_type_str.parse().unwrap_or(MealType::Dinner);

        let mealplan_id =
            Self::get_string(doc, obj_id, "mealplan_id")?.and_then(|s| Uuid::parse_str(&s).ok());

        let notes = Self::get_string(doc, obj_id, "notes")?;
        let created_by = Self::get_string(doc, obj_id, "created_by")?.unwrap_or_default();

        let created_at = Self::get_string(doc, obj_id, "created_at")?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let dish_ids = Self::extract_dish_ids(doc, obj_id)?;

        Ok(MealLogData {
            meallog: MealLog {
                id,
                date,
                meal_type,
                mealplan_id,
                dishes: Vec::new(), // Dishes are referenced by ID in junction table
                notes,
                created_by,
                created_at,
            },
            dish_ids,
        })
    }

    /// Extracts dish IDs from a meal log object.
    fn extract_dish_ids(doc: &AutoCommit, obj_id: &ObjId) -> Result<Vec<Uuid>, ProjectionError> {
        let mut dish_ids = Vec::new();

        if let Some((value, list_id)) = doc
            .get(obj_id, "dishes")
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            if value.is_object() {
                let len = doc.length(&list_id);
                for i in 0..len {
                    if let Some((item_value, _)) = doc
                        .get(&list_id, i)
                        .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
                    {
                        if let Ok(id_str) = item_value.into_string() {
                            if let Ok(id) = Uuid::parse_str(&id_str) {
                                dish_ids.push(id);
                            }
                        }
                    }
                }
            }
        }

        Ok(dish_ids)
    }

    fn get_string(
        doc: &AutoCommit,
        obj_id: &ObjId,
        key: &str,
    ) -> Result<Option<String>, ProjectionError> {
        if let Some((value, _)) = doc
            .get(obj_id, key)
            .map_err(|e| ProjectionError::AutomergeError(e.to_string()))?
        {
            Ok(value.into_string().ok())
        } else {
            Ok(None)
        }
    }

    async fn insert_meallog(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        data: &MealLogData,
    ) -> Result<(), ProjectionError> {
        let meallog = &data.meallog;
        let id = meallog.id.to_string();
        let date = meallog.date.to_string();
        let meal_type = meallog.meal_type.to_string();
        let mealplan_id = meallog.mealplan_id.map(|id| id.to_string());
        let created_at = meallog.created_at.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO meallogs (id, date, meal_type, mealplan_id, notes, created_by, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&date)
        .bind(&meal_type)
        .bind(&mealplan_id)
        .bind(&meallog.notes)
        .bind(&meallog.created_by)
        .bind(&created_at)
        .execute(&mut **tx)
        .await?;

        // Insert dish associations
        for dish_id in &data.dish_ids {
            sqlx::query("INSERT OR IGNORE INTO meallog_dishes (meallog_id, dish_id) VALUES (?, ?)")
                .bind(&id)
                .bind(dish_id.to_string())
                .execute(&mut **tx)
                .await?;
        }

        Ok(())
    }
}

/// Internal struct to hold meal log data with dish IDs.
struct MealLogData {
    meallog: MealLog,
    dish_ids: Vec<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use automerge::transaction::Transactable;
    use automerge::ObjType;
    use tempfile::TempDir;

    async fn setup_db() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        (pool, temp_dir)
    }

    fn create_test_dish_in_doc(doc: &mut AutoCommit, id: &str, name: &str) {
        // Create dish object at root[id]
        let dish_id = doc
            .put_object(ROOT, id, ObjType::Map)
            .expect("Failed to create dish object");

        doc.put(&dish_id, "name", name).unwrap();
        doc.put(&dish_id, "instructions", "Test instructions")
            .unwrap();
        doc.put(&dish_id, "created_by", "testuser").unwrap();
        doc.put(&dish_id, "created_at", "2025-01-01T00:00:00Z")
            .unwrap();
        doc.put(&dish_id, "updated_at", "2025-01-01T00:00:00Z")
            .unwrap();

        // Add empty ingredients list
        doc.put_object(&dish_id, "ingredients", ObjType::List)
            .unwrap();

        // Add tags
        let tags_id = doc.put_object(&dish_id, "tags", ObjType::List).unwrap();
        doc.insert(&tags_id, 0, "test-tag").unwrap();
    }

    fn create_dish_with_ingredients(doc: &mut AutoCommit, id: &str, name: &str) {
        let dish_id = doc
            .put_object(ROOT, id, ObjType::Map)
            .expect("Failed to create dish object");

        doc.put(&dish_id, "name", name).unwrap();
        doc.put(&dish_id, "instructions", "Cook it").unwrap();
        doc.put(&dish_id, "created_by", "chef").unwrap();
        doc.put(&dish_id, "created_at", "2025-01-01T00:00:00Z")
            .unwrap();
        doc.put(&dish_id, "updated_at", "2025-01-01T00:00:00Z")
            .unwrap();
        doc.put(&dish_id, "prep_time", 10_i64).unwrap();
        doc.put(&dish_id, "cook_time", 20_i64).unwrap();
        doc.put(&dish_id, "servings", 4_i64).unwrap();

        // Add ingredients
        let ingredients_id = doc
            .put_object(&dish_id, "ingredients", ObjType::List)
            .unwrap();

        let ing1 = doc.insert_object(&ingredients_id, 0, ObjType::Map).unwrap();
        doc.put(&ing1, "name", "flour").unwrap();
        doc.put(&ing1, "quantity", 2.0_f64).unwrap();
        doc.put(&ing1, "unit", "cups").unwrap();

        let ing2 = doc.insert_object(&ingredients_id, 1, ObjType::Map).unwrap();
        doc.put(&ing2, "name", "sugar").unwrap();
        doc.put(&ing2, "quantity", 0.5_f64).unwrap();
        doc.put(&ing2, "unit", "cups").unwrap();

        // Add nutrients
        let nutrients_id = doc
            .put_object(&dish_id, "nutrients", ObjType::List)
            .unwrap();

        let nut1 = doc.insert_object(&nutrients_id, 0, ObjType::Map).unwrap();
        doc.put(&nut1, "name", "calories").unwrap();
        doc.put(&nut1, "amount", 250.0_f64).unwrap();
        doc.put(&nut1, "unit", "kcal").unwrap();

        // Add tags
        let tags_id = doc.put_object(&dish_id, "tags", ObjType::List).unwrap();
        doc.insert(&tags_id, 0, "baking").unwrap();
        doc.insert(&tags_id, 1, "dessert").unwrap();
    }

    #[tokio::test]
    async fn test_project_all_empty_doc() {
        let (pool, _temp) = setup_db().await;
        let doc = AutoCommit::new();

        DishProjection::project_all(&doc, &pool).await.unwrap();

        // Verify no dishes in DB
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dishes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn test_project_all_single_dish() {
        let (pool, _temp) = setup_db().await;
        let mut doc = AutoCommit::new();

        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        create_test_dish_in_doc(&mut doc, uuid, "Test Dish");

        DishProjection::project_all(&doc, &pool).await.unwrap();

        // Verify dish in DB
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dishes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1);

        let name: (String,) = sqlx::query_as("SELECT name FROM dishes WHERE id = ?")
            .bind(uuid)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name.0, "Test Dish");
    }

    #[tokio::test]
    async fn test_project_all_with_ingredients_and_nutrients() {
        let (pool, _temp) = setup_db().await;
        let mut doc = AutoCommit::new();

        let uuid = "550e8400-e29b-41d4-a716-446655440001";
        create_dish_with_ingredients(&mut doc, uuid, "Cake");

        DishProjection::project_all(&doc, &pool).await.unwrap();

        // Verify ingredients
        let ingredients: Vec<(String, f64, String)> =
            sqlx::query_as("SELECT name, quantity, unit FROM ingredients WHERE dish_id = ?")
                .bind(uuid)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(ingredients.len(), 2);
        assert_eq!(ingredients[0].0, "flour");
        assert_eq!(ingredients[0].1, 2.0);
        assert_eq!(ingredients[0].2, "cups");

        // Verify nutrients
        let nutrients: Vec<(String, f64, String)> =
            sqlx::query_as("SELECT name, amount, unit FROM nutrients WHERE dish_id = ?")
                .bind(uuid)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(nutrients.len(), 1);
        assert_eq!(nutrients[0].0, "calories");
        assert_eq!(nutrients[0].1, 250.0);
    }

    #[tokio::test]
    async fn test_project_all_replaces_existing_data() {
        let (pool, _temp) = setup_db().await;

        // First projection with dish A
        let mut doc1 = AutoCommit::new();
        create_test_dish_in_doc(&mut doc1, "550e8400-e29b-41d4-a716-446655440002", "Dish A");
        DishProjection::project_all(&doc1, &pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dishes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1);

        // Second projection with dish B (should replace dish A)
        let mut doc2 = AutoCommit::new();
        create_test_dish_in_doc(&mut doc2, "550e8400-e29b-41d4-a716-446655440003", "Dish B");
        DishProjection::project_all(&doc2, &pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dishes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1);

        let name: (String,) = sqlx::query_as("SELECT name FROM dishes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name.0, "Dish B");
    }

    #[tokio::test]
    async fn test_project_all_multiple_dishes() {
        let (pool, _temp) = setup_db().await;
        let mut doc = AutoCommit::new();

        create_test_dish_in_doc(&mut doc, "550e8400-e29b-41d4-a716-446655440010", "Dish 1");
        create_test_dish_in_doc(&mut doc, "550e8400-e29b-41d4-a716-446655440011", "Dish 2");
        create_test_dish_in_doc(&mut doc, "550e8400-e29b-41d4-a716-446655440012", "Dish 3");

        DishProjection::project_all(&doc, &pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dishes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 3);
    }

    // ========== MealPlanProjection Tests ==========

    fn create_test_mealplan_in_doc(doc: &mut AutoCommit, id: &str, title: &str, dish_ids: &[&str]) {
        let plan_id = doc
            .put_object(ROOT, id, ObjType::Map)
            .expect("Failed to create mealplan object");

        doc.put(&plan_id, "date", "2025-01-15").unwrap();
        doc.put(&plan_id, "meal_type", "dinner").unwrap();
        doc.put(&plan_id, "title", title).unwrap();
        doc.put(&plan_id, "cook", "Chef").unwrap();
        doc.put(&plan_id, "created_by", "testuser").unwrap();
        doc.put(&plan_id, "created_at", "2025-01-01T00:00:00Z")
            .unwrap();
        doc.put(&plan_id, "updated_at", "2025-01-01T00:00:00Z")
            .unwrap();

        // Add dish IDs
        let dishes_id = doc.put_object(&plan_id, "dishes", ObjType::List).unwrap();
        for (i, dish_id) in dish_ids.iter().enumerate() {
            doc.insert(&dishes_id, i, *dish_id).unwrap();
        }
    }

    #[tokio::test]
    async fn test_mealplan_project_all_empty_doc() {
        let (pool, _temp) = setup_db().await;
        let doc = AutoCommit::new();

        MealPlanProjection::project_all(&doc, &pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mealplans")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn test_mealplan_project_all_single() {
        let (pool, _temp) = setup_db().await;
        let mut doc = AutoCommit::new();

        let uuid = "550e8400-e29b-41d4-a716-446655440100";
        create_test_mealplan_in_doc(&mut doc, uuid, "Sunday Dinner", &[]);

        MealPlanProjection::project_all(&doc, &pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mealplans")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1);

        let title: (String,) = sqlx::query_as("SELECT title FROM mealplans WHERE id = ?")
            .bind(uuid)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(title.0, "Sunday Dinner");
    }

    #[tokio::test]
    async fn test_mealplan_project_with_dish_ids() {
        let (pool, _temp) = setup_db().await;

        // First, create dishes in the dishes table
        let dish1_uuid = "550e8400-e29b-41d4-a716-446655440001";
        let dish2_uuid = "550e8400-e29b-41d4-a716-446655440002";

        let mut dish_doc = AutoCommit::new();
        create_test_dish_in_doc(&mut dish_doc, dish1_uuid, "Pasta");
        create_test_dish_in_doc(&mut dish_doc, dish2_uuid, "Salad");
        DishProjection::project_all(&dish_doc, &pool).await.unwrap();

        // Now create mealplan referencing those dishes
        let mut mealplan_doc = AutoCommit::new();
        let mealplan_uuid = "550e8400-e29b-41d4-a716-446655440100";
        create_test_mealplan_in_doc(
            &mut mealplan_doc,
            mealplan_uuid,
            "Dinner with Dishes",
            &[dish1_uuid, dish2_uuid],
        );

        MealPlanProjection::project_all(&mealplan_doc, &pool)
            .await
            .unwrap();

        // Verify junction table
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM mealplan_dishes WHERE mealplan_id = ?")
                .bind(mealplan_uuid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count.0, 2);
    }

    #[tokio::test]
    async fn test_mealplan_project_replaces_existing() {
        let (pool, _temp) = setup_db().await;

        // First projection
        let mut doc1 = AutoCommit::new();
        create_test_mealplan_in_doc(
            &mut doc1,
            "550e8400-e29b-41d4-a716-446655440100",
            "Plan A",
            &[],
        );
        MealPlanProjection::project_all(&doc1, &pool).await.unwrap();

        // Second projection (should replace)
        let mut doc2 = AutoCommit::new();
        create_test_mealplan_in_doc(
            &mut doc2,
            "550e8400-e29b-41d4-a716-446655440101",
            "Plan B",
            &[],
        );
        MealPlanProjection::project_all(&doc2, &pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mealplans")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1);

        let title: (String,) = sqlx::query_as("SELECT title FROM mealplans")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(title.0, "Plan B");
    }

    // ========== MealLogProjection Tests ==========

    fn create_test_meallog_in_doc(
        doc: &mut AutoCommit,
        id: &str,
        mealplan_id: Option<&str>,
        notes: Option<&str>,
        dish_ids: &[&str],
    ) {
        let log_id = doc
            .put_object(ROOT, id, ObjType::Map)
            .expect("Failed to create meallog object");

        doc.put(&log_id, "date", "2025-01-15").unwrap();
        doc.put(&log_id, "meal_type", "lunch").unwrap();
        doc.put(&log_id, "created_by", "testuser").unwrap();
        doc.put(&log_id, "created_at", "2025-01-01T00:00:00Z")
            .unwrap();

        if let Some(plan_id) = mealplan_id {
            doc.put(&log_id, "mealplan_id", plan_id).unwrap();
        }

        if let Some(n) = notes {
            doc.put(&log_id, "notes", n).unwrap();
        }

        // Add dish IDs
        let dishes_id = doc.put_object(&log_id, "dishes", ObjType::List).unwrap();
        for (i, dish_id) in dish_ids.iter().enumerate() {
            doc.insert(&dishes_id, i, *dish_id).unwrap();
        }
    }

    #[tokio::test]
    async fn test_meallog_project_all_empty_doc() {
        let (pool, _temp) = setup_db().await;
        let doc = AutoCommit::new();

        MealLogProjection::project_all(&doc, &pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM meallogs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn test_meallog_project_with_null_mealplan_id() {
        let (pool, _temp) = setup_db().await;
        let mut doc = AutoCommit::new();

        let uuid = "550e8400-e29b-41d4-a716-446655440200";
        create_test_meallog_in_doc(&mut doc, uuid, None, Some("Had a snack"), &[]);

        MealLogProjection::project_all(&doc, &pool).await.unwrap();

        let row: (Option<String>, Option<String>) =
            sqlx::query_as("SELECT mealplan_id, notes FROM meallogs WHERE id = ?")
                .bind(uuid)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert!(row.0.is_none()); // mealplan_id should be null
        assert_eq!(row.1, Some("Had a snack".to_string()));
    }

    #[tokio::test]
    async fn test_meallog_project_with_mealplan_id() {
        let (pool, _temp) = setup_db().await;

        // Create a mealplan first
        let mealplan_uuid = "550e8400-e29b-41d4-a716-446655440100";
        let mut mealplan_doc = AutoCommit::new();
        create_test_mealplan_in_doc(&mut mealplan_doc, mealplan_uuid, "Lunch Plan", &[]);
        MealPlanProjection::project_all(&mealplan_doc, &pool)
            .await
            .unwrap();

        // Create meallog referencing the mealplan
        let meallog_uuid = "550e8400-e29b-41d4-a716-446655440200";
        let mut meallog_doc = AutoCommit::new();
        create_test_meallog_in_doc(
            &mut meallog_doc,
            meallog_uuid,
            Some(mealplan_uuid),
            None,
            &[],
        );
        MealLogProjection::project_all(&meallog_doc, &pool)
            .await
            .unwrap();

        let mealplan_id: (Option<String>,) =
            sqlx::query_as("SELECT mealplan_id FROM meallogs WHERE id = ?")
                .bind(meallog_uuid)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(mealplan_id.0, Some(mealplan_uuid.to_string()));
    }

    #[tokio::test]
    async fn test_meallog_project_with_dish_ids() {
        let (pool, _temp) = setup_db().await;

        // Create dishes
        let dish_uuid = "550e8400-e29b-41d4-a716-446655440001";
        let mut dish_doc = AutoCommit::new();
        create_test_dish_in_doc(&mut dish_doc, dish_uuid, "Soup");
        DishProjection::project_all(&dish_doc, &pool).await.unwrap();

        // Create meallog with dish
        let meallog_uuid = "550e8400-e29b-41d4-a716-446655440200";
        let mut meallog_doc = AutoCommit::new();
        create_test_meallog_in_doc(&mut meallog_doc, meallog_uuid, None, None, &[dish_uuid]);
        MealLogProjection::project_all(&meallog_doc, &pool)
            .await
            .unwrap();

        // Verify junction table
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM meallog_dishes WHERE meallog_id = ?")
                .bind(meallog_uuid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn test_meallog_project_replaces_existing() {
        let (pool, _temp) = setup_db().await;

        // First projection
        let mut doc1 = AutoCommit::new();
        create_test_meallog_in_doc(
            &mut doc1,
            "550e8400-e29b-41d4-a716-446655440200",
            None,
            Some("Note 1"),
            &[],
        );
        MealLogProjection::project_all(&doc1, &pool).await.unwrap();

        // Second projection (should replace)
        let mut doc2 = AutoCommit::new();
        create_test_meallog_in_doc(
            &mut doc2,
            "550e8400-e29b-41d4-a716-446655440201",
            None,
            Some("Note 2"),
            &[],
        );
        MealLogProjection::project_all(&doc2, &pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM meallogs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1);

        let notes: (Option<String>,) = sqlx::query_as("SELECT notes FROM meallogs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(notes.0, Some("Note 2".to_string()));
    }
}
