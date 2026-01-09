# Step 7: CLI Commands

## Goal

Update CLI for new multi-user, multi-group model.

## New Commands

### Init Commands
```
fit init --new              # create new identity
fit init --join <id>        # join existing identity
```

### Device Commands
```
fit device show             # show identity doc ID for sharing
```

### Group Commands
```
fit group create <name>     # create new group
fit group join <id>         # join existing group
fit group list              # list all groups
fit group switch <name>     # set current group
fit group show              # show current group details
```

## Updated Commands

### Dish Commands
- Operate on current group's dishes doc
- Add `--group` flag for override

```
fit dish create <name> [options]
fit dish list [--tag TAG] [--ingredient ING]
fit dish show <id|name>
fit dish edit <id|name>
fit dish delete <id|name>
```

### Mealplan Commands
- Operate on current group's mealplans doc
- Resolve dish references from current group's dishes doc

```
fit mealplan create --date DATE --type TYPE --dish NAME
fit mealplan list [--from DATE] [--to DATE]
fit mealplan show <date|id> [--type TYPE]
fit mealplan delete <id>
```

### Meal Commands
- Operate on personal meallogs doc
- Snapshot dish data when logging

```
fit meal log <mealplan-id>
fit meal log --date DATE --type TYPE --dish NAME
fit meal history [--from DATE] [--to DATE]
```

### Sync Commands
```
fit sync                    # sync all documents
fit sync status             # show sync configuration
```

### Config Commands
```
fit config show             # show config
fit config set <key> <val>  # update config
```

## Config Changes

```yaml
# ~/.config/fit/config.yaml
sync_server: "ws://192.168.1.50:8080"
current_group: "family"
```

## Tasks

- [ ] Add `init` command with `--new` and `--join` flags
- [ ] Add `device show` command
- [ ] Add `group` subcommands (create, join, list, switch, show)
- [ ] Update `dish` commands for current group context
- [ ] Update `mealplan` commands for current group context
- [ ] Update `meal` commands to snapshot dishes
- [ ] Add `--group` flag to relevant commands
- [ ] Update `config` for current_group
- [ ] Remove `auth` command
- [ ] Update help text and documentation

## Done When

- All new commands implemented
- Existing commands work with multi-group model
- Auth command removed
- Help text is accurate
