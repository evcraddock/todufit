# todufit

Local-first meal planning and nutrition tracking CLI.

## Features

- **Dish management** - Create and organize recipes with ingredients, instructions, and nutrition info
- **Meal planning** - Plan meals by date and meal type (breakfast, lunch, dinner, snack)
- **Meal logging** - Track what you actually ate, from plans or unplanned meals
- **Nutrition tracking** - View per-meal and daily nutrient totals (calories, protein, carbs, fat)

## Installation

```bash
# Clone and build
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

Config file location: `~/.config/todufit/config.yaml`

```yaml
database_path: ~/.config/todufit/todufit.db
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

```bash
cargo build          # Build
cargo test           # Run tests
cargo run -- <cmd>   # Run locally
```

## Roadmap

- [ ] Cross-device sync via Automerge
- [ ] Ingredient-based shopping lists
- [ ] Meal plan templates

## License

MIT
