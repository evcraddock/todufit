# Plan 02: Identity Root Doc ID Onboarding

## Scope
Allow an authenticated user to attach a single identity root doc id to their account and persist it in SQLite. Support creating or joining an identity directly from the website (no CLI required). Enable a persistent current group selection.

## Requirements
- After login, prompt for an identity setup choice if not already set: create new identity or join existing.
- Validate root doc id format (bs58check-encoded UUID bytes).
- Save root doc id in `user_settings.root_doc_id`.
- Store `current_group_id` in SQLite for cross-device consistency.
- Expose root doc id and current group id via `GET /auth/me`.

## Functional Details
- Add endpoint: `POST /auth/settings/root-doc-id` (or similar) to save the root doc id.
- Add endpoint: `POST /auth/settings/current-group` to save the current group.
- Create identity flow (browser):
  - Generate a new identity document with a new `meallogs_doc_id`.
  - Create an initial group document with new `dishes_doc_id` and `mealplans_doc_id` (prompt for group name).
  - Save the identity root doc id in user settings and set `current_group_id`.
  - Show the identity id for sharing with other devices.
- Join identity flow (browser):
  - User enters an existing root doc id to join.
  - Save it in user settings and wait for sync if data is not available yet.
- Frontend should block access to app data until root doc id is saved.
- Allow updating root doc id, with confirmation if it is changing.

## UX Requirements
- Clear identity setup screen with: "Create new identity" and "Join existing identity" options.
- Simple form to enter root doc id (copy/paste from CLI output or shared from another device).
- For create flow, ask for initial group name and then show the generated identity id.
- Show current group name and allow switching if multiple groups exist.

## Acceptance Criteria
- Authenticated user can create a new identity from the website and see it persist across sessions.
- Authenticated user can join an existing identity by entering a root doc id.
- Current group selection persists across devices after re-login.
- Invalid root doc id inputs are rejected with a clear error message.
