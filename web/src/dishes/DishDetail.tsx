import { useState } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { useRepoState, RepoLoading } from '../repo'
import { useDishes } from './useDishes'
import { ConfirmDialog, Markdown } from '../components'

export function DishDetail() {
  const { isReady } = useRepoState()

  if (!isReady) {
    return <RepoLoading />
  }

  return <DishDetailContent />
}

function DishDetailContent() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { getDish, deleteDish, isLoading } = useDishes()
  const [isDeleting, setIsDeleting] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)

  const dish = id ? getDish(id) : undefined

  const handleDelete = () => {
    if (!id || !dish) return
    setShowDeleteDialog(true)
  }

  const confirmDelete = () => {
    if (!id) return
    setIsDeleting(true)
    setShowDeleteDialog(false)
    deleteDish(id)
    navigate('/dishes')
  }

  if (isLoading) {
    return (
      <div className="text-center py-12 text-gray-500 dark:text-gray-400">
        Loading...
      </div>
    )
  }

  if (!dish) {
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

  const totalTime = (dish.prepTime ?? 0) + (dish.cookTime ?? 0)

  return (
    <div className="max-w-4xl mx-auto bg-white dark:bg-gray-800 rounded-lg shadow p-4 sm:p-8 transition-colors">
      {/* Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 mb-4">
        <button
          onClick={() => navigate(-1)}
          className="text-blue-600 dark:text-blue-400 hover:underline py-2"
        >
          ← Back
        </button>
        <div className="flex gap-2 w-full sm:w-auto">
          <Link
            to={`/dishes/${id}/edit`}
            className="flex-1 sm:flex-none px-4 py-3 min-h-[44px] bg-gray-600 dark:bg-gray-700 text-white rounded-lg hover:bg-gray-700 dark:hover:bg-gray-600 transition-colors text-center"
          >
            Edit
          </Link>
          <button
            onClick={handleDelete}
            disabled={isDeleting}
            className="flex-1 sm:flex-none px-4 py-3 min-h-[44px] bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors disabled:opacity-50"
          >
            Delete
          </button>
        </div>
      </div>

      {/* Title */}
      <h1 className="text-xl sm:text-2xl font-bold mb-4 text-gray-900 dark:text-gray-100">{dish.name}</h1>

      {/* Meta */}
      <div className="flex gap-4 sm:gap-6 text-gray-600 dark:text-gray-400 mb-4 flex-wrap">
        {dish.servings && <span>{dish.servings} servings</span>}
        {totalTime > 0 && <span>{totalTime} min</span>}
      </div>

      {/* Tags */}
      {dish.tags.length > 0 && (
        <div className="flex gap-2 flex-wrap mb-6">
          {dish.tags.map((tag) => (
            <span
              key={tag}
              className="bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 px-3 py-1.5 rounded text-sm"
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      {/* Ingredients */}
      {dish.ingredients.length > 0 && (
        <section className="mt-8">
          <h2 className="text-xl font-semibold mb-4 text-gray-900 dark:text-gray-100">Ingredients</h2>
          <ul className="list-disc pl-6 space-y-2 text-gray-700 dark:text-gray-300">
            {dish.ingredients.map((ing, idx) => (
              <li key={idx}>
                {ing.quantity} {ing.unit} {ing.name}
              </li>
            ))}
          </ul>
        </section>
      )}

      {/* Instructions */}
      {dish.instructions && (
        <section className="mt-8">
          <h2 className="text-xl font-semibold mb-4 text-gray-900 dark:text-gray-100">Instructions</h2>
          <Markdown>{dish.instructions}</Markdown>
        </section>
      )}

      {/* Nutrients */}
      {dish.nutrients.length > 0 && (
        <section className="mt-8">
          <h2 className="text-xl font-semibold mb-4 text-gray-900 dark:text-gray-100">Nutrition (per serving)</h2>
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2">
            {dish.nutrients.map((nut, idx) => (
              <div
                key={idx}
                className="bg-gray-50 dark:bg-gray-700 px-3 py-2 rounded text-gray-700 dark:text-gray-300"
              >
                {nut.name}: {nut.amount}{nut.unit}
              </div>
            ))}
          </div>
        </section>
      )}

      <ConfirmDialog
        isOpen={showDeleteDialog}
        title="Delete Dish"
        message={`Are you sure you want to delete "${dish.name}"?`}
        onConfirm={confirmDelete}
        onCancel={() => setShowDeleteDialog(false)}
      />
    </div>
  )
}
