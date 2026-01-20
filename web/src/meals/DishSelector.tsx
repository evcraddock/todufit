import { useState, useMemo } from 'react'
import { Dish } from '../dishes'

const MIN_SEARCH_LENGTH = 2
const MAX_RESULTS = 20

interface DishSelectorProps {
  dishes: Dish[]
  selectedDishIds: string[]
  onToggleDish: (dishId: string) => void
  /** Color theme for selected items - 'blue' for meal plans, 'green' for meal logs */
  colorTheme?: 'blue' | 'green'
}

export function DishSelector({
  dishes,
  selectedDishIds,
  onToggleDish,
  colorTheme = 'blue',
}: DishSelectorProps) {
  const [search, setSearch] = useState('')

  // Get selected dishes for display
  const selectedDishes = useMemo(
    () => dishes.filter((d) => selectedDishIds.includes(d.id)),
    [dishes, selectedDishIds]
  )

  // Filter dishes based on search
  const { filteredDishes, totalMatches, showResults } = useMemo(() => {
    const query = search.trim().toLowerCase()

    // Don't show results if search is too short
    if (query.length < MIN_SEARCH_LENGTH) {
      return { filteredDishes: [], totalMatches: 0, showResults: false }
    }

    // Filter dishes by search (name or tags), excluding already selected
    const matches = dishes.filter((dish) => {
      if (selectedDishIds.includes(dish.id)) return false
      const nameMatch = dish.name.toLowerCase().includes(query)
      const tagMatch = dish.tags?.some((tag) => tag.toLowerCase().includes(query))
      return nameMatch || tagMatch
    })

    return {
      filteredDishes: matches.slice(0, MAX_RESULTS),
      totalMatches: matches.length,
      showResults: true,
    }
  }, [dishes, search, selectedDishIds])

  // Determine empty state message
  const emptyStateMessage = useMemo(() => {
    const query = search.trim()

    if (query.length === 0) {
      return 'Type to search for dishes...'
    }

    if (query.length < MIN_SEARCH_LENGTH) {
      return `Enter at least ${MIN_SEARCH_LENGTH} characters to search`
    }

    // Check if all matches are already selected
    const allMatches = dishes.filter((dish) => {
      const q = query.toLowerCase()
      const nameMatch = dish.name.toLowerCase().includes(q)
      const tagMatch = dish.tags?.some((tag) => tag.toLowerCase().includes(q))
      return nameMatch || tagMatch
    })
    if (allMatches.length > 0 && allMatches.every((d) => selectedDishIds.includes(d.id))) {
      return 'All matching dishes are already selected'
    }

    return `No dishes found matching "${query}"`
  }, [search, dishes, selectedDishIds])

  // Color classes based on theme
  const colors = {
    blue: {
      selectedBg: 'bg-blue-100 dark:bg-blue-900/40',
      selectedText: 'text-blue-800 dark:text-blue-300',
      selectedButton: 'text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300',
      rowSelected: 'bg-blue-50 dark:bg-blue-900/30',
    },
    green: {
      selectedBg: 'bg-green-100 dark:bg-green-900/40',
      selectedText: 'text-green-800 dark:text-green-300',
      selectedButton: 'text-green-600 dark:text-green-400 hover:text-green-800 dark:hover:text-green-300',
      rowSelected: 'bg-green-50 dark:bg-green-900/30',
    },
  }[colorTheme]

  return (
    <>
      {/* Search */}
      <input
        type="search"
        placeholder="Search dishes..."
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        className="w-full px-4 py-2 mb-4 border border-gray-300 dark:border-gray-600 rounded focus:outline-none focus:border-blue-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
      />

      {/* Selected dishes */}
      {selectedDishIds.length > 0 && (
        <div className="mb-4">
          <p className="text-sm text-gray-500 dark:text-gray-400 mb-2">
            Selected ({selectedDishIds.length}):
          </p>
          <div className="flex flex-wrap gap-2">
            {selectedDishes.map((dish) => (
              <span
                key={dish.id}
                className={`inline-flex items-center gap-1 ${colors.selectedBg} ${colors.selectedText} px-2 py-1 rounded text-sm`}
              >
                {dish.name}
                <button
                  type="button"
                  onClick={() => onToggleDish(dish.id)}
                  className={colors.selectedButton}
                >
                  ×
                </button>
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Search results or empty state */}
      <div className="max-h-60 overflow-y-auto border border-gray-200 dark:border-gray-700 rounded">
        {!showResults || filteredDishes.length === 0 ? (
          <p className="p-4 text-gray-400 dark:text-gray-500 text-center text-sm">
            {emptyStateMessage}
          </p>
        ) : (
          <>
            {totalMatches > MAX_RESULTS && (
              <p className="px-4 py-2 text-xs text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                Showing {MAX_RESULTS} of {totalMatches} matches. Refine your search.
              </p>
            )}
            <ul className="divide-y divide-gray-200 dark:divide-gray-700">
              {filteredDishes.map((dish) => (
                <li key={dish.id}>
                  <button
                    type="button"
                    onClick={() => onToggleDish(dish.id)}
                    className="w-full text-left px-4 py-2 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors text-gray-900 dark:text-gray-100"
                  >
                    <span className="flex items-center gap-2">
                      <input type="checkbox" checked={false} readOnly className="rounded" />
                      <span>{dish.name}</span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
    </>
  )
}
