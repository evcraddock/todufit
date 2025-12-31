# todufit

Local-first meal planning and nutrition tracking CLI.

## Features

- **Dish management** - Create and organize recipes with ingredients, instructions, and nutrition info
- **Meal planning** - Plan meals by date and meal type (breakfast, lunch, dinner, snack)
- **Meal logging** - Track what you actually ate, from plans or unplanned meals
- **Nutrition tracking** - View per-meal and daily nutrient totals (calories, protein, carbs, fat)

## Installation

### Quick Install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/evcraddock/todufit/main/install.sh | bash
```

### Download Binary

Pre-built binaries are available on the [releases page](https://github.com/evcraddock/todufit/releases):
- Linux x86_64
- macOS x86_64 (Intel)
- macOS aarch64 (Apple Silicon)
- Windows x86_64

### Build from Source

```bash
git clone https://github.com/evcraddock/todufit.git
cd todufit
cargo install --path .
```

## Quick Start

```bash
# Create a dish with nutrition info
todufit dish create "Grilled Salmon" \
  --servings 2 \
  --nutrients '{"calories": 450, "protein": 40, "carbs": 5, "fat": 28}'

# Plan a meal
todufit mealplan create --date 2025-01-01 --type dinner --title "New Year Dinner" --dish "Grilled Salmon"

# Log a meal from a plan
todufit meal log <plan-id>

# Or log an unplanned meal
todufit meal log --date 2025-01-01 --type lunch --dish "Grilled Salmon" --notes "Quick lunch"

# View meal history with nutrition totals
todufit meal history
```

## Commands

### Dishes

```bash
todufit dish create <name> [options]    # Create a dish
todufit dish list                       # List all dishes
todufit dish show <name|id>             # Show dish details
todufit dish update <name|id> [options] # Update a dish
todufit dish delete <name|id>           # Delete a dish
todufit dish add-ingredient <name|id> --name <ing> --quantity <n> --unit <u>
todufit dish remove-ingredient <name|id> --name <ing>
```

**Nutrients** are passed as JSON:
```bash
--nutrients '{"calories": 650, "protein": 25, "carbs": 80, "fat": 28}'
```
Units: kcal for calories, grams for everything else.

### Meal Plans

```bash
todufit mealplan create --date <YYYY-MM-DD> --type <type> [--title] [--dish <name>]...
todufit mealplan list [--from <date>] [--to <date>]
todufit mealplan show <id|date> [--type <type>]
todufit mealplan update <id> [options]
todufit mealplan delete <id>
todufit mealplan add-dish <plan-id> <dish>
todufit mealplan remove-dish <plan-id> <dish>
```

### Meal Logging

```bash
# Log from an existing plan
todufit meal log <plan-id> [--notes <text>]

# Log an unplanned meal
todufit meal log --date <YYYY-MM-DD> --type <type> [--dish <name>]... [--notes <text>]

# View history (default: last 7 days)
todufit meal history [--from <date>] [--to <date>] [--format text|json]
```

### Configuration

```bash
todufit config show    # Show current config
```

**Config file locations (platform-specific):**
- Linux: `~/.config/todufit/config.yaml`
- macOS: `~/Library/Application Support/todufit/config.yaml`
- Windows: `%APPDATA%\todufit\config.yaml`

**Data directory (database):**
- Linux: `~/.local/share/todufit/`
- macOS: `~/Library/Application Support/todufit/`
- Windows: `%APPDATA%\todufit\`

```yaml
# config.yaml
database_path: /custom/path/todufit.db  # optional, overrides default
created_by: your-name
```

## Example Workflow

```bash
# Set up some dishes
todufit dish create "Overnight Oats" --servings 1 \
  --nutrients '{"calories": 350, "protein": 12, "carbs": 55, "fat": 10}'

todufit dish create "Chicken Salad" --servings 1 \
  --nutrients '{"calories": 400, "protein": 35, "carbs": 15, "fat": 22}'

# Plan tomorrow's meals
todufit mealplan create --date 2025-01-02 --type breakfast --dish "Overnight Oats"
todufit mealplan create --date 2025-01-02 --type lunch --dish "Chicken Salad"

# Next day: log what you ate
todufit meal log <breakfast-plan-id>
todufit meal log <lunch-plan-id> --notes "Added extra dressing"

# Check your nutrition
todufit meal history --from 2025-01-02 --to 2025-01-02
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

### Quick Commands

```bash
make build           # Build debug binary
make test            # Run tests
make fmt             # Format code
make lint            # Run clippy + format check
make run ARGS="..."  # Run CLI with arguments
```

### Development Environment with Sync Server

For testing sync functionality, you can run the sync server locally:

1. **Set up environment:**
   ```bash
   cp .env.example .env
   cp config/server.yaml.example config/server.yaml
   ```

2. **Configure API key** in `config/server.yaml`:
   ```yaml
   api_keys:
     - key: "your-dev-api-key"
       user_id: "dev"
       group_id: "default"
   ```

3. **Start the development environment:**
   ```bash
   make dev
   ```
   This starts the sync server on the configured port (default: 8080).

4. **Other dev commands:**
   ```bash
   make dev-stop    # Stop the development environment
   make dev-logs    # Tail development logs
   ```

5. **Configure CLI for sync** in `~/.config/todufit/config.yaml`:
   ```yaml
   sync:
     server_url: "ws://localhost:8080"
     api_key: "your-dev-api-key"
     auto_sync: true  # optional: sync after every write
   ```

   Or use environment variables:
   ```bash
   export TODUFIT_SYNC_URL=ws://localhost:8080
   export TODUFIT_SYNC_API_KEY=your-dev-api-key
   ```

6. **Sync commands:**
   ```bash
   todufit sync          # Sync all data with server
   todufit sync status   # Show sync configuration
   ```

   With `auto_sync: true`, changes sync automatically after every write.

## Roadmap

- [x] Cross-device sync via Automerge
- [ ] Ingredient-based shopping lists
- [ ] Meal plan templates

## License

MIT
