import * as Automerge from '@automerge/automerge'
import { useDocument } from '../repo'
import { useRepoState } from '../repo/RepoContext'
import { Dish, DishesDoc, CliDish } from './types'

// Helper to create ImmutableString for non-collaborative text
// This ensures strings are stored as scalar values (compatible with automerge-rs)
// rather than collaborative text (which automerge-rs 0.7 cannot read)
function imm(value: string): Automerge.ImmutableString {
  return new Automerge.ImmutableString(value)
}

// Helper to extract string value from automerge strings or numbers
// Handles compatibility between automerge-rs and automerge-js string formats
function getString(value: unknown): string {
  if (typeof value === 'string') {
    return value
  }
  if (typeof value === 'number') {
    return String(value)
  }
  if (value && typeof value === 'object' && 'val' in value) {
    return String((value as { val: unknown }).val)
  }
  if (value && typeof value === 'object' && 'toString' in value) {
    return String(value)
  }
  return ''
}

// Convert CLI dish (snake_case) to web dish (camelCase)
// Uses getString to handle automerge string compatibility
function convertCliDish(id: string, cliDish: CliDish): Dish {
  return {
    id,
    name: getString(cliDish.name),
    instructions: getString(cliDish.instructions),
    prepTime: cliDish.prep_time,
    cookTime: cliDish.cook_time,
    servings: cliDish.servings,
    tags: (cliDish.tags ?? []).map((t) => getString(t)),
    // Convert strings inside ingredients array
    ingredients: (cliDish.ingredients ?? []).map((ing) => ({
      name: getString(ing.name),
      quantity: getString(ing.quantity),
      unit: getString(ing.unit),
    })),
    // Convert strings inside nutrients array
    nutrients: (cliDish.nutrients ?? []).map((nut) => ({
      name: getString(nut.name),
      amount: nut.amount,
      unit: getString(nut.unit),
    })),
    createdAt: getString(cliDish.created_at),
    updatedAt: getString(cliDish.updated_at),
  }
}

export function useDishes() {
  const { docUrls } = useRepoState()
  const [doc, changeDoc] = useDocument<DishesDoc>(docUrls?.dishes)

  // Convert CLI format (root-level, snake_case) to web format (camelCase with id)
  // Filter out invalid entries (automerge metadata, etc)
  // A valid dish must have a 'name' property that can be converted to a non-empty string
  const isValidDish = (entry: unknown): entry is CliDish => {
    if (entry === null || typeof entry !== 'object' || !('name' in entry)) {
      return false
    }
    const name = getString((entry as { name: unknown }).name)
    return name.length > 0
  }

  const dishes = doc
    ? Object.entries(doc)
        .filter(([, entry]) => isValidDish(entry))
        .map(([id, cliDish]) => convertCliDish(id, cliDish))
    : []

  const getDish = (id: string): Dish | undefined => {
    const cliDish = doc?.[id]
    return cliDish ? convertCliDish(id, cliDish) : undefined
  }

  const addDish = (dish: Dish) => {
    changeDoc((d) => {
      // Convert web format to CLI format (snake_case)
      // Use ImmutableString for all string fields to ensure compatibility with automerge-rs
      // Plain JS strings in Automerge 3.x are treated as collaborative text (Text type)
      // but automerge-rs 0.7 expects scalar strings (Str type)
      const cliDish: Record<string, unknown> = {
        name: imm(dish.name),
        created_at: imm(dish.createdAt),
        updated_at: imm(dish.updatedAt),
        created_by: imm('web'),
        instructions: imm(dish.instructions || ''),
        tags: (dish.tags || []).map((t) => imm(t)),
        ingredients: (dish.ingredients || []).map((ing) => ({
          name: imm(ing.name),
          quantity: ing.quantity,
          unit: imm(ing.unit),
        })),
      }
      if (dish.prepTime !== undefined) cliDish.prep_time = dish.prepTime
      if (dish.cookTime !== undefined) cliDish.cook_time = dish.cookTime
      if (dish.servings !== undefined) cliDish.servings = dish.servings
      if (dish.nutrients && dish.nutrients.length > 0) {
        cliDish.nutrients = dish.nutrients.map((nut) => ({
          name: imm(nut.name),
          amount: nut.amount,
          unit: imm(nut.unit),
        }))
      }
      d[dish.id] = cliDish as unknown as CliDish
    })
  }

  const updateDish = (id: string, updates: Partial<Dish>) => {
    changeDoc((d) => {
      if (d[id]) {
        // Apply updates in CLI format using ImmutableString for string fields
        if (updates.name !== undefined) d[id].name = imm(updates.name) as unknown as string
        if (updates.instructions !== undefined)
          d[id].instructions = imm(updates.instructions) as unknown as string
        if (updates.prepTime !== undefined) d[id].prep_time = updates.prepTime
        if (updates.cookTime !== undefined) d[id].cook_time = updates.cookTime
        if (updates.servings !== undefined) d[id].servings = updates.servings
        if (updates.tags !== undefined)
          d[id].tags = updates.tags.map((t) => imm(t)) as unknown as string[]
        if (updates.ingredients !== undefined)
          d[id].ingredients = updates.ingredients.map((ing) => ({
            name: imm(ing.name),
            quantity: ing.quantity,
            unit: imm(ing.unit),
          })) as unknown as typeof d[typeof id]['ingredients']
        if (updates.nutrients !== undefined)
          d[id].nutrients = updates.nutrients?.map((nut) => ({
            name: imm(nut.name),
            amount: nut.amount,
            unit: imm(nut.unit),
          })) as unknown as typeof d[typeof id]['nutrients']
        d[id].updated_at = imm(new Date().toISOString()) as unknown as string
      }
    })
  }

  const deleteDish = (id: string) => {
    changeDoc((d) => {
      delete d[id]
    })
  }

  // Get unique tags from all dishes
  const allTags = [...new Set(dishes.flatMap((d) => d.tags))].sort()

  return {
    dishes,
    getDish,
    addDish,
    updateDish,
    deleteDish,
    allTags,
    isLoading: !doc,
  }
}
