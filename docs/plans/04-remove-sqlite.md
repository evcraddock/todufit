# Step 4: Remove SQLite Projection

## Goal

Remove SQLite projection layer. Query Automerge documents directly.

## What to Remove

- `todu-fit-cli/src/db/` - entire directory
- `todu-fit-cli/src/sync/projection.rs`
- `todu-fit-cli/migrations/` - SQL migrations
- SQLite dependencies from `Cargo.toml`

## What to Add

In-memory query helpers:

```rust
impl DishesDocument {
    fn get_by_id(&self, id: Uuid) -> Option<&Dish>;
    fn get_by_name(&self, name: &str) -> Option<&Dish>;
    fn list(&self) -> Vec<&Dish>;
    fn filter_by_tag(&self, tag: &str) -> Vec<&Dish>;
    fn filter_by_ingredient(&self, ingredient: &str) -> Vec<&Dish>;
    fn search(&self, query: &str) -> Vec<&Dish>;
}

impl MealPlansDocument {
    fn get_by_id(&self, id: Uuid) -> Option<&MealPlan>;
    fn list_by_date_range(&self, from: Date, to: Date) -> Vec<&MealPlan>;
    fn get_by_date_and_type(&self, date: Date, meal_type: MealType) -> Option<&MealPlan>;
}

impl MealLogsDocument {
    fn get_by_id(&self, id: Uuid) -> Option<&MealLog>;
    fn list_by_date_range(&self, from: Date, to: Date) -> Vec<&MealLog>;
}
```

## Tasks

- [ ] Add query methods to document types
- [ ] Update commands to use in-memory queries
- [ ] Remove projection calls from sync repositories
- [ ] Remove `db/` module
- [ ] Remove `projection.rs`
- [ ] Remove migrations
- [ ] Remove `sqlx` from dependencies
- [ ] Run tests, fix breakages

## Done When

- No SQLite code remains
- All queries work against in-memory Automerge data
- Tests pass
