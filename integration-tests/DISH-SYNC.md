# Dish Sync Testing

Pass/fail test for bidirectional sync between CLI and web.

## Rules

- If any step fails or errors, mark the test as FAIL immediately and move on
- Do not troubleshoot, retry, or attempt fixes
- Do not refresh the page unless the test explicitly says to

## Prerequisites

- Environment running (`make status` shows services running)
- CLI and web sharing identity (use sync-setup skill, which includes login)

If any prerequisite fails, mark ALL tests in this file as FAIL and move on.

## CLI → Web Create (Pass/Fail)

1. Open http://localhost:5173/dishes in browser
2. Create a dish via CLI:
   ```bash
   make fit-dish-create NAME="CLI Test Dish"
   make fit-sync
   ```
3. Wait 10 seconds
4. **PASS**: Dish appears in browser (no refresh)
5. **FAIL**: CLI error, dish does not appear, or any other issue

## CLI → Web Delete (Pass/Fail)

1. With http://localhost:5173/dishes open in browser
2. Delete the dish via CLI:
   ```bash
   echo y | make fit ARGS='dish delete "CLI Test Dish"'
   make fit-sync
   ```
3. Wait 10 seconds
4. **PASS**: Dish disappears from browser (no refresh)
5. **FAIL**: CLI error, dish does not disappear, or any other issue

## Web → CLI Create (Pass/Fail)

1. Create dish at http://localhost:5173/dishes/new (name: "Web Test Dish")
2. Wait 10 seconds, then sync and verify:
   ```bash
   make fit-sync
   make fit-dishes
   ```
3. **PASS**: "Web Test Dish" appears in CLI output
4. **FAIL**: Form error, CLI error, dish does not appear, or any other issue

## Web → CLI Delete (Pass/Fail)

1. Delete the web-created dish via CLI:
   ```bash
   echo y | make fit ARGS='dish delete "Web Test Dish"'
   make fit-sync
   ```
2. Wait 5 seconds, then verify in browser at http://localhost:5173/dishes
3. **PASS**: "Web Test Dish" no longer appears in browser or CLI
4. **FAIL**: CLI error, dish still appears, or any other issue
