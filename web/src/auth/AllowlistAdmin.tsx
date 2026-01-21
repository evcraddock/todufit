import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from './AuthContext'
import { getAllowlist, addToAllowlist, removeFromAllowlist, AllowedEmail } from './api'
import { ConfirmDialog } from '../components'

export function AllowlistAdmin() {
  const { auth, isLoading: authLoading } = useAuth()
  const navigate = useNavigate()
  const [emails, setEmails] = useState<AllowedEmail[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [newEmail, setNewEmail] = useState('')
  const [isAdding, setIsAdding] = useState(false)
  const [deletingEmail, setDeletingEmail] = useState<string | null>(null)
  const [removeEmailTarget, setRemoveEmailTarget] = useState<string | null>(null)
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)

  // Redirect non-admin users
  useEffect(() => {
    if (!authLoading && (!auth || !auth.isAdmin)) {
      navigate('/', { replace: true })
    }
  }, [auth, authLoading, navigate])

  const loadAllowlist = useCallback(async () => {
    try {
      setIsLoading(true)
      setError(null)
      const response = await getAllowlist()
      setEmails(response.emails)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load allowlist')
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    if (auth?.isAdmin) {
      loadAllowlist()
    }
  }, [auth?.isAdmin, loadAllowlist])

  const validateEmail = (email: string): boolean => {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
    return emailRegex.test(email)
  }

  const handleAddEmail = async (e: React.FormEvent) => {
    e.preventDefault()
    
    const email = newEmail.trim().toLowerCase()
    
    if (!email) {
      setMessage({ type: 'error', text: 'Email address is required' })
      return
    }

    if (!validateEmail(email)) {
      setMessage({ type: 'error', text: 'Please enter a valid email address' })
      return
    }

    try {
      setIsAdding(true)
      setMessage(null)
      await addToAllowlist(email)
      setMessage({ type: 'success', text: `${email} added to allowlist` })
      setNewEmail('')
      await loadAllowlist()
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : 'Failed to add email' })
    } finally {
      setIsAdding(false)
    }
  }

  const handleRemoveEmail = (email: string) => {
    setRemoveEmailTarget(email)
  }

  const confirmRemoveEmail = async () => {
    if (!removeEmailTarget) return

    try {
      setDeletingEmail(removeEmailTarget)
      setRemoveEmailTarget(null)
      setMessage(null)
      await removeFromAllowlist(removeEmailTarget)
      setMessage({ type: 'success', text: `${removeEmailTarget} removed from allowlist` })
      await loadAllowlist()
    } catch (err) {
      setMessage({ type: 'error', text: err instanceof Error ? err.message : 'Failed to remove email' })
    } finally {
      setDeletingEmail(null)
    }
  }

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    })
  }

  // Show nothing while checking auth
  if (authLoading || !auth?.isAdmin) {
    return null
  }

  return (
    <div className="max-w-4xl mx-auto">
      <h2 className="text-2xl font-semibold mb-6 text-gray-900 dark:text-gray-100">
        Manage Allowed Users
      </h2>

      {/* Message */}
      {message && (
        <div
          className={`mb-6 p-4 rounded-lg ${
            message.type === 'success'
              ? 'bg-green-100 dark:bg-green-900/30 border border-green-300 dark:border-green-700 text-green-700 dark:text-green-400'
              : 'bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-400'
          }`}
        >
          {message.text}
        </div>
      )}

      {/* Add Email Form */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-4 sm:p-6 mb-6 transition-colors">
        <h3 className="text-lg font-medium mb-4 text-gray-900 dark:text-gray-100">
          Add Email to Allowlist
        </h3>
        <form onSubmit={handleAddEmail} className="flex flex-col sm:flex-row gap-3">
          <input
            type="email"
            value={newEmail}
            onChange={(e) => setNewEmail(e.target.value)}
            placeholder="user@example.com"
            className="flex-1 px-4 py-3 min-h-[44px] border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            disabled={isAdding}
          />
          <button
            type="submit"
            disabled={isAdding || !newEmail.trim()}
            className="px-6 py-3 min-h-[44px] bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors font-medium"
          >
            {isAdding ? 'Adding...' : 'Add'}
          </button>
        </form>
      </div>

      {/* Email List */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-4 sm:p-6 transition-colors">
        <h3 className="text-lg font-medium mb-4 text-gray-900 dark:text-gray-100">
          Allowed Emails
        </h3>

        {error && (
          <div className="mb-4 p-4 bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-400 rounded-lg">
            {error}
            <button
              onClick={loadAllowlist}
              className="ml-2 underline hover:no-underline"
            >
              Retry
            </button>
          </div>
        )}

        {isLoading ? (
          <div className="text-center py-8 text-gray-500 dark:text-gray-400">
            Loading...
          </div>
        ) : emails.length === 0 ? (
          <div className="text-center py-8 text-gray-500 dark:text-gray-400">
            No emails in allowlist yet. The admin email from environment is always allowed.
          </div>
        ) : (
          <>
            {/* Mobile card layout */}
            <div className="sm:hidden space-y-3">
              {emails.map((item) => (
                <div
                  key={item.email}
                  className="border border-gray-200 dark:border-gray-700 rounded-lg p-4"
                >
                  <div className="flex justify-between items-start gap-3">
                    <div className="flex-1 min-w-0">
                      <p className="text-gray-900 dark:text-gray-100 break-all">
                        {item.email}
                      </p>
                      <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                        Added {formatDate(item.created_at)}
                      </p>
                    </div>
                    <button
                      onClick={() => handleRemoveEmail(item.email)}
                      disabled={deletingEmail === item.email}
                      className="px-3 py-2 min-h-[44px] text-red-600 dark:text-red-400 hover:text-red-800 dark:hover:text-red-300 text-sm disabled:opacity-50 flex-shrink-0"
                    >
                      {deletingEmail === item.email ? 'Removing...' : 'Remove'}
                    </button>
                  </div>
                </div>
              ))}
            </div>

            {/* Desktop table layout */}
            <div className="hidden sm:block overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-gray-200 dark:border-gray-700">
                    <th className="text-left py-3 px-4 text-sm font-medium text-gray-700 dark:text-gray-300">
                      Email
                    </th>
                    <th className="text-left py-3 px-4 text-sm font-medium text-gray-700 dark:text-gray-300">
                      Added
                    </th>
                    <th className="text-right py-3 px-4 text-sm font-medium text-gray-700 dark:text-gray-300">
                      Actions
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {emails.map((item) => (
                    <tr
                      key={item.email}
                      className="border-b border-gray-100 dark:border-gray-700 last:border-b-0"
                    >
                      <td className="py-3 px-4 text-gray-900 dark:text-gray-100">
                        {item.email}
                      </td>
                      <td className="py-3 px-4 text-gray-500 dark:text-gray-400 text-sm whitespace-nowrap">
                        {formatDate(item.created_at)}
                      </td>
                      <td className="py-3 px-4 text-right">
                        <button
                          onClick={() => handleRemoveEmail(item.email)}
                          disabled={deletingEmail === item.email}
                          className="px-3 py-2 text-red-600 dark:text-red-400 hover:text-red-800 dark:hover:text-red-300 text-sm disabled:opacity-50"
                        >
                          {deletingEmail === item.email ? 'Removing...' : 'Remove'}
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </>
        )}
      </div>

      {/* Info */}
      <p className="mt-4 text-sm text-gray-500 dark:text-gray-400">
        Only users with email addresses in this list (or the admin email from environment) can log in to the application.
      </p>

      <ConfirmDialog
        isOpen={removeEmailTarget !== null}
        title="Remove from Allowlist"
        message={`Remove ${removeEmailTarget} from the allowlist?\n\nThis user will no longer be able to log in.`}
        confirmLabel="Remove"
        onConfirm={confirmRemoveEmail}
        onCancel={() => setRemoveEmailTarget(null)}
      />
    </div>
  )
}
