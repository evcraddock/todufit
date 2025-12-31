mod dish_repo;
mod mealplan_repo;

pub use dish_repo::DishRepository;
#[allow(unused_imports)]
pub use mealplan_repo::MealPlanRepository;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;
use std::str::FromStr;

/// Initialize the database connection pool and run migrations
pub async fn init_db(db_path: Option<PathBuf>) -> Result<SqlitePool, sqlx::Error> {
    let path = db_path.expect("database_path must be provided");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }

    let db_url = format!("sqlite:{}?mode=rwc", path.display());

    let options = SqliteConnectOptions::from_str(&db_url)?
        .foreign_keys(true)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_init_db_creates_tables() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = init_db(Some(db_path)).await.unwrap();

        // Verify tables exist
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let table_names: Vec<&str> = tables.iter().map(|t| t.0.as_str()).collect();
        assert!(table_names.contains(&"dishes"));
        assert!(table_names.contains(&"ingredients"));
        assert!(table_names.contains(&"nutrients"));
    }
}
