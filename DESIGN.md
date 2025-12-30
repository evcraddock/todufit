# Todufit Design Document

A local-first fitness app with meal planning, dishes, and cross-device sync.

## Requirements
- Database driven
- Runs locally on Linux and macOS
- Local-first with sync to a sync server
- Uses Automerge or similar CRDT technology
- Installable on a machine (not just dev environment)

## Out of Scope (for now)
- Vector database / semantic search (shelved for future consideration)

## Design Decisions

### Programming Language
**Decision:** Rust

**Rationale:**
- Single binary distribution (easy install on Linux/macOS)
- automerge-rs is the reference CRDT implementation
- Excellent CLI tooling ecosystem (clap, etc.)
- User is comfortable with Rust

### Architecture
**Decision:** Split architecture based on device role

**Primary devices (laptop/desktop):**
- SQLite as local database (source of truth for queries)
- Automerge for sync protocol
- Full CLI with LLM (Claude) integration for meal planning
- Complex queries supported locally

**Secondary devices (phone/tablet):**
- Automerge-only (no SQLite)
- Read-only viewer UI
- Displays synced meal plans
- Works offline with last synced state
- PWA/Web (deferred - focus on CLI first)

**Sync Server:**
- Central hub for Automerge document sync
- All devices sync through server
- Authenticated (API key/token per user)
- User identity tracked (Automerge actor IDs)
- Shared document space per group (all users see everything)
- Authorization: verify user belongs to group
- User onboarding/group management: deferred
- Built in Rust (same stack as CLI, full control over auth)

```
┌────────────────────┐         ┌────────────────────┐
│ Laptop (primary)   │         │ Phone (viewer)     │
│ SQLite + Automerge │◀───────▶│ Automerge only     │
│ full CLI + Claude  │  sync   │ read-only UI       │
└────────────────────┘    │    └────────────────────┘
                          │
                    ┌─────▼─────┐
                    │Sync Server│
                    └───────────┘
```

**Rationale:**
- LLM needs rich queries for meal planning (SQLite on primary)
- Viewers don't need complex queries (Automerge sufficient)
- Single binary install for normies
- Distribution: GitHub releases + curl installer initially, package managers later
- Offline-capable on all devices

### Rust Libraries
**Decision:**
- **CLI:** clap
- **Database:** sqlx (SQLite)
- **Async runtime:** tokio
- **HTTP/WebSocket:** axum
- **CRDT/Sync:** automerge-rs

---

### MVP Scope
**Decision:** Meal planning only

**Core entities (from mealplan-mcp):**
- Dish (name, ingredients, instructions, nutrients)
- Ingredient
- Nutrient
- MealPlan
- MealType

**Future (post-MVP):**
- Workout tracking
- Weight/body measurements
- Calorie/macro goals
- Exercise library

---

## Open Questions

*To be resolved through discussion*
