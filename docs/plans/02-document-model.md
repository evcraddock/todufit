# Step 2: Document Model

## Goal

Define the structure of the five document types.

## Document Types

### Identity Document (personal)
```
{
  meallogs_doc_id: "<doc-id>",
  groups: [
    {
      name: "family",
      group_doc_id: "<doc-id>"
    }
  ]
}
```

### Group Document (shared)
```
{
  name: "family",
  dishes_doc_id: "<doc-id>",
  mealplans_doc_id: "<doc-id>"
}
```

### Dishes Document (shared)
```
{
  "<dish-uuid>": {
    id, name, ingredients, nutrients, tags, ...
  },
  ...
}
```

### MealPlans Document (shared)
```
{
  "<mealplan-uuid>": {
    id, date, meal_type, dish_ids: [...], ...
  },
  ...
}
```

### MealLogs Document (personal)
```
{
  "<meallog-uuid>": {
    id, date, meal_type,
    dishes: [{ source_dish_id, name, nutrients, ... }],  // snapshots
    ...
  },
  ...
}
```

## Tasks

- [ ] Define `IdentityDocument` struct and Automerge read/write
- [ ] Define `GroupDocument` struct and Automerge read/write
- [ ] Update `Dish` model if needed
- [ ] Update `MealPlan` model - dish_ids instead of embedded dishes
- [ ] Update `MealLog` model - dish snapshots instead of dish_ids
- [ ] Add tests for each document type

## Done When

- All five document types have defined structures
- Can create, read, and write each document type to Automerge
