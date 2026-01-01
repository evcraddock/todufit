# Changelog

All notable changes to this project will be documented in this file.

## [0.8.0] - 2026-01-01

### Added

- **Magic link authentication** - Secure email-based login for CLI users
  - `todufit auth login` - Request magic link via email
  - `todufit auth logout` - Remove API key from config
  - `todufit auth status` - Show authentication status
- **User management** (`todufit-admin`)
  - Users stored in `users.automerge` file
  - `todufit-admin user add` - Register email with group
  - `todufit-admin user list` - List registered users
  - `todufit-admin user remove` - Remove user
- **Server auth endpoints**
  - `POST /auth/login` - Request magic link
  - `GET /auth/verify` - Verify token and issue API key
- **SMTP email sending** for magic links

### Configuration

New auth config options for server (`~/.config/todufit-server/config.yaml`):
```yaml
auth:
  smtp_host: smtp.example.com
  smtp_port: 587
  smtp_user: noreply@example.com
  smtp_pass: secret
  from_email: noreply@example.com
  from_name: ToduFit
  server_url: https://sync.example.com
  token_expiry_minutes: 10
```

Static API keys still supported for development.

## [0.7.0] - 2025-12-31

### Added

- **Sync server** (`todufit-server`) - WebSocket server for multi-device sync
  - API key authentication with user/group support
  - Server-side Automerge document storage
  - Real-time sync via WebSocket
- **Sync CLI** - `todufit sync` command for bidirectional sync
  - `todufit sync` - Sync all data with server
  - `todufit sync status` - Show sync configuration
- **Auto-sync** - Optional automatic sync after every write (`auto_sync: true`)
- **Dev environment** - `make dev` runs local sync server via Procfile

### Configuration

New sync config options:
```yaml
sync:
  server_url: "ws://localhost:8080"
  api_key: "your-api-key"
  auto_sync: true
```

Environment variables: `TODUFIT_SYNC_URL`, `TODUFIT_SYNC_API_KEY`

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
