# Step 5: Identity Management

## Goal

Implement identity and group management.

## Identity States

1. **Uninitialized** - no root_doc_id file
2. **Initialized** - has root_doc_id, has identity document
3. **Pending sync** - has root_doc_id, no local identity document (joined but not synced)

## Identity Operations

### Create New Identity
1. Generate new DocumentId for identity doc
2. Generate new DocumentId for meallogs doc
3. Create identity document with meallogs_doc_id
4. Create empty meallogs document
5. Save root_doc_id file
6. Save both documents

### Join Existing Identity
1. Save provided DocumentId as root_doc_id
2. Mark as "pending sync"
3. On next sync, fetch identity doc and all referenced docs

## Group Operations

### Create Group
1. Generate DocumentIds for group, dishes, mealplans docs
2. Create group document with dishes_doc_id, mealplans_doc_id
3. Create empty dishes and mealplans documents
4. Add group reference to identity document
5. Save all documents

### Join Group
1. Add group_doc_id to identity document
2. On sync, fetch group doc and discover dishes/mealplans doc IDs
3. Fetch those documents

## Tasks

- [ ] Create `Identity` struct with state management
- [ ] Implement `initialize_new()`
- [ ] Implement `initialize_join(doc_id)`
- [ ] Implement `is_initialized()`, `is_pending_sync()`
- [ ] Implement `create_group(name)`
- [ ] Implement `join_group(group_doc_id)`
- [ ] Implement `list_groups()`
- [ ] Add tests

## Done When

- Can create new identity
- Can join existing identity (pending sync state)
- Can create and join groups
