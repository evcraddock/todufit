use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{MealPlan, MealType};

pub struct MealPlanRepository {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct MealPlanRow {
    id: String,
    date: String,
    meal_type: String,
    title: String,
    cook: String,
    created_by: String,
    created_at: String,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct DishIdRow {
    dish_id: String,
}

impl MealPlanRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[cfg(test)]
    pub async fn create(&self, mealplan: &MealPlan) -> Result<MealPlan, sqlx::Error> {
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
        .execute(&self.pool)
        .await?;

        // Add dish associations
        for dish_id in &mealplan.dish_ids {
            self.add_dish(mealplan.id, *dish_id).await?;
        }

        self.get_by_id(mealplan.id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MealPlan>, sqlx::Error> {
        let id_str = id.to_string();

        let row: Option<MealPlanRow> = sqlx::query_as("SELECT * FROM mealplans WHERE id = ?")
            .bind(&id_str)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => self.hydrate_mealplan(row).await.map(Some),
            None => Ok(None),
        }
    }

    pub async fn get_by_date(&self, date: NaiveDate) -> Result<Vec<MealPlan>, sqlx::Error> {
        let date_str = date.to_string();

        let rows: Vec<MealPlanRow> =
            sqlx::query_as("SELECT * FROM mealplans WHERE date = ? ORDER BY meal_type")
                .bind(&date_str)
                .fetch_all(&self.pool)
                .await?;

        let mut plans = Vec::with_capacity(rows.len());
        for row in rows {
            plans.push(self.hydrate_mealplan(row).await?);
        }
        Ok(plans)
    }

    pub async fn get_by_date_and_type(
        &self,
        date: NaiveDate,
        meal_type: MealType,
    ) -> Result<Option<MealPlan>, sqlx::Error> {
        let date_str = date.to_string();
        let meal_type_str = meal_type.to_string();

        let row: Option<MealPlanRow> =
            sqlx::query_as("SELECT * FROM mealplans WHERE date = ? AND meal_type = ?")
                .bind(&date_str)
                .bind(&meal_type_str)
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some(row) => self.hydrate_mealplan(row).await.map(Some),
            None => Ok(None),
        }
    }

    pub async fn list(&self) -> Result<Vec<MealPlan>, sqlx::Error> {
        let rows: Vec<MealPlanRow> =
            sqlx::query_as("SELECT * FROM mealplans ORDER BY date DESC, meal_type")
                .fetch_all(&self.pool)
                .await?;

        let mut plans = Vec::with_capacity(rows.len());
        for row in rows {
            plans.push(self.hydrate_mealplan(row).await?);
        }
        Ok(plans)
    }

    pub async fn list_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealPlan>, sqlx::Error> {
        let from_str = from.to_string();
        let to_str = to.to_string();

        let rows: Vec<MealPlanRow> = sqlx::query_as(
            "SELECT * FROM mealplans WHERE date >= ? AND date <= ? ORDER BY date, meal_type",
        )
        .bind(&from_str)
        .bind(&to_str)
        .fetch_all(&self.pool)
        .await?;

        let mut plans = Vec::with_capacity(rows.len());
        for row in rows {
            plans.push(self.hydrate_mealplan(row).await?);
        }
        Ok(plans)
    }

    #[cfg(test)]
    pub async fn update(&self, mealplan: &MealPlan) -> Result<MealPlan, sqlx::Error> {
        let id = mealplan.id.to_string();
        let date = mealplan.date.to_string();
        let meal_type = mealplan.meal_type.to_string();
        let updated_at = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE mealplans 
            SET date = ?, meal_type = ?, title = ?, cook = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&date)
        .bind(&meal_type)
        .bind(&mealplan.title)
        .bind(&mealplan.cook)
        .bind(&updated_at)
        .bind(&id)
        .execute(&self.pool)
        .await?;

        self.get_by_id(mealplan.id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    #[cfg(test)]
    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let id_str = id.to_string();
        sqlx::query("DELETE FROM mealplans WHERE id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[cfg(test)]
    pub async fn add_dish(&self, mealplan_id: Uuid, dish_id: Uuid) -> Result<(), sqlx::Error> {
        let mealplan_id_str = mealplan_id.to_string();
        let dish_id_str = dish_id.to_string();

        sqlx::query("INSERT OR IGNORE INTO mealplan_dishes (mealplan_id, dish_id) VALUES (?, ?)")
            .bind(&mealplan_id_str)
            .bind(&dish_id_str)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[cfg(test)]
    pub async fn remove_dish(&self, mealplan_id: Uuid, dish_id: Uuid) -> Result<(), sqlx::Error> {
        let mealplan_id_str = mealplan_id.to_string();
        let dish_id_str = dish_id.to_string();

        sqlx::query("DELETE FROM mealplan_dishes WHERE mealplan_id = ? AND dish_id = ?")
            .bind(&mealplan_id_str)
            .bind(&dish_id_str)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn hydrate_mealplan(&self, row: MealPlanRow) -> Result<MealPlan, sqlx::Error> {
        // Get associated dish IDs
        let dish_id_rows: Vec<DishIdRow> =
            sqlx::query_as("SELECT dish_id FROM mealplan_dishes WHERE mealplan_id = ?")
                .bind(&row.id)
                .fetch_all(&self.pool)
                .await?;

        let dish_ids: Vec<Uuid> = dish_id_rows
            .into_iter()
            .filter_map(|r| Uuid::parse_str(&r.dish_id).ok())
            .collect();

        let meal_type: MealType = row.meal_type.parse().unwrap_or(MealType::Dinner);

        Ok(MealPlan {
            id: Uuid::parse_str(&row.id).unwrap(),
            date: NaiveDate::parse_from_str(&row.date, "%Y-%m-%d").unwrap(),
            meal_type,
            title: row.title,
            cook: row.cook,
            dish_ids,
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
    use crate::db::{init_db, DishRepository};
    use crate::models::Dish;
    use tempfile::TempDir;

    struct TestContext {
        mealplan_repo: MealPlanRepository,
        dish_repo: DishRepository,
        _temp_dir: TempDir,
    }

    async fn setup() -> TestContext {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        TestContext {
            mealplan_repo: MealPlanRepository::new(pool.clone()),
            dish_repo: DishRepository::new(pool),
            _temp_dir: temp_dir,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_mealplan() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Test Dinner", "user1");
        let created = ctx.mealplan_repo.create(&plan).await.unwrap();

        assert_eq!(created.title, "Test Dinner");
        assert_eq!(created.date, date);
        assert_eq!(created.meal_type, MealType::Dinner);

        let fetched = ctx.mealplan_repo.get_by_id(plan.id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Test Dinner");
    }

    #[tokio::test]
    async fn test_get_by_date() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        ctx.mealplan_repo
            .create(&MealPlan::new(
                date,
                MealType::Breakfast,
                "Breakfast",
                "user1",
            ))
            .await
            .unwrap();
        ctx.mealplan_repo
            .create(&MealPlan::new(date, MealType::Dinner, "Dinner", "user1"))
            .await
            .unwrap();

        let plans = ctx.mealplan_repo.get_by_date(date).await.unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_date_and_type() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        ctx.mealplan_repo
            .create(&MealPlan::new(date, MealType::Lunch, "Lunch", "user1"))
            .await
            .unwrap();

        let plan = ctx
            .mealplan_repo
            .get_by_date_and_type(date, MealType::Lunch)
            .await
            .unwrap();
        assert!(plan.is_some());
        assert_eq!(plan.unwrap().title, "Lunch");

        let none = ctx
            .mealplan_repo
            .get_by_date_and_type(date, MealType::Dinner)
            .await
            .unwrap();
        assert!(none.is_none());
    }

    #[tokio::test]
    async fn test_list_range() {
        let ctx = setup().await;

        let jan1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let jan5 = NaiveDate::from_ymd_opt(2025, 1, 5).unwrap();
        let jan10 = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();

        ctx.mealplan_repo
            .create(&MealPlan::new(jan1, MealType::Dinner, "Jan 1", "user1"))
            .await
            .unwrap();
        ctx.mealplan_repo
            .create(&MealPlan::new(jan5, MealType::Dinner, "Jan 5", "user1"))
            .await
            .unwrap();
        ctx.mealplan_repo
            .create(&MealPlan::new(jan10, MealType::Dinner, "Jan 10", "user1"))
            .await
            .unwrap();

        let plans = ctx.mealplan_repo.list_range(jan1, jan5).await.unwrap();
        assert_eq!(plans.len(), 2);
    }

    #[tokio::test]
    async fn test_add_and_remove_dish() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        // Create a dish
        let dish = Dish::new("Test Dish", "user1");
        ctx.dish_repo.create(&dish).await.unwrap();

        // Create a mealplan
        let plan = MealPlan::new(date, MealType::Dinner, "Dinner", "user1");
        ctx.mealplan_repo.create(&plan).await.unwrap();

        // Add dish to mealplan
        ctx.mealplan_repo.add_dish(plan.id, dish.id).await.unwrap();

        // Verify dish ID is associated
        let fetched = ctx.mealplan_repo.get_by_id(plan.id).await.unwrap().unwrap();
        assert_eq!(fetched.dish_ids.len(), 1);
        assert_eq!(fetched.dish_ids[0], dish.id);

        // Remove dish
        ctx.mealplan_repo
            .remove_dish(plan.id, dish.id)
            .await
            .unwrap();

        // Verify dish is removed
        let fetched = ctx.mealplan_repo.get_by_id(plan.id).await.unwrap().unwrap();
        assert!(fetched.dish_ids.is_empty());
    }

    #[tokio::test]
    async fn test_delete_mealplan() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "To Delete", "user1");
        ctx.mealplan_repo.create(&plan).await.unwrap();

        ctx.mealplan_repo.delete(plan.id).await.unwrap();

        let fetched = ctx.mealplan_repo.get_by_id(plan.id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_update_mealplan() {
        let ctx = setup().await;
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let plan = MealPlan::new(date, MealType::Dinner, "Original", "user1");
        let created = ctx.mealplan_repo.create(&plan).await.unwrap();

        let mut updated_plan = created.clone();
        updated_plan.title = "Updated".to_string();
        updated_plan.cook = "Chef".to_string();

        let updated = ctx.mealplan_repo.update(&updated_plan).await.unwrap();
        assert_eq!(updated.title, "Updated");
        assert_eq!(updated.cook, "Chef");
    }
}
