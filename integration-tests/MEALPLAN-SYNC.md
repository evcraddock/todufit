# Meal Plan Sync Testing

Pass/fail test for bidirectional sync between CLI and web.

## Rules

- If any step fails or errors, mark the test as FAIL immediately and move on
- Do not troubleshoot, retry, or attempt fixes
- Do not refresh the page unless the test explicitly says to

## Prerequisites

- Environment running (`make status` shows services running)
- Logged into web app (use web-login skill)
- CLI and web sharing identity (use sync-setup skill)

If any prerequisite fails, mark ALL tests in this file as FAIL and move on.

## CLI → Web Create (Pass/Fail)

1. Open http://localhost:5173/meals/today in browser
2. Create a meal plan via CLI (use today's date in YYYY-MM-DD format):
   ```bash
   make fit ARGS='mealplan create --date <today> --type lunch --title "CLI Test Lunch"'
   make fit-sync
   ```
3. Wait 10 seconds
4. **PASS**: Meal plan appears under "Lunch" section (no refresh)
5. **FAIL**: CLI error, meal plan does not appear, or any other issue

## CLI → Web Delete (Pass/Fail)

1. With http://localhost:5173/meals/today open in browser
2. Get the plan ID and delete via CLI:
   ```bash
   make fit ARGS='mealplan list --from <today> --to <today>'
   # Find the plan ID from output, then:
   echo y | make fit ARGS='mealplan delete "<plan-id>"'
   make fit-sync
   ```
3. Wait 10 seconds
4. **PASS**: Meal plan disappears from browser (no refresh)
5. **FAIL**: CLI error, meal plan does not disappear, or any other issue

## Web → CLI Create (Pass/Fail)

1. Go to http://localhost:5173/meals/plan/new?date=today&type=dinner
2. Fill in title "Web Test Dinner" and save
3. Wait 10 seconds, then sync and verify:
   ```bash
   make fit-sync
   make fit ARGS='mealplan list --from <today> --to <today>'
   ```
4. **PASS**: "Web Test Dinner" appears in CLI output
5. **FAIL**: Form error, CLI error, meal plan does not appear, or any other issue

## Web → CLI Delete (Pass/Fail)

1. Delete the web-created meal plan via CLI:
   ```bash
   make fit ARGS='mealplan list --from <today> --to <today>'
   # Find the plan ID for "Web Test Dinner", then:
   echo y | make fit ARGS='mealplan delete "<plan-id>"'
   make fit-sync
   ```
2. Wait 5 seconds, then verify in browser at http://localhost:5173/meals/today
3. **PASS**: "Web Test Dinner" no longer appears in browser or CLI
4. **FAIL**: CLI error, meal plan still appears, or any other issue
