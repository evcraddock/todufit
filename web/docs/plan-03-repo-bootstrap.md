# Plan 03: Repo Bootstrap From Identity + Group Docs

## Scope
Switch Automerge document discovery from hash-based IDs to the CLI identity/group document model using the root doc id.

## Requirements
- Use root doc id to load the identity document.
- Parse identity document JSON stored under `data` key in the Automerge doc.
- Discover `meallogs_doc_id` and group refs from identity doc.
- Load the selected group document to get `dishes_doc_id` and `mealplans_doc_id`.
- Update repo state to use those document URLs for dishes, meal plans, and meal logs.

## Functional Details
- Replace deterministic doc ID hashing with identity/group document discovery.
- If identity doc is not found yet, surface a "pending sync" state in UI.
- If the selected group is missing or invalid, prompt user to select another group.

## UX Requirements
- Visible loading and error states while identity/group docs are fetched.
- Clear guidance if identity doc or group doc is missing on the server.

## Acceptance Criteria
- With a valid root doc id, dishes/mealplans/meallogs load and sync via the identity and group documents.
- The app does not read or write hashed doc IDs anymore.
- Pending-sync and missing-group states are actionable for the user.
