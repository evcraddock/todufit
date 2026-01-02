-- MealLog schema
-- A meallog represents what was actually eaten (vs mealplan which is planned)

CREATE TABLE meallogs (
    id TEXT PRIMARY KEY,  -- UUID as text
    date TEXT NOT NULL,   -- ISO date (YYYY-MM-DD)
    meal_type TEXT NOT NULL CHECK(meal_type IN ('breakfast', 'lunch', 'dinner', 'snack')),
    mealplan_id TEXT REFERENCES mealplans(id) ON DELETE SET NULL,  -- nullable, links to planned meal
    notes TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Junction table for many-to-many relationship between meallogs and dishes
CREATE TABLE meallog_dishes (
    meallog_id TEXT NOT NULL REFERENCES meallogs(id) ON DELETE CASCADE,
    dish_id TEXT NOT NULL REFERENCES dishes(id) ON DELETE CASCADE,
    PRIMARY KEY (meallog_id, dish_id)
);

-- Indexes for common queries
CREATE INDEX idx_meallogs_date ON meallogs(date);
CREATE INDEX idx_meallogs_meal_type ON meallogs(meal_type);
CREATE INDEX idx_meallogs_mealplan_id ON meallogs(mealplan_id);
CREATE INDEX idx_meallog_dishes_dish_id ON meallog_dishes(dish_id);
