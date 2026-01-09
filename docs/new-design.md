# Todu-Fit Redesign

> Working document capturing design decisions.

## Context

Exploring a simpler architecture inspired by **rott** (a local-first links manager). Goals:
- Remove authentication complexity
- Support multiple users (household sharing)
- Work fully offline, sync when available
- Simplify by removing SQLite projection

## Constraints

- Local-first, never SaaS
- Household scale: max ~10 people
- Multi-device per person
- Sync server runs on local network

## Key Decisions

### 1. No Authentication

**Network is the trust boundary.** Sync server runs on local network (home LAN). Anyone on the network can sync. No magic links, API keys, or accounts.

- App works fully offline
- Sync is opportunistic - try to connect, if it fails, keep going
- No "logged in" vs "logged out" states
- Use standard automerge-repo-sync-server (no custom server)

### 2. No SQLite Projection

Query Automerge documents directly in memory. At household scale (100-300 recipes, few thousand mealplans/logs), in-memory filtering is instant.

**Benefits:**
- No projection layer to maintain
- No sync consistency issues between Automerge and SQLite
- Simpler architecture
- Automerge IS the database

**Complex queries (e.g., "dishes with tag keto and ingredient chicken"):**
- Just filter in memory
- 300 dishes × 10 ingredients = microseconds
- Build query helpers in code, not SQL

### 3. Document Ownership Model

Five document types with clear ownership:

| Document | Ownership | Purpose |
|----------|-----------|---------|
| identity | personal | Your root doc. Points to your groups and meallogs |
| group | shared | Group manifest. Points to dishes and mealplans |
| dishes | shared | Recipes owned by the group |
| mealplans | shared | Meal schedule owned by the group |
| meallogs | personal | What you actually ate (private to you) |

**Relationships:**
```
identity (personal)
├── meallogs_doc_id → meallogs (personal)
└── groups[]
    └── group_doc_id → group (shared)
                       ├── dishes_doc_id → dishes (shared)
                       └── mealplans_doc_id → mealplans (shared)
```

### 4. Document ID as Identity

Each person has ONE root ID: their identity doc ID.

- Created once, never changes
- All other doc IDs discovered by syncing the identity doc
- Config file only needs: root_doc_id + sync_server + current_group

### 5. Cross-Document References

**Mealplans → Dishes: Reference by ID (live lookup)**
- Planning needs current info
- If someone edits a recipe, you see the update when planning

**Meallogs → Dishes: Snapshot at log time**
- Historical accuracy matters
- "On Jan 5, I ate Pasta which had 500 calories"
- If recipe changes later, log stays accurate
- Self-contained, works without group docs

```
// mealplan entry - references dish
{
  date: "2026-01-14",
  meal_type: "dinner",
  dish_ids: ["abc-123"]  // lookup from dishes doc
}

// meallog entry - snapshots dish
{
  date: "2026-01-09",
  meal_type: "dinner",
  dishes: [
    {
      source_dish_id: "abc-123",  // for linking back
      name: "Grilled Salmon",
      nutrients: { calories: 450, protein: 40, ... }
    }
  ]
}
```

### 6. Setup Flows

**First device (new user):**
```
fit init --new
```
- Creates identity doc (your root doc ID)
- Creates your meallogs doc
- You can create a new group or join existing

**Additional device (same user):**
```
fit init --join <identity-doc-id>
```
- Only needs ONE ID
- Syncs identity doc → learns everything else
- Syncs meallogs, group docs, dishes, mealplans
- If sync server unavailable, shows error and works offline

**Joining a group:**
```
fit group join <group-doc-id>
```
- Adds group to your identity doc
- Sync discovers dishes + mealplans doc IDs from group doc

**Creating a new group:**
```
fit group create "family"
```
- Creates group doc, dishes doc, mealplans doc
- Adds group to your identity doc
- Share the group doc ID with others

### 7. CLI Structure

**Current group context:**
- Stored in config file (device-local, not synced)
- Auto-selected if only one group
- Override with `--group` flag

```
fit group list                    # show all groups
fit group switch <name>           # set current group
fit group show                    # show current group details
```

**Dish commands (operate on current group):**
```
fit dish create "Grilled Salmon" --tag healthy --tag quick
fit dish list
fit dish list --tag keto --ingredient chicken
fit dish show <id|name>
fit dish edit <id|name>
fit dish delete <id|name>
```

**Mealplan commands (operate on current group):**
```
fit mealplan create --date 2026-01-14 --type dinner --dish "Salmon"
fit mealplan list [--from DATE] [--to DATE]
fit mealplan show <date> [--type TYPE]
```

**Meal logging (personal):**
```
fit meal log <mealplan-id>                    # log from plan
fit meal log --date DATE --type TYPE --dish "Salmon"  # ad-hoc
fit meal history [--from DATE] [--to DATE]
```

**Identity/device commands:**
```
fit init --new                    # first device
fit init --join <identity-id>     # additional device
fit device show                   # show your identity doc ID
```

**Sync:**
```
fit sync                          # manual sync
fit sync status                   # show sync config
```

### 8. Config & Storage

**Config file (device-local settings):**
```yaml
# ~/.config/fit/config.yaml
sync_server: "ws://192.168.1.50:8080"  # optional
current_group: "family"                 # device-local preference
```

**Identity file:**
```
# ~/.local/share/fit/root_doc_id
ABC123  # your identity doc ID
```

**Storage layout:**
```
~/.local/share/fit/
├── root_doc_id                    # your identity doc ID
├── <identity-id>.automerge        # your identity doc
├── <meallogs-id>.automerge        # your meallogs
├── <group-id>.automerge           # group manifest (shared)
├── <dishes-id>.automerge          # recipes (shared)
└── <mealplans-id>.automerge       # meal schedule (shared)
```

## Deferred Decisions

Not needed for MVP:
- Data lifecycle (leaving groups, deleting groups)
- Access control (kicking people from groups)
- Schema evolution/versioning

---

*Last updated: 2026-01-09*
