# Todu Fit

Local-first meal planning and nutrition tracking CLI.

## Features

- **Dish management** - Create and organize recipes with ingredients, instructions, and nutrition info
- **Meal planning** - Plan meals by date and meal type (breakfast, lunch, dinner, snack)
- **Meal logging** - Track what you actually ate, from plans or unplanned meals
- **Nutrition tracking** - View per-meal and daily nutrient totals (calories, protein, carbs, fat)
- **Shopping lists** - Auto-generated ingredient lists from meal plans with check-off tracking
- **Groups** - Share dishes and meal plans with family or household members
- **Cross-device sync** - Sync data across devices via [todu-sync](https://github.com/evcraddock/todu-sync) server

## Installation

### Quick Install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/evcraddock/todu-fit/main/install.sh | bash
```

### Download Binary

Pre-built binaries are available on the [releases page](https://github.com/evcraddock/todu-fit/releases):
- Linux x86_64
- macOS x86_64 (Intel)
- macOS aarch64 (Apple Silicon)
- Windows x86_64

### Build from Source

```bash
git clone https://github.com/evcraddock/todu-fit.git
cd todu-fit
cargo install --path todu-fit-cli
```

## Quick Start

```bash
# Create a dish with nutrition info
fit dish create "Grilled Salmon" \
  --servings 2 \
  --nutrients '{"calories": 450, "protein": 40, "carbs": 5, "fat": 28}'

# Plan a meal
fit mealplan create --date 2025-01-01 --type dinner --title "New Year Dinner" --dish "Grilled Salmon"

# Log a meal from a plan
fit meal log <plan-id>

# Or log an unplanned meal
fit meal log --date 2025-01-01 --type lunch --dish "Grilled Salmon" --notes "Quick lunch"

# View meal history with nutrition totals
fit meal history
```

## Commands

### Dishes

```bash
fit dish create <name> [options]    # Create a dish
fit dish list                       # List all dishes
fit dish show <name|id>             # Show dish details
fit dish update <name|id> [options] # Update a dish
fit dish delete <name|id>           # Delete a dish
fit dish add-ingredient <name|id> --name <ing> --quantity <n> --unit <u>
fit dish remove-ingredient <name|id> --name <ing>
```

**Nutrients** are passed as JSON:
```bash
--nutrients '{"calories": 650, "protein": 25, "carbs": 80, "fat": 28}'
```
Units: kcal for calories, grams for everything else.

### Meal Plans

```bash
fit mealplan create --date <YYYY-MM-DD> --type <type> [--title] [--dish <name>]...
fit mealplan list [--from <date>] [--to <date>]
fit mealplan show <id|date> [--type <type>]
fit mealplan update <id> [options]
fit mealplan delete <id>
fit mealplan add-dish <plan-id> <dish>
fit mealplan remove-dish <plan-id> <dish>
```

### Meal Logging

```bash
# Log from an existing plan
fit meal log <plan-id> [--notes <text>]

# Log an unplanned meal
fit meal log --date <YYYY-MM-DD> --type <type> [--dish <name>]... [--notes <text>]

# View history (default: last 7 days)
fit meal history [--from <date>] [--to <date>] [--format text|json]
```

### Shopping

Generate shopping lists from your meal plans with automatic ingredient aggregation.

```bash
# List shopping items for the current week
fit shopping list [--week <YYYY-MM-DD>] [--format table|json]

# Add a manual item
fit shopping add <name> [--qty <quantity>] [--unit <unit>] [--week <date>]

# Remove a manual item
fit shopping remove <name> [--week <date>]

# Mark items as purchased
fit shopping check <name> [--week <date>]
fit shopping uncheck <name> [--week <date>]

# Clear all checked items
fit shopping clear-checked [--week <date>]
```

The shopping list automatically aggregates ingredients from all meal plans in the week and deduplicates by name and unit.

### Identity & Groups

Manage your identity and share data across devices and with others.

```bash
# Initialize identity
fit init --new              # Create new identity
fit init --join <doc-id>    # Join existing identity from another device
fit init --force            # Force reset (delete existing data)

# View identity for sharing
fit device show             # Display identity document ID and groups

# Manage groups for shared dishes and meal plans
fit group create <name>     # Create a new group
fit group join <id> [--name <name>]  # Join an existing group
fit group list              # List all groups
fit group show              # Show current group details
fit group switch <name>     # Switch to a different group
fit group leave <name>      # Leave a group
```

### Sync

Cross-device sync is provided by [todu-sync](https://github.com/evcraddock/todu-sync), a standalone Automerge sync server.

```bash
fit sync          # Sync all data with server
fit sync status   # Show sync configuration and server status
```

**Configure sync** in `~/.config/fit/config.yaml`:
```yaml
sync:
  server_url: "ws://your-sync-server:8080"
  auto_sync: true  # optional: sync after every write
```

### Configuration

```bash
fit config show    # Show current config
fit config init    # Initialize config file with defaults
```

**Config file locations (platform-specific):**
- Linux: `~/.config/fit/config.yaml`
- macOS: `~/Library/Application Support/fit/config.yaml`
- Windows: `%APPDATA%\fit\config.yaml`

**Data directory (database):**
- Linux: `~/.local/share/fit/`
- macOS: `~/Library/Application Support/fit/`
- Windows: `%APPDATA%\fit\`

```yaml
# config.yaml
database_path: /custom/path/fit.db  # optional, overrides default
created_by: your-name
```

## Example Workflow

```bash
# Set up some dishes
fit dish create "Overnight Oats" --servings 1 \
  --nutrients '{"calories": 350, "protein": 12, "carbs": 55, "fat": 10}'

fit dish create "Chicken Salad" --servings 1 \
  --nutrients '{"calories": 400, "protein": 35, "carbs": 15, "fat": 22}'

# Plan tomorrow's meals
fit mealplan create --date 2025-01-02 --type breakfast --dish "Overnight Oats"
fit mealplan create --date 2025-01-02 --type lunch --dish "Chicken Salad"

# Next day: log what you ate
fit meal log <breakfast-plan-id>
fit meal log <lunch-plan-id> --notes "Added extra dressing"

# Check your nutrition
fit meal history --from 2025-01-02 --to 2025-01-02
```

Output:
```
2025-01-02
------------------------------------------------------------
  breakfast (planned): Overnight Oats
             Calories: 350 | Protein: 12g | Carbs: 55g | Fat: 10g
  lunch (planned): Chicken Salad
             Calories: 400 | Protein: 35g | Carbs: 15g | Fat: 22g
             Notes: Added extra dressing
  --------------------------------------------------------
  Daily Total: Calories: 750 | Protein: 47g | Carbs: 70g | Fat: 32g

Total: 2 meal(s)
```

## Development

```bash
cargo build                            # Build debug binary
cargo test                             # Run tests
cargo fmt                              # Format code
cargo clippy                           # Run linter
cargo run -p todu-fit-cli -- <args>    # Run CLI with arguments
```

## Architecture

```
┌─────────────────────────────────────────────┐
│                fit CLI                       │
│                                             │
│  ┌─────────────┐      ┌─────────────────┐  │
│  │  Automerge  │─────▶│     SQLite      │  │
│  │   (sync)    │      │   (queries)     │  │
│  └─────────────┘      └─────────────────┘  │
│         │                                   │
└─────────│───────────────────────────────────┘
          │ WebSocket sync
          ▼
   ┌─────────────┐
   │  todu-sync  │  (separate deployment)
   │   server    │
   └─────────────┘
```

- **Automerge** is the source of truth for sync (CRDT)
- **SQLite** is a local projection for fast queries
- **todu-sync** is a standalone sync server (see [todu-sync](https://github.com/evcraddock/todu-sync))

## Roadmap

- [x] Cross-device sync via Automerge
- [x] Ingredient-based shopping lists
- [ ] Meal plan templates

## License

MIT
