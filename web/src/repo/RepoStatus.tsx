import { ReactNode } from 'react'
import { useRepoState } from './RepoContext'

interface RepoStatusProps {
  children: ReactNode
}

/**
 * Wrapper component that shows loading/error states for repo initialization.
 * Renders children only when repo is ready.
 */
export function RepoStatus({ children }: RepoStatusProps) {
  const { isReady, status, error } = useRepoState()

  if (status === 'idle') {
    return (
      <div className="flex items-center justify-center min-h-[50vh]">
        <div className="text-gray-500 dark:text-gray-400">Initializing...</div>
      </div>
    )
  }

  if (status === 'loading') {
    return (
      <div className="flex items-center justify-center min-h-[50vh]">
        <div className="text-center">
          <div className="animate-spin h-8 w-8 border-4 border-blue-500 border-t-transparent rounded-full mx-auto mb-4"></div>
          <div className="text-gray-500 dark:text-gray-400">Connecting to sync server...</div>
        </div>
      </div>
    )
  }

  if (status === 'pending_sync') {
    return (
      <div className="flex items-center justify-center min-h-[50vh]">
        <div className="text-center max-w-md">
          <div className="animate-pulse text-4xl mb-4">🔄</div>
          <h2 className="text-xl font-semibold mb-2 text-gray-900 dark:text-gray-100">Syncing Identity</h2>
          <p className="text-gray-600 dark:text-gray-400 mb-4">
            Waiting for your identity data to sync from other devices.
            This may take a moment...
          </p>
          {error && (
            <p className="text-sm text-gray-500 dark:text-gray-400">{error}</p>
          )}
        </div>
      </div>
    )
  }

  if (status === 'error') {
    return (
      <div className="flex items-center justify-center min-h-[50vh]">
        <div className="text-center max-w-md">
          <div className="text-4xl mb-4">⚠️</div>
          <h2 className="text-xl font-semibold mb-2 text-red-600 dark:text-red-400">Connection Error</h2>
          <p className="text-gray-600 dark:text-gray-400 mb-4">
            {error || 'Failed to connect to the sync server.'}
          </p>
          <button
            onClick={() => window.location.reload()}
            className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 transition-colors"
          >
            Retry
          </button>
        </div>
      </div>
    )
  }

  if (!isReady) {
    return (
      <div className="flex items-center justify-center min-h-[50vh]">
        <div className="text-gray-500 dark:text-gray-400">Connecting...</div>
      </div>
    )
  }

  return <>{children}</>
}

/**
 * Simple loading indicator for inline use
 */
export function RepoLoading() {
  const { status, error } = useRepoState()

  if (status === 'pending_sync') {
    return (
      <div className="text-center py-12 text-gray-500 dark:text-gray-400">
        <div className="animate-pulse">Syncing data...</div>
        {error && <div className="text-sm text-gray-400 mt-2">{error}</div>}
      </div>
    )
  }

  if (status === 'error') {
    return (
      <div className="text-center py-12 text-red-500 dark:text-red-400">
        {error || 'Connection error'}
      </div>
    )
  }

  return (
    <div className="text-center py-12 text-gray-500 dark:text-gray-400">
      Connecting...
    </div>
  )
}
