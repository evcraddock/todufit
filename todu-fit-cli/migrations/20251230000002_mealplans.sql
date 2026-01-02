-- MealPlan schema
-- A mealplan represents a planned meal for a specific date and meal type

CREATE TABLE mealplans (
    id TEXT PRIMARY KEY,  -- UUID as text
    date TEXT NOT NULL,   -- ISO date (YYYY-MM-DD)
    meal_type TEXT NOT NULL CHECK(meal_type IN ('breakfast', 'lunch', 'dinner', 'snack')),
    title TEXT NOT NULL,
    cook TEXT NOT NULL DEFAULT 'Unknown',
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Junction table for many-to-many relationship between mealplans and dishes
CREATE TABLE mealplan_dishes (
    mealplan_id TEXT NOT NULL REFERENCES mealplans(id) ON DELETE CASCADE,
    dish_id TEXT NOT NULL REFERENCES dishes(id) ON DELETE CASCADE,
    PRIMARY KEY (mealplan_id, dish_id)
);

-- Indexes for common queries
CREATE INDEX idx_mealplans_date ON mealplans(date);
CREATE INDEX idx_mealplans_meal_type ON mealplans(meal_type);
CREATE INDEX idx_mealplan_dishes_dish_id ON mealplan_dishes(dish_id);
