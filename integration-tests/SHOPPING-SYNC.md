# Shopping Cart Sync Testing

Pass/fail test for bidirectional sync of shopping cart between CLI and web.

## Rules

- If any step fails or errors, mark the test as FAIL immediately and move on
- Do not troubleshoot, retry, or attempt fixes
- Do not refresh the page unless the test explicitly says to

## Prerequisites

- Environment running (`make status` shows services running)
- Logged into web app (use web-login skill)
- CLI and web sharing identity (use sync-setup skill)

If any prerequisite fails, mark ALL tests in this file as FAIL and move on.

## Setup: Create Test Data

Before running tests, create a dish with ingredients and a meal plan:

```bash
# 1. Create a dish
make fit-dish-create NAME="Shopping Test Pasta"

# 2. Add ingredients to the dish
make fit ARGS='dish add-ingredient "Shopping Test Pasta" --name "Penne pasta" --quantity "1" --unit "lb"'
make fit ARGS='dish add-ingredient "Shopping Test Pasta" --name "Marinara sauce" --quantity "24" --unit "oz"'
make fit ARGS='dish add-ingredient "Shopping Test Pasta" --name "Parmesan cheese" --quantity "0.5" --unit "cup"'

# 3. Get Monday of current week (for meal plan date)
MONDAY=$(date -d "monday this week" +%Y-%m-%d 2>/dev/null || date -v-mon +%Y-%m-%d)
echo "Using Monday: $MONDAY"

# 4. Create a meal plan for Monday dinner
make fit ARGS="mealplan create --date $MONDAY --type dinner --title 'Pasta Night'"

# 5. Add the dish to the meal plan (get plan ID first)
PLAN_ID=$(make fit ARGS="mealplan list --from $MONDAY --to $MONDAY -f json" 2>&1 | grep -o '"id": *"[^"]*"' | head -1 | sed 's/"id": *"\([^"]*\)"/\1/')
make fit ARGS="mealplan add-dish $PLAN_ID \"Shopping Test Pasta\""

# 6. Sync
make fit-sync
```

## Ingredient Auto-Population (Pass/Fail)

1. Open http://localhost:5173/meals in browser
2. Click "Shopping Cart" tab (or expand if collapsed)
3. **PASS**: Shopping cart shows "Penne pasta", "Marinara sauce", and "Parmesan cheese" with quantities
4. **FAIL**: Ingredients don't appear or quantities are wrong

## CLI → Web Manual Item (Pass/Fail)

1. With http://localhost:5173/meals open and Shopping Cart tab visible
2. Add a manual item via CLI:
   ```bash
   MONDAY=$(date -d "monday this week" +%Y-%m-%d 2>/dev/null || date -v-mon +%Y-%m-%d)
   make fit ARGS="shopping add 'Garlic bread' -q 1 -u loaf -w $MONDAY"
   make fit-sync
   ```
3. Wait 10 seconds
4. **PASS**: "Garlic bread" appears in shopping cart (no refresh)
5. **FAIL**: CLI error, item does not appear, or any other issue

## Web → CLI Manual Item (Pass/Fail)

1. With http://localhost:5173/meals open and Shopping Cart tab visible
2. In the "Add item..." form:
   - Enter "Olive oil" in the name field
   - Enter "1" in Qty field
   - Enter "bottle" in Unit field
   - Click "Add"
3. Wait 10 seconds, then sync and verify:
   ```bash
   MONDAY=$(date -d "monday this week" +%Y-%m-%d 2>/dev/null || date -v-mon +%Y-%m-%d)
   make fit-sync
   make fit ARGS="shopping list -w $MONDAY"
   ```
4. **PASS**: "Olive oil" appears in CLI output
5. **FAIL**: Form error, CLI error, item does not appear, or any other issue

## CLI → Web Check Item (Pass/Fail)

1. With http://localhost:5173/meals open and Shopping Cart tab visible
2. Check an item via CLI:
   ```bash
   MONDAY=$(date -d "monday this week" +%Y-%m-%d 2>/dev/null || date -v-mon +%Y-%m-%d)
   make fit ARGS="shopping check 'Penne pasta' -w $MONDAY"
   make fit-sync
   ```
3. Wait 10 seconds
4. **PASS**: "Penne pasta" shows as checked (strikethrough) in web (no refresh)
5. **FAIL**: CLI error, item not checked, or any other issue

## Web → CLI Check Item (Pass/Fail)

1. With http://localhost:5173/meals open and Shopping Cart tab visible
2. Click the checkbox next to "Marinara sauce" to check it
3. Wait 10 seconds, then sync and verify:
   ```bash
   MONDAY=$(date -d "monday this week" +%Y-%m-%d 2>/dev/null || date -v-mon +%Y-%m-%d)
   make fit-sync
   make fit ARGS="shopping list -w $MONDAY -f json"
   ```
4. **PASS**: "Marinara sauce" shows `checked: true` in CLI JSON output
5. **FAIL**: Click didn't work, CLI error, item not checked, or any other issue

## CLI → Web Remove Manual Item (Pass/Fail)

1. With http://localhost:5173/meals open and Shopping Cart tab visible
2. Remove the CLI-added manual item:
   ```bash
   MONDAY=$(date -d "monday this week" +%Y-%m-%d 2>/dev/null || date -v-mon +%Y-%m-%d)
   make fit ARGS="shopping remove 'Garlic bread' -w $MONDAY"
   make fit-sync
   ```
3. Wait 10 seconds
4. **PASS**: "Garlic bread" disappears from shopping cart (no refresh)
5. **FAIL**: CLI error, item still appears, or any other issue

## Web → CLI Remove Manual Item (Pass/Fail)

1. With http://localhost:5173/meals open and Shopping Cart tab visible
2. Click the ✕ button next to "Olive oil" to remove it
3. Wait 10 seconds, then sync and verify:
   ```bash
   MONDAY=$(date -d "monday this week" +%Y-%m-%d 2>/dev/null || date -v-mon +%Y-%m-%d)
   make fit-sync
   make fit ARGS="shopping list -w $MONDAY"
   ```
4. **PASS**: "Olive oil" no longer appears in CLI output
5. **FAIL**: Button didn't work, CLI error, item still appears, or any other issue

## Cleanup

After tests, clean up test data:

```bash
# Delete the meal plan (this removes dish association)
MONDAY=$(date -d "monday this week" +%Y-%m-%d 2>/dev/null || date -v-mon +%Y-%m-%d)
PLAN_ID=$(make fit ARGS="mealplan list --from $MONDAY --to $MONDAY -f json" 2>&1 | grep -o '"id": *"[^"]*"' | head -1 | sed 's/"id": *"\([^"]*\)"/\1/')
echo y | make fit ARGS="mealplan delete $PLAN_ID"

# Delete the test dish
echo y | make fit ARGS='dish delete "Shopping Test Pasta"'

# Sync
make fit-sync
```
