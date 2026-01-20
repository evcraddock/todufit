# AGENTS.md - Monorepo Development Guide

This document explains the todu-fit monorepo for AI agents and developers.

## Overview

todu-fit is a meal planning and nutrition tracking application with:

- **todu-fit-core** - Shared Rust library (models, Automerge sync logic)
- **todu-fit-cli** - Command-line interface (`fit` binary)
- **web/** - React web application with Hono backend

All clients (CLI and web) use Automerge CRDTs for offline-first data storage and WebSocket-based sync.

## Monorepo Structure

```
todu-fit/
├── AGENTS.md              # This file
├── Makefile               # Development commands
├── Procfile               # Overmind service definitions
├── Cargo.toml             # Rust workspace definition
├── compose.yaml           # Docker compose for sync server
├── config.dev.yaml        # CLI dev configuration
├── config.test.yaml       # CLI test configuration
├── .env / .env.test       # Web app environment configs
├── todu-fit-core/         # Shared Rust library
│   └── src/
├── todu-fit-cli/          # CLI application
│   └── src/
├── web/                   # React + Hono web app
│   ├── src/               # React frontend
│   ├── server/            # Hono server modules
│   └── server.ts          # Hono entry point
├── integration-tests/     # Manual sync test procedures
└── data/                  # Local dev/test data (gitignored)
    ├── dev/
    └── test/
```

## Services

| Service | Port | Purpose |
|---------|------|---------|
| **vite** | 5173 | React dev server with HMR |
| **hono** | 3000 | Auth endpoints, API server |
| **sync** | 8080 | Automerge sync server (Docker) |

Both CLI and web connect to the sync server at `ws://localhost:8080`.

## Prerequisites

- **Rust** (latest stable) - for CLI
- **Node.js** (v20+) - for web app
- **Docker** - for sync server
- **Overmind + tmux** - process manager

## Starting the Environment

```bash
# Start dev environment (persistent data)
make dev

# Start test environment (wipes test data first)
make test

# Check status
make status

# Stop everything
make stop
```

## DO NOT CONTROL THE DEV SERVER

**NEVER start, stop, or restart the dev server.** The user controls the server lifecycle.

If you believe the server needs action, **ask the user first**.

Use `make status` to check if services are running.

## CLI Development

### Build First

The CLI must be built before running:

```bash
make build  # Runs: cargo build --release -p todu-fit-cli
```

### Configuration Auto-Detection

The Makefile auto-detects the environment by checking `TODU_ENV` from the running hono server:

- If hono shows `TODU_ENV=test` → uses `config.test.yaml`
- Otherwise → uses `config.dev.yaml`

**Verify which config is active:**
```bash
make fit-config
```

### CLI Config Files

**config.dev.yaml:**
```yaml
data_dir: ./data/dev/cli
sync:
  server_url: ws://localhost:8080
  auto_sync: true
```

**config.test.yaml:**
```yaml
data_dir: ./data/test/cli
sync:
  server_url: ws://localhost:8080
  auto_sync: true
```

### Running CLI Commands

**Via Makefile (uses pre-built binary, auto-detects config):**
```bash
make fit ARGS="dish list"
make fit-dishes
make fit-dish-create NAME="Chicken Salad"
make fit-sync
```

**Via cargo (for development iteration):**
```bash
cargo run -p todu-fit-cli -- -c config.dev.yaml dish list
cargo run -p todu-fit-cli -- -c config.dev.yaml sync
```

**Key difference:**
- `make fit` uses `./target/release/fit` (fast, but requires `make build` first)
- `cargo run` compiles if needed (slower, but always current)

### CLI Command Reference

| Command | Description |
|---------|-------------|
| `make build` | Build CLI release binary |
| `make fit-init` | Initialize CLI identity |
| `make fit-sync` | Sync to local server |
| `make fit-config` | Show config and root_doc_id |
| `make fit-dishes` | List dishes |
| `make fit-dish-create NAME="..."` | Create a dish |
| `make fit ARGS="..."` | Run any fit command |

## Web Development

The web app lives in `web/` and has its own tooling:

```bash
make web-install  # Install npm dependencies
make web-build    # Build for production
make web-lint     # TypeScript type check
```

During development, the web app runs via Overmind (vite + hono services).

## Testing

### Unit Tests

```bash
make test-cli     # Rust tests: cargo test
make web-lint     # TypeScript checks
```

### Integration Tests

Manual test procedures in `integration-tests/`:
- `DISH-SYNC.md` - Dish sync between CLI and web
- `MEALLOG-SYNC.md` - Meal log sync
- `MEALPLAN-SYNC.md` - Meal plan sync
- `SHOPPING-SYNC.md` - Shopping list sync

### CLI + Web Sync Testing

1. Start dev environment: `make dev`
2. Initialize CLI: `make fit-init`
3. Get CLI's root_doc_id: `make fit-config`
4. Login to web app at http://localhost:5173
5. Go to Settings → Sync Settings
6. Change root doc ID to CLI's value
7. Test bidirectional sync:
   ```bash
   make fit-dish-create NAME="Test"
   make fit-sync
   # Verify in web, add item in web
   make fit-sync
   make fit-dishes
   ```

## Viewing Logs

Logs are in tmux panes managed by Overmind:

```bash
make logs          # Stream all logs (Ctrl+C to stop)
make connect-hono  # Attach to hono pane
make connect-vite  # Attach to vite pane
make connect-sync  # Attach to sync pane
```

**In tmux pane:**
- Scroll: `Ctrl+b [`, then arrows/Page Up
- Exit scroll: `q`
- Detach: `Ctrl+b d`

**Programmatic access (for AI agents):**
```bash
TMUX_SOCK=$(ls -t /tmp/tmux-$(id -u)/overmind-todu-fit-* 2>/dev/null | head -1)
tmux -L "$(basename $TMUX_SOCK)" capture-pane -t todu-fit:hono -p -S -50
```

## Data Directories

```
data/
├── dev/                  # Persistent dev data
│   ├── cli/              # CLI Automerge data
│   ├── sync/             # Sync server data
│   └── todu-fit.sqlite   # Web app database
└── test/                 # Wiped by `make test`
    ├── cli/
    ├── sync/
    └── todu-fit.sqlite
```

**Reset data:**
```bash
make reset       # Reset dev data (caution!)
make test-reset  # Reset test data
```

## Code Quality

```bash
make lint        # CLI: cargo fmt --check && cargo clippy
make web-lint    # Web: tsc type check
```

## Environment Variables

Web app uses `.env` (dev) or `.env.test` (test). Key variables:

| Variable | Description |
|----------|-------------|
| `TODU_ENV` | Environment name (dev/test) |
| `PORT` | Hono server port |
| `DATABASE_PATH` | SQLite database path |
| `VITE_SYNC_URL` | Sync server URL for browser |
| `SMTP_DEV_MODE` | Log emails instead of sending |

## Common Issues

### CLI: "No such file" when running `make fit`

Build the CLI first:
```bash
make build
```

### CLI: Wrong config being used

Check which config is active:
```bash
make fit-config
```

Force a specific config:
```bash
make fit ARGS="dish list" CLI_CONFIG=config.dev.yaml
```

### Port already in use

```bash
lsof -i :5173  # vite
lsof -i :3000  # hono
lsof -i :8080  # sync
kill -9 <PID>
```

### Magic link not showing

In dev mode, magic links are logged to hono. Connect and scroll up:
```bash
make connect-hono
# Ctrl+b [ to scroll, find "MAGIC LINK EMAIL"
```
