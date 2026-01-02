# Data Model

## Overview

Todufit uses Automerge CRDTs as the source of truth, with SQLite as a projection layer for fast queries. This document describes the entity schemas and how they're represented in both systems.

## Entities

### Dish

A recipe with optional nutrition information.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | UUID | Yes | Unique identifier |
| name | String | Yes | Dish name |
| ingredients | [Ingredient] | No | List of ingredients |
| instructions | String | No | Cooking instructions |
| nutrients | [Nutrient] | No | Nutrition per serving |
| prep_time | Integer | No | Prep time in minutes |
| cook_time | Integer | No | Cook time in minutes |
| servings | Integer | No | Number of servings |
| tags | [String] | No | Categorization tags |
| image_url | String | No | URL to dish image |
| source_url | String | No | Recipe source URL |
| created_by | String | Yes | User ID who created |
| created_at | DateTime | Yes | Creation timestamp |
| updated_at | DateTime | Yes | Last update timestamp |

### Ingredient

A component of a dish.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | String | Yes | Ingredient name |
| quantity | Float | Yes | Amount |
| unit | String | Yes | Unit of measurement |

### Nutrient

Nutritional information for a dish (per serving).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | String | Yes | Nutrient name (calories, protein, etc.) |
| amount | Float | Yes | Amount |
| unit | String | Yes | Unit (kcal, g, mg) |

**Standard nutrients:**
- `calories` (kcal)
- `protein` (g)
- `carbs` (g)
- `fat` (g)

### MealPlan

A planned meal for a specific date and time.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | UUID | Yes | Unique identifier |
| date | Date | Yes | Planned date (YYYY-MM-DD) |
| meal_type | MealType | Yes | Type of meal |
| title | String | No | Optional title |
| cook | String | No | Who's cooking |
| dishes | [DishReference] | No | Dishes in this meal |
| created_by | String | Yes | User ID who created |
| created_at | DateTime | Yes | Creation timestamp |
| updated_at | DateTime | Yes | Last update timestamp |

### MealLog

A record of an actual meal consumed.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| id | UUID | Yes | Unique identifier |
| date | Date | Yes | Date eaten (YYYY-MM-DD) |
| meal_type | MealType | Yes | Type of meal |
| mealplan_id | UUID | No | Associated meal plan (if from plan) |
| dishes | [DishReference] | No | Dishes actually eaten |
| notes | String | No | Optional notes |
| created_by | String | Yes | User ID who logged |
| created_at | DateTime | Yes | Creation timestamp |

### DishReference

A reference to a dish with serving information.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| dish_id | UUID | Yes | Reference to dish |
| servings | Float | Yes | Number of servings consumed |

### MealType

Enumeration of meal types.

| Value | Description |
|-------|-------------|
| breakfast | Morning meal |
| lunch | Midday meal |
| dinner | Evening meal |
| snack | Between-meal snack |

## Automerge Document Structure

Three separate Automerge documents, each a map of UUID string to entity object.

### dishes.automerge

```json
{
  "550e8400-e29b-41d4-a716-446655440001": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "name": "Grilled Salmon",
    "ingredients": [
      {"name": "salmon fillet", "quantity": 8, "unit": "oz"},
      {"name": "olive oil", "quantity": 1, "unit": "tbsp"},
      {"name": "lemon", "quantity": 1, "unit": "whole"}
    ],
    "instructions": "1. Preheat grill...",
    "nutrients": [
      {"name": "calories", "amount": 450, "unit": "kcal"},
      {"name": "protein", "amount": 40, "unit": "g"},
      {"name": "carbs", "amount": 5, "unit": "g"},
      {"name": "fat", "amount": 28, "unit": "g"}
    ],
    "prep_time": 10,
    "cook_time": 15,
    "servings": 2,
    "tags": ["dinner", "healthy", "quick"],
    "image_url": null,
    "source_url": null,
    "created_by": "usr_abc123",
    "created_at": "2025-01-01T12:00:00Z",
    "updated_at": "2025-01-01T12:00:00Z"
  },
  "550e8400-e29b-41d4-a716-446655440002": {
    // Another dish...
  }
}
```

### mealplans.automerge

```json
{
  "660e8400-e29b-41d4-a716-446655440001": {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "date": "2025-01-15",
    "meal_type": "dinner",
    "title": "Family Dinner",
    "cook": "Erik",
    "dishes": [
      {"dish_id": "550e8400-e29b-41d4-a716-446655440001", "servings": 2}
    ],
    "created_by": "usr_abc123",
    "created_at": "2025-01-10T08:00:00Z",
    "updated_at": "2025-01-10T08:00:00Z"
  }
}
```

### meallogs.automerge

```json
{
  "770e8400-e29b-41d4-a716-446655440001": {
    "id": "770e8400-e29b-41d4-a716-446655440001",
    "date": "2025-01-15",
    "meal_type": "dinner",
    "mealplan_id": "660e8400-e29b-41d4-a716-446655440001",
    "dishes": [
      {"dish_id": "550e8400-e29b-41d4-a716-446655440001", "servings": 1.5}
    ],
    "notes": "Only had 1.5 servings, saved rest for tomorrow",
    "created_by": "usr_abc123",
    "created_at": "2025-01-15T19:30:00Z"
  }
}
```

## SQLite Schema (Projection)

The SQLite database is a queryable projection of Automerge documents. It's rebuilt from Automerge on sync.

```sql
-- Dishes
CREATE TABLE dishes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    instructions TEXT NOT NULL DEFAULT '',
    prep_time INTEGER,
    cook_time INTEGER,
    servings INTEGER,
    tags TEXT NOT NULL DEFAULT '[]',  -- JSON array
    image_url TEXT,
    source_url TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Ingredients (one-to-many with dishes)
CREATE TABLE ingredients (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dish_id TEXT NOT NULL REFERENCES dishes(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    quantity REAL NOT NULL,
    unit TEXT NOT NULL
);

-- Nutrients (one-to-many with dishes)
CREATE TABLE nutrients (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dish_id TEXT NOT NULL REFERENCES dishes(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    amount REAL NOT NULL,
    unit TEXT NOT NULL
);

-- Meal Plans
CREATE TABLE meal_plans (
    id TEXT PRIMARY KEY,
    date TEXT NOT NULL,           -- YYYY-MM-DD
    meal_type TEXT NOT NULL,      -- breakfast, lunch, dinner, snack
    title TEXT,
    cook TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Meal Plan Dishes (many-to-many)
CREATE TABLE meal_plan_dishes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meal_plan_id TEXT NOT NULL REFERENCES meal_plans(id) ON DELETE CASCADE,
    dish_id TEXT NOT NULL REFERENCES dishes(id),
    servings REAL NOT NULL DEFAULT 1.0
);

-- Meal Logs
CREATE TABLE meal_logs (
    id TEXT PRIMARY KEY,
    date TEXT NOT NULL,           -- YYYY-MM-DD
    meal_type TEXT NOT NULL,
    meal_plan_id TEXT REFERENCES meal_plans(id),
    notes TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Meal Log Dishes (many-to-many)
CREATE TABLE meal_log_dishes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meal_log_id TEXT NOT NULL REFERENCES meal_logs(id) ON DELETE CASCADE,
    dish_id TEXT NOT NULL REFERENCES dishes(id),
    servings REAL NOT NULL DEFAULT 1.0
);

-- Indexes
CREATE INDEX idx_dishes_name ON dishes(name);
CREATE INDEX idx_dishes_created_by ON dishes(created_by);
CREATE INDEX idx_ingredients_dish ON ingredients(dish_id);
CREATE INDEX idx_nutrients_dish ON nutrients(dish_id);
CREATE INDEX idx_meal_plans_date ON meal_plans(date);
CREATE INDEX idx_meal_plans_type ON meal_plans(meal_type);
CREATE INDEX idx_meal_logs_date ON meal_logs(date);
CREATE INDEX idx_meal_logs_plan ON meal_logs(meal_plan_id);
```

## Projection Process

When syncing, the projection layer:

1. Loads Automerge document
2. Clears relevant SQLite tables (within transaction)
3. Iterates over all entities in document
4. Inserts into SQLite tables
5. Commits transaction

```rust
// Pseudocode
async fn project_dishes(doc: &AutoCommit, pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    
    // Clear existing (order matters for foreign keys)
    sqlx::query("DELETE FROM nutrients").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM ingredients").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM dishes").execute(&mut *tx).await?;
    
    // Extract and insert all dishes
    for (id, dish) in extract_dishes(doc)? {
        insert_dish(&mut tx, &dish).await?;
        for ingredient in &dish.ingredients {
            insert_ingredient(&mut tx, &id, ingredient).await?;
        }
        for nutrient in &dish.nutrients {
            insert_nutrient(&mut tx, &id, nutrient).await?;
        }
    }
    
    tx.commit().await?;
    Ok(())
}
```

## Conflict Resolution

Automerge handles conflicts automatically using CRDT semantics:

| Scenario | Resolution |
|----------|------------|
| Same field edited concurrently | Last-writer-wins per field |
| Item added to list | Both items kept, ordered by timestamp |
| Item deleted while edited | Delete wins |
| Different fields edited | Both changes kept |

**Example:** If device A changes `dish.name` while device B changes `dish.servings`, both changes are preserved. If both change `dish.name`, the later timestamp wins.

## Sync Granularity

**Current design:** One document per entity type (dishes, mealplans, meallogs).

**Implications:**
- All dishes sync together
- Efficient for typical usage (hundreds of items)
- May be slow for very large datasets

**Future consideration:** Shard by time if needed:
- `mealplans-2025-01.automerge`
- `meallogs-2025-01.automerge`

Only consider this if performance becomes an issue.

## Data Size Estimates

| Entity | Typical Size | 100 items | 1000 items |
|--------|-------------|-----------|------------|
| Dish | ~1-2 KB | ~100-200 KB | ~1-2 MB |
| MealPlan | ~200-500 B | ~20-50 KB | ~200-500 KB |
| MealLog | ~200-500 B | ~20-50 KB | ~200-500 KB |

With Automerge delta sync, only changes are transmitted after initial sync.

## Cross-Platform Compatibility

All platforms use the same document format:

| Platform | Library | Binary Compatible |
|----------|---------|-------------------|
| CLI (Rust) | automerge-rs | Yes |
| Web (Rust) | automerge-rs | Yes |
| iOS (Swift) | automerge-swift | Yes |

automerge-swift and automerge-rs use the same binary format, ensuring seamless sync across all clients.

## Soft Deletes (Optional Future)

Currently, deletes remove entities from the Automerge document. For audit/recovery, consider soft deletes:

```json
{
  "id": "...",
  "deleted_at": "2025-01-15T10:00:00Z",
  "deleted_by": "usr_abc123",
  // ... other fields preserved
}
```

Query layer would filter out `deleted_at IS NOT NULL`.
