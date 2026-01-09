# Step 3: Multi-Document Storage

## Goal

Store and manage multiple Automerge documents by their DocumentId.

## Current State

- Single storage path per document type (`dishes.automerge`, etc.)
- Hardcoded filenames

## New State

- Documents stored by ID: `<doc-id>.automerge`
- Discovery through identity document
- Load documents on demand

## Storage Layout

```
~/.local/share/fit/
├── root_doc_id                    # text file with identity doc ID
├── <identity-id>.automerge
├── <meallogs-id>.automerge
├── <group-id>.automerge
├── <dishes-id>.automerge
└── <mealplans-id>.automerge
```

## Tasks

- [ ] Create `DocumentStorage` that stores by DocumentId
  - `save(doc_id, bytes)`
  - `load(doc_id) -> Option<bytes>`
  - `exists(doc_id) -> bool`
  - `list() -> Vec<DocumentId>`
- [ ] Add `root_doc_id` file handling
  - `save_root_id(doc_id)`
  - `load_root_id() -> Option<DocumentId>`
- [ ] Remove old hardcoded storage paths
- [ ] Add tests

## Done When

- Can store and retrieve documents by DocumentId
- Root doc ID persists across restarts
