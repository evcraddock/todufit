use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{Dish, Ingredient, MealLog, MealType, Nutrient};

pub struct MealLogRepository {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct MealLogRow {
    id: String,
    date: String,
    meal_type: String,
    mealplan_id: Option<String>,
    notes: Option<String>,
    created_by: String,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct DishRow {
    id: String,
    name: String,
    instructions: String,
    prep_time: Option<i32>,
    cook_time: Option<i32>,
    servings: Option<i32>,
    tags: String,
    image_url: Option<String>,
    source_url: Option<String>,
    created_by: String,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct IngredientRow {
    name: String,
    quantity: f64,
    unit: String,
}

#[derive(sqlx::FromRow)]
struct NutrientRow {
    name: String,
    amount: f64,
    unit: String,
}

impl MealLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, log: &MealLog) -> Result<MealLog, sqlx::Error> {
        let id = log.id.to_string();
        let date = log.date.to_string();
        let meal_type = log.meal_type.to_string();
        let mealplan_id = log.mealplan_id.map(|id| id.to_string());
        let created_at = log.created_at.to_rfc3339();

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
        .bind(&log.notes)
        .bind(&log.created_by)
        .bind(&created_at)
        .execute(&self.pool)
        .await?;

        // Add dishes
        for dish in &log.dishes {
            self.add_dish(log.id, dish.id).await?;
        }

        self.get_by_id(log.id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MealLog>, sqlx::Error> {
        let id_str = id.to_string();

        let row: Option<MealLogRow> = sqlx::query_as("SELECT * FROM meallogs WHERE id = ?")
            .bind(&id_str)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => self.hydrate_meallog(row).await.map(Some),
            None => Ok(None),
        }
    }

    pub async fn list_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealLog>, sqlx::Error> {
        let from_str = from.to_string();
        let to_str = to.to_string();

        let rows: Vec<MealLogRow> = sqlx::query_as(
            "SELECT * FROM meallogs WHERE date >= ? AND date <= ? ORDER BY date, meal_type",
        )
        .bind(&from_str)
        .bind(&to_str)
        .fetch_all(&self.pool)
        .await?;

        let mut logs = Vec::with_capacity(rows.len());
        for row in rows {
            logs.push(self.hydrate_meallog(row).await?);
        }
        Ok(logs)
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let id_str = id.to_string();
        sqlx::query("DELETE FROM meallogs WHERE id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn add_dish(&self, meallog_id: Uuid, dish_id: Uuid) -> Result<(), sqlx::Error> {
        let meallog_id_str = meallog_id.to_string();
        let dish_id_str = dish_id.to_string();

        sqlx::query("INSERT OR IGNORE INTO meallog_dishes (meallog_id, dish_id) VALUES (?, ?)")
            .bind(&meallog_id_str)
            .bind(&dish_id_str)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_dish(&self, meallog_id: Uuid, dish_id: Uuid) -> Result<(), sqlx::Error> {
        let meallog_id_str = meallog_id.to_string();
        let dish_id_str = dish_id.to_string();

        sqlx::query("DELETE FROM meallog_dishes WHERE meallog_id = ? AND dish_id = ?")
            .bind(&meallog_id_str)
            .bind(&dish_id_str)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn hydrate_meallog(&self, row: MealLogRow) -> Result<MealLog, sqlx::Error> {
        // Get associated dishes
        let dish_rows: Vec<DishRow> = sqlx::query_as(
            r#"
            SELECT d.* FROM dishes d
            INNER JOIN meallog_dishes ml ON d.id = ml.dish_id
            WHERE ml.meallog_id = ?
            "#,
        )
        .bind(&row.id)
        .fetch_all(&self.pool)
        .await?;

        let mut dishes = Vec::with_capacity(dish_rows.len());
        for dish_row in dish_rows {
            dishes.push(self.hydrate_dish(dish_row).await?);
        }

        let meal_type: MealType = row.meal_type.parse().unwrap_or(MealType::Dinner);
        let mealplan_id = row
            .mealplan_id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok());

        Ok(MealLog {
            id: Uuid::parse_str(&row.id).unwrap(),
            date: NaiveDate::parse_from_str(&row.date, "%Y-%m-%d").unwrap(),
            meal_type,
            mealplan_id,
            dishes,
            notes: row.notes,
            created_by: row.created_by,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }

    async fn hydrate_dish(&self, row: DishRow) -> Result<Dish, sqlx::Error> {
        let ingredients: Vec<IngredientRow> =
            sqlx::query_as("SELECT name, quantity, unit FROM ingredients WHERE dish_id = ?")
                .bind(&row.id)
                .fetch_all(&self.pool)
                .await?;

        let nutrients: Vec<NutrientRow> =
            sqlx::query_as("SELECT name, amount, unit FROM nutrients WHERE dish_id = ?")
                .bind(&row.id)
                .fetch_all(&self.pool)
                .await?;

        let tags: Vec<String> = serde_json::from_str(&row.tags).unwrap_or_default();

        let nutrients = if nutrients.is_empty() {
            None
        } else {
            Some(
                nutrients
                    .into_iter()
                    .map(|n| Nutrient::new(n.name, n.amount, n.unit))
                    .collect(),
            )
        };

        Ok(Dish {
            id: Uuid::parse_str(&row.id).unwrap(),
            name: row.name,
            ingredients: ingredients
                .into_iter()
                .map(|i| Ingredient::new(i.name, i.quantity, i.unit))
                .collect(),
            instructions: row.instructions,
            nutrients,
            prep_time: row.prep_time,
            cook_time: row.cook_time,
            servings: row.servings,
            tags,
            image_url: row.image_url,
            source_url: row.source_url,
            created_by: row.created_by,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_db, DishRepository, MealPlanRepository};
    use crate::models::MealPlan;
    use tempfile::TempDir;

    struct TestContext {
        meallog_repo: MealLogRepository,
        mealplan_repo: MealPlanRepository,
        dish_repo: DishRepository,
        _temp_dir: TempDir,
    }

    async fn setup() -> TestContext {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        TestContext {
            meallog_repo: MealLogRepository::new(pool.clone()),
            mealplan_repo: MealPlanRepository::new(pool.clone()),
            dish_repo: DishRepository::new(pool),
            _temp_dir: temp_dir,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_meallog() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let log = MealLog::new(date, MealType::Dinner, "user1").with_notes("Delicious dinner");
        let created = ctx.meallog_repo.create(&log).await.unwrap();

        assert_eq!(created.date, date);
        assert_eq!(created.meal_type, MealType::Dinner);
        assert_eq!(created.notes, Some("Delicious dinner".to_string()));
        assert!(created.mealplan_id.is_none());

        let fetched = ctx.meallog_repo.get_by_id(log.id).await.unwrap().unwrap();
        assert_eq!(fetched.notes, Some("Delicious dinner".to_string()));
    }

    #[tokio::test]
    async fn test_meallog_with_mealplan_id() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        // Create a meal plan first
        let plan = MealPlan::new(date, MealType::Lunch, "Planned Lunch", "user1");
        ctx.mealplan_repo.create(&plan).await.unwrap();

        // Create a meal log referencing the plan
        let log = MealLog::new(date, MealType::Lunch, "user1").with_mealplan_id(plan.id);
        let created = ctx.meallog_repo.create(&log).await.unwrap();

        assert_eq!(created.mealplan_id, Some(plan.id));

        let fetched = ctx.meallog_repo.get_by_id(log.id).await.unwrap().unwrap();
        assert_eq!(fetched.mealplan_id, Some(plan.id));
    }

    #[tokio::test]
    async fn test_meallog_mealplan_id_can_be_none() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let log = MealLog::new(date, MealType::Snack, "user1");
        let created = ctx.meallog_repo.create(&log).await.unwrap();

        assert!(created.mealplan_id.is_none());

        let fetched = ctx.meallog_repo.get_by_id(log.id).await.unwrap().unwrap();
        assert!(fetched.mealplan_id.is_none());
    }

    #[tokio::test]
    async fn test_list_range() {
        let ctx = setup().await;

        let jan1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let jan5 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let jan10 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        ctx.meallog_repo
            .create(&MealLog::new(jan1, MealType::Dinner, "user1"))
            .await
            .unwrap();
        ctx.meallog_repo
            .create(&MealLog::new(jan5, MealType::Lunch, "user1"))
            .await
            .unwrap();
        ctx.meallog_repo
            .create(&MealLog::new(jan10, MealType::Breakfast, "user1"))
            .await
            .unwrap();

        // Query range jan1 to jan5 should return 2 logs
        let logs = ctx.meallog_repo.list_range(jan1, jan5).await.unwrap();
        assert_eq!(logs.len(), 2);

        // Full range should return all 3
        let all_logs = ctx.meallog_repo.list_range(jan1, jan10).await.unwrap();
        assert_eq!(all_logs.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_meallog() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let log = MealLog::new(date, MealType::Dinner, "user1");
        ctx.meallog_repo.create(&log).await.unwrap();

        ctx.meallog_repo.delete(log.id).await.unwrap();

        let fetched = ctx.meallog_repo.get_by_id(log.id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_meallog_with_dishes() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        // Create dishes
        let dish1 = Dish::new("Pasta", "user1");
        let dish2 = Dish::new("Salad", "user1");
        ctx.dish_repo.create(&dish1).await.unwrap();
        ctx.dish_repo.create(&dish2).await.unwrap();

        // Create meal log with dishes
        let log = MealLog::new(date, MealType::Dinner, "user1")
            .with_dishes(vec![dish1.clone(), dish2.clone()]);
        let created = ctx.meallog_repo.create(&log).await.unwrap();

        assert_eq!(created.dishes.len(), 2);

        let fetched = ctx.meallog_repo.get_by_id(log.id).await.unwrap().unwrap();
        assert_eq!(fetched.dishes.len(), 2);

        let dish_names: Vec<&str> = fetched.dishes.iter().map(|d| d.name.as_str()).collect();
        assert!(dish_names.contains(&"Pasta"));
        assert!(dish_names.contains(&"Salad"));
    }

    #[tokio::test]
    async fn test_add_and_remove_dish() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        // Create a dish
        let dish = Dish::new("Test Dish", "user1");
        ctx.dish_repo.create(&dish).await.unwrap();

        // Create a meal log
        let log = MealLog::new(date, MealType::Dinner, "user1");
        ctx.meallog_repo.create(&log).await.unwrap();

        // Add dish to meallog
        ctx.meallog_repo.add_dish(log.id, dish.id).await.unwrap();

        // Verify dish is associated
        let fetched = ctx.meallog_repo.get_by_id(log.id).await.unwrap().unwrap();
        assert_eq!(fetched.dishes.len(), 1);
        assert_eq!(fetched.dishes[0].name, "Test Dish");

        // Remove dish
        ctx.meallog_repo.remove_dish(log.id, dish.id).await.unwrap();

        // Verify dish is removed
        let fetched = ctx.meallog_repo.get_by_id(log.id).await.unwrap().unwrap();
        assert!(fetched.dishes.is_empty());
    }
}
