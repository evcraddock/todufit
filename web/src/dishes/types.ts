export interface Ingredient {
  name: string
  quantity: string
  unit: string
}

export interface Nutrient {
  name: string
  amount: number
  unit: string
}

export interface Dish {
  id: string
  name: string
  instructions: string
  prepTime?: number
  cookTime?: number
  servings?: number
  tags: string[]
  ingredients: Ingredient[]
  nutrients: Nutrient[]
  createdAt: string
  updatedAt: string
}

// CLI document structure - dishes stored at root level with snake_case
export interface CliDish {
  name: string
  instructions: string
  prep_time?: number
  cook_time?: number
  servings?: number
  tags: string[]
  ingredients: Ingredient[]
  nutrients: Nutrient[]
  created_at: string
  updated_at: string
  created_by?: string
}

// Document as stored by CLI - UUIDs as keys at root level
export type DishesDoc = Record<string, CliDish>
