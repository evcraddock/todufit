import { useState, useMemo } from 'react'
import { Link } from 'react-router-dom'
import { useRepoState, RepoLoading } from '../repo'
import { useDishes } from './useDishes'

const ITEMS_PER_PAGE = 20

export function DishList() {
  const { isReady } = useRepoState()

  if (!isReady) {
    return <RepoLoading />
  }

  return <DishListContent />
}

function DishListContent() {
  const { dishes, allTags, isLoading } = useDishes()
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedTag, setSelectedTag] = useState('')
  const [currentPage, setCurrentPage] = useState(1)

  // Filter dishes by search query and tag
  const filteredDishes = useMemo(() => {
    let result = dishes

    if (searchQuery) {
      const query = searchQuery.toLowerCase()
      result = result.filter((dish) =>
        dish.name.toLowerCase().includes(query)
      )
    }

    if (selectedTag) {
      result = result.filter((dish) => dish.tags.includes(selectedTag))
    }

    // Sort by name
    return result.sort((a, b) => a.name.localeCompare(b.name))
  }, [dishes, searchQuery, selectedTag])

  // Pagination
  const totalPages = Math.ceil(filteredDishes.length / ITEMS_PER_PAGE)
  const paginatedDishes = useMemo(() => {
    const start = (currentPage - 1) * ITEMS_PER_PAGE
    return filteredDishes.slice(start, start + ITEMS_PER_PAGE)
  }, [filteredDishes, currentPage])

  // Reset page when filters change
  const handleSearch = (query: string) => {
    setSearchQuery(query)
    setCurrentPage(1)
  }

  const handleTagChange = (tag: string) => {
    setSelectedTag(tag)
    setCurrentPage(1)
  }

  if (isLoading) {
    return (
      <div className="text-center py-12 text-gray-500 dark:text-gray-400">
        Loading dishes...
      </div>
    )
  }

  return (
    <div className="max-w-4xl mx-auto">
      {/* Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">Dishes</h1>
        <Link
          to="/dishes/new"
          className="bg-blue-600 text-white px-4 py-3 min-h-[44px] rounded-lg hover:bg-blue-700 transition-colors w-full sm:w-auto text-center"
        >
          New Dish
        </Link>
      </div>

      {/* Search and Filter */}
      <div className="flex flex-col sm:flex-row gap-3 mb-6">
        <input
          type="search"
          placeholder="Search dishes..."
          value={searchQuery}
          onChange={(e) => handleSearch(e.target.value)}
          className="flex-1 px-4 py-3 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
        />
        {allTags.length > 0 && (
          <select
            value={selectedTag}
            onChange={(e) => handleTagChange(e.target.value)}
            className="px-4 py-3 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          >
            <option value="">All tags</option>
            {allTags.map((tag) => (
              <option key={tag} value={tag}>
                {tag}
              </option>
            ))}
          </select>
        )}
      </div>

      {/* Dish Grid */}
      {filteredDishes.length === 0 ? (
        <div className="text-center py-12 text-gray-500 dark:text-gray-400">
          {dishes.length === 0 ? (
            <p>
              No dishes yet.{' '}
              <Link to="/dishes/new" className="text-blue-600 dark:text-blue-400 hover:underline">
                Create your first dish
              </Link>
              .
            </p>
          ) : (
            <p>No dishes found.</p>
          )}
        </div>
      ) : (
        <>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {paginatedDishes.map((dish) => (
              <Link
                key={dish.id}
                to={`/dishes/${dish.id}`}
                className="bg-white dark:bg-gray-800 rounded-lg shadow hover:shadow-lg transition-shadow p-4 sm:p-5 min-h-[80px] active:bg-gray-50 dark:active:bg-gray-700"
              >
                <h3 className="font-semibold text-gray-900 dark:text-gray-100 mb-2 text-base">{dish.name}</h3>
                {(dish.prepTime || dish.cookTime) && (
                  <p className="text-gray-500 dark:text-gray-400 text-sm mb-2">
                    {(dish.prepTime ?? 0) + (dish.cookTime ?? 0)} min
                  </p>
                )}
                {dish.tags.length > 0 && (
                  <div className="flex gap-2 flex-wrap">
                    {dish.tags.slice(0, 3).map((tag) => (
                      <span
                        key={tag}
                        className="bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 text-xs px-2 py-1 rounded"
                      >
                        {tag}
                      </span>
                    ))}
                  </div>
                )}
              </Link>
            ))}
          </div>

          {/* Pagination */}
          {totalPages > 1 && (
            <div className="flex flex-col items-center gap-3 mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
              <p className="text-gray-500 dark:text-gray-400 text-sm text-center">
                Showing {(currentPage - 1) * ITEMS_PER_PAGE + 1}-
                {Math.min(currentPage * ITEMS_PER_PAGE, filteredDishes.length)} of{' '}
                {filteredDishes.length} dishes
              </p>
              <div className="flex items-center gap-2 sm:gap-4">
                <button
                  onClick={() => setCurrentPage((p) => p - 1)}
                  disabled={currentPage === 1}
                  className="px-4 py-3 min-h-[44px] bg-gray-600 dark:bg-gray-700 text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-700 dark:hover:bg-gray-600 transition-colors"
                >
                  ← Prev
                </button>
                <span className="text-gray-700 dark:text-gray-300 font-medium text-sm sm:text-base px-2">
                  {currentPage} / {totalPages}
                </span>
                <button
                  onClick={() => setCurrentPage((p) => p + 1)}
                  disabled={currentPage === totalPages}
                  className="px-4 py-3 min-h-[44px] bg-gray-600 dark:bg-gray-700 text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-700 dark:hover:bg-gray-600 transition-colors"
                >
                  Next →
                </button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  )
}
