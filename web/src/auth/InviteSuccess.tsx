import { useEffect, useState } from 'react'
import { useSearchParams, useNavigate } from 'react-router-dom'
import { useAuth } from './AuthContext'

export function InviteSuccess() {
  const [searchParams] = useSearchParams()
  const navigate = useNavigate()
  const { isAuthenticated, isLoading } = useAuth()
  const [status, setStatus] = useState<'loading' | 'success' | 'error'>('loading')
  const [errorMessage, setErrorMessage] = useState<string | null>(null)

  const groupDocId = searchParams.get('group_doc_id')
  const groupName = searchParams.get('group_name')

  useEffect(() => {
    // Wait for auth to finish loading
    if (isLoading) return

    // If not authenticated, redirect to login
    if (!isAuthenticated) {
      navigate('/login')
      return
    }

    // Validate params
    if (!groupDocId || !groupName) {
      setStatus('error')
      setErrorMessage('Missing group information')
      return
    }

    // Store the pending group in localStorage for RepoContext to pick up
    // The RepoContext will handle adding this group to the identity document
    localStorage.setItem('pendingJoinGroupDocId', groupDocId)
    localStorage.setItem('pendingJoinGroupName', groupName)

    setStatus('success')
  }, [isLoading, isAuthenticated, groupDocId, groupName, navigate])

  const handleContinue = () => {
    // Navigate to home, which will trigger RepoContext to process the pending group
    window.location.href = '/'
  }

  if (isLoading || status === 'loading') {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-md">
        <div className="text-center">
          <div className="text-gray-500 dark:text-gray-400">Loading...</div>
        </div>
      </div>
    )
  }

  if (status === 'error') {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-md">
        <div className="text-center">
          <div className="text-5xl mb-4">❌</div>
          <h2 className="text-2xl font-semibold mb-2 text-gray-900 dark:text-gray-100">
            Something went wrong
          </h2>
          <p className="text-gray-600 dark:text-gray-400 mb-6">
            {errorMessage || 'Failed to join the group'}
          </p>
          <button
            onClick={() => navigate('/')}
            className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          >
            Go Home
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-md mx-auto mt-12 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-md">
      <div className="text-center">
        <div className="text-5xl mb-4">🎉</div>
        <h2 className="text-2xl font-semibold mb-2 text-gray-900 dark:text-gray-100">
          You've joined "{groupName}"!
        </h2>
        <p className="text-gray-600 dark:text-gray-400 mb-6">
          You can now share dishes, meal plans, and shopping lists with this group.
        </p>
        <button
          onClick={handleContinue}
          className="w-full px-6 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors"
        >
          Continue to App
        </button>
      </div>
    </div>
  )
}
