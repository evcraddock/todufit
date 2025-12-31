use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{Dish, Ingredient, Nutrient};

pub struct DishRepository {
    pool: SqlitePool,
}

// Row types for database queries
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

impl DishRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, dish: &Dish) -> Result<Dish, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

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
        .execute(&mut *tx)
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
            .execute(&mut *tx)
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
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        // Return the created dish
        self.get_by_id(dish.id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Dish>, sqlx::Error> {
        let id_str = id.to_string();

        let row: Option<DishRow> = sqlx::query_as("SELECT * FROM dishes WHERE id = ?")
            .bind(&id_str)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => self.hydrate_dish(row).await.map(Some),
            None => Ok(None),
        }
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Option<Dish>, sqlx::Error> {
        let row: Option<DishRow> =
            sqlx::query_as("SELECT * FROM dishes WHERE LOWER(name) = LOWER(?)")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some(row) => self.hydrate_dish(row).await.map(Some),
            None => Ok(None),
        }
    }

    pub async fn list(&self) -> Result<Vec<Dish>, sqlx::Error> {
        let rows: Vec<DishRow> = sqlx::query_as("SELECT * FROM dishes ORDER BY name")
            .fetch_all(&self.pool)
            .await?;

        let mut dishes = Vec::with_capacity(rows.len());
        for row in rows {
            dishes.push(self.hydrate_dish(row).await?);
        }
        Ok(dishes)
    }

    pub async fn update(&self, dish: &Dish) -> Result<Dish, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let id = dish.id.to_string();
        let tags = serde_json::to_string(&dish.tags).unwrap_or_else(|_| "[]".to_string());
        let updated_at = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE dishes 
            SET name = ?, instructions = ?, prep_time = ?, cook_time = ?, servings = ?, 
                tags = ?, image_url = ?, source_url = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&dish.name)
        .bind(&dish.instructions)
        .bind(dish.prep_time)
        .bind(dish.cook_time)
        .bind(dish.servings)
        .bind(&tags)
        .bind(&dish.image_url)
        .bind(&dish.source_url)
        .bind(&updated_at)
        .bind(&id)
        .execute(&mut *tx)
        .await?;

        // Replace ingredients
        sqlx::query("DELETE FROM ingredients WHERE dish_id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await?;

        for ingredient in &dish.ingredients {
            sqlx::query(
                "INSERT INTO ingredients (dish_id, name, quantity, unit) VALUES (?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&ingredient.name)
            .bind(ingredient.quantity)
            .bind(&ingredient.unit)
            .execute(&mut *tx)
            .await?;
        }

        // Replace nutrients
        sqlx::query("DELETE FROM nutrients WHERE dish_id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await?;

        if let Some(nutrients) = &dish.nutrients {
            for nutrient in nutrients {
                sqlx::query(
                    "INSERT INTO nutrients (dish_id, name, amount, unit) VALUES (?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(&nutrient.name)
                .bind(nutrient.amount)
                .bind(&nutrient.unit)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        self.get_by_id(dish.id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        let id_str = id.to_string();
        // CASCADE will handle ingredients and nutrients
        sqlx::query("DELETE FROM dishes WHERE id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn add_ingredient(
        &self,
        dish_id: Uuid,
        ingredient: &Ingredient,
    ) -> Result<(), sqlx::Error> {
        let id_str = dish_id.to_string();
        sqlx::query("INSERT INTO ingredients (dish_id, name, quantity, unit) VALUES (?, ?, ?, ?)")
            .bind(&id_str)
            .bind(&ingredient.name)
            .bind(ingredient.quantity)
            .bind(&ingredient.unit)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_ingredient(
        &self,
        dish_id: Uuid,
        ingredient_name: &str,
    ) -> Result<(), sqlx::Error> {
        let id_str = dish_id.to_string();
        sqlx::query("DELETE FROM ingredients WHERE dish_id = ? AND LOWER(name) = LOWER(?)")
            .bind(&id_str)
            .bind(ingredient_name)
            .execute(&self.pool)
            .await?;
        Ok(())
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
    use crate::db::init_db;
    use tempfile::TempDir;

    struct TestContext {
        repo: DishRepository,
        _temp_dir: TempDir, // Keep alive for duration of test
    }

    async fn setup_repo() -> TestContext {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = init_db(Some(db_path)).await.unwrap();
        TestContext {
            repo: DishRepository::new(pool),
            _temp_dir: temp_dir,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_dish() {
        let ctx = setup_repo().await;
        let repo = &ctx.repo;

        let dish = Dish::new("Test Pasta", "user1")
            .with_ingredients(vec![
                Ingredient::new("pasta", 200.0, "g"),
                Ingredient::new("sauce", 1.0, "cup"),
            ])
            .with_instructions("Boil pasta. Add sauce.")
            .with_prep_time(5)
            .with_cook_time(15)
            .with_servings(2)
            .with_tags(vec!["italian".into(), "quick".into()]);

        let created = repo.create(&dish).await.unwrap();
        assert_eq!(created.name, "Test Pasta");
        assert_eq!(created.ingredients.len(), 2);

        let fetched = repo.get_by_id(dish.id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "Test Pasta");
        assert_eq!(fetched.ingredients.len(), 2);
        assert_eq!(fetched.tags, vec!["italian", "quick"]);
    }

    #[tokio::test]
    async fn test_get_by_name_case_insensitive() {
        let ctx = setup_repo().await;
        let repo = &ctx.repo;

        let dish = Dish::new("Chicken Curry", "user1");
        repo.create(&dish).await.unwrap();

        let found = repo.get_by_name("chicken curry").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Chicken Curry");

        let found = repo.get_by_name("CHICKEN CURRY").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_list_dishes() {
        let ctx = setup_repo().await;
        let repo = &ctx.repo;

        repo.create(&Dish::new("Dish A", "user1")).await.unwrap();
        repo.create(&Dish::new("Dish B", "user1")).await.unwrap();
        repo.create(&Dish::new("Dish C", "user1")).await.unwrap();

        let dishes = repo.list().await.unwrap();
        assert_eq!(dishes.len(), 3);
        // Should be sorted by name
        assert_eq!(dishes[0].name, "Dish A");
        assert_eq!(dishes[1].name, "Dish B");
        assert_eq!(dishes[2].name, "Dish C");
    }

    #[tokio::test]
    async fn test_update_dish() {
        let ctx = setup_repo().await;
        let repo = &ctx.repo;

        let dish = Dish::new("Original Name", "user1")
            .with_ingredients(vec![Ingredient::new("item1", 1.0, "unit")]);
        let created = repo.create(&dish).await.unwrap();

        let mut updated_dish = created.clone();
        updated_dish.name = "Updated Name".to_string();
        updated_dish.ingredients = vec![
            Ingredient::new("new_item1", 2.0, "cups"),
            Ingredient::new("new_item2", 3.0, "tbsp"),
        ];

        let updated = repo.update(&updated_dish).await.unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.ingredients.len(), 2);

        let fetched = repo.get_by_id(dish.id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "Updated Name");
        assert_eq!(fetched.ingredients.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_dish_cascades() {
        let ctx = setup_repo().await;
        let repo = &ctx.repo;

        let dish = Dish::new("To Delete", "user1")
            .with_ingredients(vec![Ingredient::new("item", 1.0, "unit")])
            .with_nutrients(vec![Nutrient::new("calories", 100.0, "kcal")]);

        repo.create(&dish).await.unwrap();

        // Verify it exists
        assert!(repo.get_by_id(dish.id).await.unwrap().is_some());

        // Delete
        repo.delete(dish.id).await.unwrap();

        // Verify it's gone
        assert!(repo.get_by_id(dish.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_add_ingredient() {
        let ctx = setup_repo().await;
        let repo = &ctx.repo;

        let dish = Dish::new("Test Dish", "user1");
        repo.create(&dish).await.unwrap();

        repo.add_ingredient(dish.id, &Ingredient::new("new ingredient", 5.0, "oz"))
            .await
            .unwrap();

        let fetched = repo.get_by_id(dish.id).await.unwrap().unwrap();
        assert_eq!(fetched.ingredients.len(), 1);
        assert_eq!(fetched.ingredients[0].name, "new ingredient");
    }

    #[tokio::test]
    async fn test_remove_ingredient() {
        let ctx = setup_repo().await;
        let repo = &ctx.repo;

        let dish = Dish::new("Test Dish", "user1").with_ingredients(vec![
            Ingredient::new("keep", 1.0, "unit"),
            Ingredient::new("remove", 2.0, "unit"),
        ]);
        repo.create(&dish).await.unwrap();

        repo.remove_ingredient(dish.id, "Remove").await.unwrap(); // case-insensitive

        let fetched = repo.get_by_id(dish.id).await.unwrap().unwrap();
        assert_eq!(fetched.ingredients.len(), 1);
        assert_eq!(fetched.ingredients[0].name, "keep");
    }

    #[tokio::test]
    async fn test_dish_with_nutrients() {
        let ctx = setup_repo().await;
        let repo = &ctx.repo;

        let dish = Dish::new("Nutritious Dish", "user1").with_nutrients(vec![
            Nutrient::new("calories", 250.0, "kcal"),
            Nutrient::new("protein", 15.0, "g"),
        ]);

        repo.create(&dish).await.unwrap();

        let fetched = repo.get_by_id(dish.id).await.unwrap().unwrap();
        assert!(fetched.nutrients.is_some());
        let nutrients = fetched.nutrients.unwrap();
        assert_eq!(nutrients.len(), 2);
    }
}
