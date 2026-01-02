-- Initial schema for todufit
-- Creates dishes, ingredients, and nutrients tables

CREATE TABLE dishes (
    id TEXT PRIMARY KEY,  -- UUID as text
    name TEXT NOT NULL,
    instructions TEXT NOT NULL DEFAULT '',
    prep_time INTEGER,  -- minutes, nullable
    cook_time INTEGER,  -- minutes, nullable
    servings INTEGER,   -- nullable
    tags TEXT NOT NULL DEFAULT '[]',  -- JSON array
    image_url TEXT,
    source_url TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE ingredients (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dish_id TEXT NOT NULL REFERENCES dishes(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    quantity REAL NOT NULL,
    unit TEXT NOT NULL
);

CREATE TABLE nutrients (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dish_id TEXT NOT NULL REFERENCES dishes(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    amount REAL NOT NULL,
    unit TEXT NOT NULL
);

-- Indexes for common queries
CREATE INDEX idx_ingredients_dish_id ON ingredients(dish_id);
CREATE INDEX idx_nutrients_dish_id ON nutrients(dish_id);
CREATE INDEX idx_dishes_name ON dishes(name);
CREATE INDEX idx_dishes_created_by ON dishes(created_by);
