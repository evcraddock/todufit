# Changelog

All notable changes to this project will be documented in this file.

## [0.6.0] - 2025-12-30

### Changed

- **Automerge as source of truth**: All CLI commands now write to Automerge documents first, then project to SQLite. This is the foundation for offline-first sync.
  - Dish commands (create, update, delete, add-ingredient, remove-ingredient)
  - MealPlan commands (create, update, delete, add-dish, remove-dish)
  - Meal commands (log)

### Added

- `SyncDishRepository` - Sync-aware repository for dishes
- `SyncMealPlanRepository` - Sync-aware repository for meal plans
- `SyncMealLogRepository` - Sync-aware repository for meal logs
- Automerge documents persist to `~/.local/share/todufit/`:
  - `dishes.automerge`
  - `mealplans.automerge`
  - `meallogs.automerge`

## [0.5.0] - 2025-12-30

### Added

- Automerge sync infrastructure (schema, storage, projection, writer modules)
- Document storage for persisting Automerge docs to disk

## [0.4.0] - 2025-12-29

### Added

- Meal logging with `meal log` command
- Meal history with `meal history` command
- Nutrient tracking for dishes and daily totals

## [0.3.0] - 2025-12-28

### Added

- Meal plan management (create, list, show, update, delete)
- Add/remove dishes from meal plans

## [0.2.0] - 2025-12-27

### Added

- Dish management (create, list, show, update, delete)
- Ingredient management (add, remove)
- Tag support for dishes

## [0.1.0] - 2025-12-26

### Added

- Initial project structure
- SQLite database with migrations
- Configuration system
