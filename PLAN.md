# Todufit Implementation Plan

## Overview
Local-first meal planning CLI with Automerge sync.

---

## Data Models

### Dish
- id: uuid
- name: string
- ingredients: List[Ingredient]
- instructions: string
- nutrients: Optional[List[Nutrient]]
- prep_time: Optional[int] (minutes)
- cook_time: Optional[int] (minutes)
- servings: Optional[int]
- tags: List[string] (e.g., "breakfast", "quick", "vegetarian")
- image_url: Optional[string]
- source_url: Optional[string]
- created_by: user_id
- created_at: datetime
- updated_at: datetime

### Ingredient
- name: string
- quantity: float
- unit: string

### Nutrient
- name: string (protein, carbs, fat, etc.)
- amount: float
- unit: string (g, mg, kcal)

### MealPlan
- id: uuid
- date: datetime
- meal_type: MealType
- title: string
- cook: string (who's cooking)
- dishes: List[Dish]
- created_by: user_id
- created_at: datetime
- updated_at: datetime

### MealType (enum)
- breakfast
- lunch
- dinner
- snack

### MealLog
- id: uuid
- date: datetime
- meal_type: MealType
- mealplan_id: Optional (if from a plan)
- dishes: List[Dish] (could differ from plan)
- notes: Optional[string]
- created_by: user_id
- created_at: datetime

---

## CLI Commands

### Dish
```bash
todufit dish create "Chicken Soup"
todufit dish list
todufit dish show <id|name>
todufit dish update <id|name>
todufit dish delete <id|name>
todufit dish add-ingredient <id|name> --name "chicken" --quantity 2 --unit "lbs"
todufit dish remove-ingredient <id|name> --name "chicken"
```

### MealPlan
```bash
todufit mealplan create --date 2025-01-01 --type dinner --title "Sunday Dinner"
todufit mealplan list
todufit mealplan show <id|date>
todufit mealplan update <id>
todufit mealplan delete <id>
todufit mealplan add-dish <plan-id> <dish-id>
todufit mealplan remove-dish <plan-id> <dish-id>
```

### Sync
```bash
todufit sync push
todufit sync pull
todufit sync status
```

### Meal (logging)
```bash
# Mark a planned meal as eaten
todufit meal log <mealplan-id>

# Log an unplanned meal
todufit meal log --date 2025-01-01 --type dinner --dish "Pizza"

# View meal history
todufit meal history
todufit meal history --from 2025-01-01 --to 2025-01-07
```

### Config
```bash
todufit config init
todufit config show
```

---

## Sync Protocol

### Document Structure
One Automerge document per entity type:
- `dishes.automerge` - all dishes
- `mealplans.automerge` - all meal plans
- `meallogs.automerge` - all meal logs

### Local Storage
```
~/.todufit/
  data/
    dishes.automerge
    mealplans.automerge
    meallogs.automerge
  todufit.db          # SQLite projection
  config.yaml
```

### Sync Flow
1. CLI stores Automerge docs locally
2. On `sync push/pull`, exchange changes with server via WebSocket
3. Server stores merged docs, relays to other clients
4. CLI projects Automerge changes to SQLite for querying

### Server Auth
- API key/token per user
- Server verifies user belongs to group
- All users in group share same document set

---

## Milestones

### Milestone 1: Local CLI (no sync)
- [ ] Project setup (Cargo, clap, sqlx)
- [ ] Dish CRUD commands
- [ ] SQLite storage
- [ ] Basic queries (list, show, search)

### Milestone 2: MealPlan + MealLog
- [ ] MealPlan CRUD commands
- [ ] MealLog CRUD commands
- [ ] Dish relationships (add/remove dish from plan)
- [ ] Date-based queries

### Milestone 3: Automerge integration
- [ ] Add automerge-rs
- [ ] Automerge docs as source of truth
- [ ] SQLite as projection layer
- [ ] Local sync between docs and SQLite

### Milestone 4: Sync server
- [ ] Rust server (axum + WebSocket)
- [ ] Auth (API key/token)
- [ ] Group-based document access
- [ ] Push/pull sync commands
- [ ] Multi-device sync working
