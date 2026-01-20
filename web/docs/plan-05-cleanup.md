# Plan 05: Cleanup and Cut-over

## Scope
Remove legacy API-key auth and hashed doc ID usage; finalize the cut-over to identity/group doc IDs and session auth.

## Requirements
- Remove API key fields from client auth state and storage.
- Remove `?key=` usage in sync URL.
- Remove deterministic doc ID hashing utilities from the web app.
- Remove unused auth endpoints related to API keys.
- Update documentation and config examples to reflect the new flow.

## Constraints
- No migration from hashed doc IDs; users must use the root doc id flow.

## Acceptance Criteria
- App functions end-to-end with session auth + identity doc id.
- No references to API keys or hashed doc IDs remain in the codebase.
- Documentation reflects the new workflow.
