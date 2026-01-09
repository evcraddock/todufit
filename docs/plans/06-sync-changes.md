# Step 6: Sync Changes

## Goal

Update sync to work without authentication, using document IDs for routing.

## Current State

- Magic link auth flow
- API key stored in config
- Custom todu-sync server

## New State

- No authentication
- Document ID identifies what to sync
- Standard automerge-repo-sync-server compatible
- Sync all documents referenced by identity doc

## Sync Flow

1. Load identity document
2. Collect all doc IDs to sync:
   - Identity doc itself
   - Meallogs doc
   - For each group: group doc, dishes doc, mealplans doc
3. Connect to sync server
4. Sync each document
5. Save updated documents

## What to Remove

- `todu-fit-cli/src/commands/auth.rs`
- API key from config
- Auth-related code in sync client

## What to Change

- Sync client takes list of DocumentIds, not document types
- No authentication headers/handshake
- Handle "pending sync" state (join without local doc)

## Tasks

- [ ] Remove auth command
- [ ] Remove api_key from config
- [ ] Update sync client to work without auth
- [ ] Update sync client to sync by DocumentId
- [ ] Implement sync for pending state (initial join)
- [ ] Handle sync server unavailable gracefully
- [ ] Add tests

## Done When

- Auth code removed
- Can sync documents by ID without authentication
- Joining a new identity syncs successfully when server available
- Works offline when server unavailable
