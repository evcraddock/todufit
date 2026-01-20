# Meal Log Sync Testing

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

1. Open http://localhost:5173/log/2026-01-10 in browser
2. Create a meal log via CLI:
   ```bash
   make fit ARGS='meal log --date 2026-01-10 --type dinner --notes "CLI test log"'
   make fit-sync
   ```
3. Wait 10 seconds
4. **PASS**: Meal log appears under "Dinner" section (no refresh)
5. **FAIL**: CLI error, meal log does not appear, or any other issue

## CLI → Web Delete (Pass/Fail)

> **BLOCKED**: Requires `fit meal log delete` command (Task #1128)

1. With http://localhost:5173/log/2026-01-10 open in browser
2. Get the log ID and delete via CLI:
   ```bash
   make fit ARGS='meal history --from 2026-01-10 --to 2026-01-10 -f json'
   # Find the log ID from output, then:
   echo y | make fit ARGS='meal log delete "<log-id>"'
   make fit-sync
   ```
3. Wait 10 seconds
4. **PASS**: Meal log disappears from browser (no refresh)
5. **FAIL**: CLI error, meal log does not disappear, or any other issue

## Web → CLI Create (Pass/Fail)

1. Go to http://localhost:5173/log/new?date=2026-01-10&type=breakfast
2. Add notes "Web test log" and save
3. Wait 10 seconds, then sync and verify:
   ```bash
   make fit-sync
   make fit ARGS='meal history --from 2026-01-10 --to 2026-01-10'
   ```
4. **PASS**: "Web test log" appears in CLI output
5. **FAIL**: Form error, CLI error, meal log does not appear, or any other issue

## Web → CLI Delete (Pass/Fail)

> **BLOCKED**: Requires `fit meal log delete` command (Task #1128)

1. Delete the web-created meal log via CLI:
   ```bash
   make fit ARGS='meal history --from 2026-01-10 --to 2026-01-10 -f json'
   # Find the log ID for the breakfast entry with "Web test log", then:
   echo y | make fit ARGS='meal log delete "<log-id>"'
   make fit-sync
   ```
2. Wait 5 seconds, then verify in browser at http://localhost:5173/log/2026-01-10
3. **PASS**: "Web test log" no longer appears in browser or CLI
4. **FAIL**: CLI error, meal log still appears, or any other issue
