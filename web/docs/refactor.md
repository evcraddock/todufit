# Auth + Sync Refactor Working Notes

## Goals
- Replace todu-sync usage with the official Automerge repo sync server.
- Move auth into todu-fit-web (no API keys).
- Let authenticated users attach an existing CLI identity root doc id.
- Use that identity + group model for document discovery.
- Keep a constant WebSocket sync connection.

## Current State (todu-fit-web)
- Auth is API-key based via `/auth` endpoints; key is stored in localStorage and appended to `VITE_SYNC_URL` as `?key=`.
- Doc IDs are deterministic hashes of `userId/groupId` in `src/repo/docId.ts`.
- Dishes/mealplans/meallogs docs already use the CLI data shapes (snake_case, maps keyed by UUID strings).
- The production server (`server.ts`) only serves static assets (no auth or DB).

## Target Model (todu-fit CLI)
- Identity key == identity root document ID (bs58check-encoded UUID bytes).
- Identity document stores:
  - `meallogs_doc_id`
  - `groups: [{ name, doc_id }]`
- Group document stores:
  - `dishes_doc_id`
  - `mealplans_doc_id`
- Sync server is unauthenticated (automerge-repo WS protocol).

## Proposed Approach

### 1) Auth in todu-fit-web (no API keys)
- Implement new `/auth` endpoints in the Hono server.
- Use session cookies (HTTP-only) instead of API keys.
- Keep email magic link + passkey flows, but re-point to new server endpoints.
- Minimal session-aware endpoints:
  - `POST /auth/login` (start login flow)
  - `POST /auth/verify` or `GET /auth/callback` (complete login, set session cookie)
  - `POST /auth/logout`
  - `GET /auth/me` (returns user profile + settings)

SQLite tables (example):
- `users`: id, email, created_at, etc.
- `sessions`: id, user_id, expires_at
- `user_settings`: user_id, root_doc_id, current_group_id (optional)
- `passkeys` or `magic_links` (if continuing those flows)

### 2) Root doc id onboarding
- After login, prompt the user to enter their CLI identity root doc id (bs58check string).
- Validate the format before saving.
- Store in `user_settings.root_doc_id` (SQLite).
- Expose via `GET /auth/me` so the frontend can bootstrap.

### 3) Repo bootstrapping + document discovery
- Drop deterministic doc ID hashing in `src/repo/docId.ts`.
- Use the root doc id as the entry point:
  1. Load identity doc: `automerge:<root_doc_id>`.
  2. Parse the JSON stored in the Automerge doc under key `data` (matches CLI storage).
  3. Read `meallogs_doc_id` and group refs from the identity doc.
  4. Load the selected group doc to get `dishes_doc_id` and `mealplans_doc_id`.
- Store selected group (user preference) in SQLite (server-side); optionally cache in localStorage for instant startup.
- Handle "Pending sync" if identity doc is not found yet (same behavior as CLI join).

### 4) Sync server connectivity
- Use the official sync server: `@automerge/automerge-repo-sync-server`.
- No auth on the sync server; protect it at the network level.
- Remove `?key=` from the client sync URL.
- Use a WS proxy: browser -> todu-fit-web `/sync` (session-protected) -> internal sync server.
- Note: a direct browser connection to the sync server is only needed if you later expose it to clients.

If proxying:
- Add a WS route in the Hono server (e.g., `/sync`).
- Authorize the WS connection using the session cookie before piping messages.
- Proxy bytes to the internal sync server via `ws`.

### 5) Data compatibility / migration
- Web app data structures already match CLI docs, but doc IDs do not.
- Moving to identity+group doc IDs will not see existing data stored under hashed IDs.
- No migration planned; this is a clean cut-over to documents referenced by the root doc id.

## Suggested Implementation Sequence
1. Add Hono auth endpoints + SQLite persistence (sessions + user_settings).
2. Add UI for root doc id entry and group selection.
3. Update repo bootstrap to load identity + group docs from root doc id.
4. Switch WS sync URL to official sync server (direct or proxy).
5. Remove API key flow and hashed doc IDs.

## Decisions
- Keep magic link + passkey login.
- Single identity per user (one root doc id; multi-identity would mean multiple root doc ids per account).
- Use WS proxy via todu-fit-web `/sync` to reach the internal sync server.
- No migration; clean cut-over to documents referenced by the root doc id.

## Decisions
- Store `current_group_id` in SQLite for cross-device consistency, and optionally cache in localStorage for instant UI on startup/offline.
