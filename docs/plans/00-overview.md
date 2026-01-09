# Implementation Plan Overview

Refactoring todu-fit to the new architecture described in [new-design.md](../new-design.md).

## Steps

| Step | Document | Description |
|------|----------|-------------|
| 1 | [01-document-id.md](01-document-id.md) | Add DocumentId type |
| 2 | [02-document-model.md](02-document-model.md) | Define new document structure |
| 3 | [03-multi-doc-storage.md](03-multi-doc-storage.md) | Multi-document storage layer |
| 4 | [04-remove-sqlite.md](04-remove-sqlite.md) | Remove SQLite projection |
| 5 | [05-identity-management.md](05-identity-management.md) | Identity and group management |
| 6 | [06-sync-changes.md](06-sync-changes.md) | Update sync for no-auth model |
| 7 | [07-cli-commands.md](07-cli-commands.md) | New and updated CLI commands |

## Dependencies

```
01-document-id
     │
     ▼
02-document-model
     │
     ▼
03-multi-doc-storage ◄─── 04-remove-sqlite
     │
     ▼
05-identity-management
     │
     ▼
06-sync-changes
     │
     ▼
07-cli-commands
```

## Approach

- Each step should result in working (if incomplete) code
- Tests should pass after each step
- Steps can be individual PRs or commits
