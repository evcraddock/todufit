import { useState, useEffect } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { useRepoState, RepoLoading } from '../repo'
import { useDishes } from './useDishes'
import { Dish, Ingredient } from './types'

interface IngredientInput {
  quantity: string
  unit: string
  name: string
}

export function DishForm() {
  const { isReady } = useRepoState()

  if (!isReady) {
    return <RepoLoading />
  }

  return <DishFormContent />
}

function DishFormContent() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { getDish, addDish, updateDish, isLoading } = useDishes()

  const isEdit = Boolean(id)
  const existingDish = id ? getDish(id) : undefined

  // Form state
  const [name, setName] = useState('')
  const [prepTime, setPrepTime] = useState('')
  const [cookTime, setCookTime] = useState('')
  const [servings, setServings] = useState('')
  const [tags, setTags] = useState('')
  const [ingredients, setIngredients] = useState<IngredientInput[]>([])
  const [instructions, setInstructions] = useState('')

  // Load existing dish data - only when id changes or loading completes
  // Using id as dependency instead of existingDish to avoid infinite loops
  // (existingDish is a new object reference on every render)
  useEffect(() => {
    if (existingDish) {
      setName(existingDish.name)
      setPrepTime(existingDish.prepTime?.toString() ?? '')
      setCookTime(existingDish.cookTime?.toString() ?? '')
      setServings(existingDish.servings?.toString() ?? '')
      setTags(existingDish.tags.join(', '))
      setIngredients(
        existingDish.ingredients.map((ing) => ({
          quantity: ing.quantity,
          unit: ing.unit,
          name: ing.name,
        }))
      )
      setInstructions(existingDish.instructions)
    }
  }, [id, isLoading])

  const handleAddIngredient = () => {
    setIngredients([...ingredients, { quantity: '', unit: '', name: '' }])
  }

  const handleRemoveIngredient = (index: number) => {
    setIngredients(ingredients.filter((_, i) => i !== index))
  }

  const handleIngredientChange = (
    index: number,
    field: keyof IngredientInput,
    value: string
  ) => {
    const updated = [...ingredients]
    updated[index][field] = value
    setIngredients(updated)
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()

    if (!name.trim()) {
      alert('Name is required')
      return
    }

    const now = new Date().toISOString()
    
    // Parse tags
    const parsedTags = tags
      .split(',')
      .map((t) => t.trim())
      .filter((t) => t.length > 0)

    // Parse ingredients (filter out empty rows)
    const parsedIngredients: Ingredient[] = ingredients
      .filter((ing) => ing.name.trim())
      .map((ing) => ({
        quantity: ing.quantity.trim(),
        unit: ing.unit.trim(),
        name: ing.name.trim(),
      }))

    if (isEdit && id) {
      // Update existing dish
      updateDish(id, {
        name: name.trim(),
        prepTime: prepTime ? parseInt(prepTime, 10) : undefined,
        cookTime: cookTime ? parseInt(cookTime, 10) : undefined,
        servings: servings ? parseInt(servings, 10) : undefined,
        tags: parsedTags,
        ingredients: parsedIngredients,
        instructions: instructions.trim(),
      })
      navigate(`/dishes/${id}`)
    } else {
      // Create new dish
      const newDish: Dish = {
        id: crypto.randomUUID(),
        name: name.trim(),
        prepTime: prepTime ? parseInt(prepTime, 10) : undefined,
        cookTime: cookTime ? parseInt(cookTime, 10) : undefined,
        servings: servings ? parseInt(servings, 10) : undefined,
        tags: parsedTags,
        ingredients: parsedIngredients,
        nutrients: [],
        instructions: instructions.trim(),
        createdAt: now,
        updatedAt: now,
      }
      addDish(newDish)
      navigate(`/dishes/${newDish.id}`)
    }
  }

  if (isLoading) {
    return (
      <div className="text-center py-12 text-gray-500 dark:text-gray-400">
        Loading...
      </div>
    )
  }

  if (isEdit && !existingDish) {
    return (
      <div className="max-w-4xl mx-auto bg-white dark:bg-gray-800 rounded-lg shadow p-8 transition-colors">
        <p className="text-gray-500 dark:text-gray-400 text-center">
          Dish not found.{' '}
          <Link to="/dishes" className="text-blue-600 dark:text-blue-400 hover:underline">
            Back to dishes
          </Link>
        </p>
      </div>
    )
  }

  return (
    <div className="max-w-4xl mx-auto bg-white dark:bg-gray-800 rounded-lg shadow p-4 sm:p-8 transition-colors">
      <h1 className="text-2xl font-bold mb-6 text-gray-900 dark:text-gray-100">
        {isEdit ? 'Edit Dish' : 'New Dish'}
      </h1>

      <form onSubmit={handleSubmit} className="space-y-6">
        {/* Name */}
        <div>
          <label htmlFor="name" className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">
            Name *
          </label>
          <input
            type="text"
            id="name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            autoFocus
            className="w-full px-4 py-3 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          />
        </div>

        {/* Time & Servings */}
        <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 sm:gap-4">
          <div>
            <label htmlFor="prepTime" className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">
              Prep Time (min)
            </label>
            <input
              type="number"
              id="prepTime"
              value={prepTime}
              onChange={(e) => setPrepTime(e.target.value)}
              min="0"
              className="w-full px-4 py-3 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
            />
          </div>
          <div>
            <label htmlFor="cookTime" className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">
              Cook Time (min)
            </label>
            <input
              type="number"
              id="cookTime"
              value={cookTime}
              onChange={(e) => setCookTime(e.target.value)}
              min="0"
              className="w-full px-4 py-3 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
            />
          </div>
          <div className="col-span-2 sm:col-span-1">
            <label htmlFor="servings" className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">
              Servings
            </label>
            <input
              type="number"
              id="servings"
              value={servings}
              onChange={(e) => setServings(e.target.value)}
              min="1"
              className="w-full px-4 py-3 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
            />
          </div>
        </div>

        {/* Tags */}
        <div>
          <label htmlFor="tags" className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">
            Tags (comma-separated)
          </label>
          <input
            type="text"
            id="tags"
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            placeholder="e.g., italian, quick, vegetarian"
            className="w-full px-4 py-3 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          />
        </div>

        {/* Ingredients */}
        <fieldset className="border border-gray-300 dark:border-gray-600 rounded-lg p-4">
          <legend className="text-sm font-medium px-2 text-gray-700 dark:text-gray-300">Ingredients</legend>
          <div className="space-y-3 mb-4">
            {ingredients.map((ing, idx) => (
              <div key={idx} className="flex flex-col sm:flex-row gap-2">
                <div className="flex gap-2 flex-1">
                  <input
                    type="text"
                    value={ing.quantity}
                    onChange={(e) => handleIngredientChange(idx, 'quantity', e.target.value)}
                    placeholder="Qty"
                    className="w-20 sm:w-20 px-3 py-2 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                  />
                  <input
                    type="text"
                    value={ing.unit}
                    onChange={(e) => handleIngredientChange(idx, 'unit', e.target.value)}
                    placeholder="Unit"
                    className="w-24 sm:w-24 px-3 py-2 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                  />
                  <input
                    type="text"
                    value={ing.name}
                    onChange={(e) => handleIngredientChange(idx, 'name', e.target.value)}
                    placeholder="Ingredient"
                    className="flex-1 px-3 py-2 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                  />
                </div>
                <button
                  type="button"
                  onClick={() => handleRemoveIngredient(idx)}
                  className="w-11 h-11 min-w-[44px] min-h-[44px] bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors flex items-center justify-center self-end sm:self-auto"
                >
                  ×
                </button>
              </div>
            ))}
          </div>
          <button
            type="button"
            onClick={handleAddIngredient}
            className="px-4 py-2 min-h-[44px] text-sm bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg transition-colors"
          >
            + Add Ingredient
          </button>
        </fieldset>

        {/* Instructions */}
        <div>
          <label htmlFor="instructions" className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">
            Instructions
          </label>
          <textarea
            id="instructions"
            value={instructions}
            onChange={(e) => setInstructions(e.target.value)}
            rows={8}
            placeholder="Step-by-step cooking instructions..."
            className="w-full px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-y bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
          />
        </div>

        {/* Actions */}
        <div className="flex flex-col-reverse sm:flex-row justify-end gap-3 sm:gap-4 pt-4">
          <Link
            to={isEdit ? `/dishes/${id}` : '/dishes'}
            className="px-4 py-3 min-h-[44px] bg-gray-600 dark:bg-gray-700 text-white rounded-lg hover:bg-gray-700 dark:hover:bg-gray-600 transition-colors text-center"
          >
            Cancel
          </Link>
          <button
            type="submit"
            className="px-6 py-3 min-h-[44px] bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium"
          >
            {isEdit ? 'Save Changes' : 'Create Dish'}
          </button>
        </div>
      </form>
    </div>
  )
}
