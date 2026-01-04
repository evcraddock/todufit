# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Changed

- **BREAKING: Updated sync protocol** - Now uses automerge-repo WebSocket protocol
  - Single WebSocket connection for all document types (was one per type)
  - CBOR-encoded messages with handshake (join/peer) before sync
  - Connects to `/sync?key=xxx` (was `/sync/:doc_type?key=xxx`)
  - Requires todu-sync v0.10.0 or later

### Added

- Fetch user/group identity from server via `/me` endpoint
- Deterministic document ID generation matching server algorithm
- `Hash` derive for `DocType` enum

### Dependencies

- Added `ciborium` for CBOR encoding
- Added `bs58` for base58 encoding
- Added `sha2` for document ID hashing
- Added `serde_bytes` for binary data serialization
- Added `reqwest` for HTTP requests

## [0.9.0] - 2026-01-01

### Changed

- **Sync server extracted** - The sync server has been moved to a standalone repository:
  [todu-sync](https://github.com/evcraddock/todu-sync). This makes the server
  reusable by other applications and simplifies todufit's dependencies.

### Removed

- `todufit-server` binary (now `todu-sync` in separate repo)
- `todufit-admin` binary (now `todu-sync-admin` in separate repo)
- Server-only dependencies: `lettre`, `sha2`, `tower`, `tower-http`

### Migration

If you were running the embedded sync server, switch to [todu-sync](https://github.com/evcraddock/todu-sync):

```bash
# Install todu-sync
cargo install --git https://github.com/evcraddock/todu-sync

# Run the server
todu-sync
```

Your existing `users.automerge` and data files are compatible.

## [0.8.0] - 2026-01-01

### Added

- **Magic link authentication** - Secure email-based login for CLI users
  - `todufit auth login` - Request magic link via email
  - `todufit auth logout` - Remove API key from config
  - `todufit auth status` - Show authentication status

## [0.7.0] - 2025-12-31

### Added

- **Sync CLI** - `todufit sync` command for bidirectional sync
  - `todufit sync` - Sync all data with server
  - `todufit sync status` - Show sync configuration
- **Auto-sync** - Optional automatic sync after every write (`auto_sync: true`)

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
- Automerge documents persist to `~/.local/share/fit/`:
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
