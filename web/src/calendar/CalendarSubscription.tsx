/**
 * Calendar subscription management component.
 *
 * Allows users to generate, view, copy, and regenerate calendar subscription URLs.
 */

import { useState, useEffect, useCallback } from 'react'
import { getCalendarToken, createCalendarToken, revokeCalendarToken } from './api'
import { ConfirmDialog } from '../components'

interface CalendarSubscriptionProps {
  groupDocId: string
  groupName: string
}

export function CalendarSubscription({ groupDocId, groupName }: CalendarSubscriptionProps) {
  const [isLoading, setIsLoading] = useState(true)
  const [subscriptionUrl, setSubscriptionUrl] = useState<string | null>(null)
  const [httpUrl, setHttpUrl] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [successMessage, setSuccessMessage] = useState<string | null>(null)
  const [isCreating, setIsCreating] = useState(false)
  const [showRegenerateConfirm, setShowRegenerateConfirm] = useState(false)
  const [showRevokeConfirm, setShowRevokeConfirm] = useState(false)
  const [copied, setCopied] = useState(false)

  const loadToken = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      const data = await getCalendarToken(groupDocId)
      setSubscriptionUrl(data.subscription_url || null)
      setHttpUrl(data.http_url || null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load calendar token')
    } finally {
      setIsLoading(false)
    }
  }, [groupDocId])

  useEffect(() => {
    loadToken()
  }, [loadToken])

  const handleCreate = async () => {
    setIsCreating(true)
    setError(null)
    setSuccessMessage(null)
    try {
      const data = await createCalendarToken(groupDocId, false)
      setSubscriptionUrl(data.subscription_url)
      setHttpUrl(data.http_url)
      setSuccessMessage('Calendar subscription created!')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create calendar token')
    } finally {
      setIsCreating(false)
    }
  }

  const handleRegenerate = async () => {
    setShowRegenerateConfirm(false)
    setIsCreating(true)
    setError(null)
    setSuccessMessage(null)
    try {
      const data = await createCalendarToken(groupDocId, true)
      setSubscriptionUrl(data.subscription_url)
      setHttpUrl(data.http_url)
      setSuccessMessage('Calendar subscription URL regenerated. Previous URL no longer works.')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to regenerate calendar token')
    } finally {
      setIsCreating(false)
    }
  }

  const handleRevoke = async () => {
    setShowRevokeConfirm(false)
    setIsCreating(true)
    setError(null)
    setSuccessMessage(null)
    try {
      await revokeCalendarToken(groupDocId)
      setSubscriptionUrl(null)
      setHttpUrl(null)
      setSuccessMessage('Calendar subscription revoked.')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to revoke calendar token')
    } finally {
      setIsCreating(false)
    }
  }

  const handleCopy = async () => {
    if (!subscriptionUrl) return
    try {
      await navigator.clipboard.writeText(subscriptionUrl)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      // Fallback for older browsers
      const textArea = document.createElement('textarea')
      textArea.value = subscriptionUrl
      document.body.appendChild(textArea)
      textArea.select()
      document.execCommand('copy')
      document.body.removeChild(textArea)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  if (isLoading) {
    return (
      <div className="animate-pulse">
        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/4 mb-2"></div>
        <div className="h-10 bg-gray-200 dark:bg-gray-700 rounded"></div>
      </div>
    )
  }

  return (
    <div>
      <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        Calendar Subscription
      </h4>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-3">
        Subscribe to "{groupName}" meal plans in your calendar app (Apple Calendar, Google Calendar, etc.)
      </p>

      {error && (
        <div className="mb-3 p-3 bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-400 rounded-lg text-sm">
          {error}
        </div>
      )}

      {successMessage && (
        <div className="mb-3 p-3 bg-green-100 dark:bg-green-900/30 border border-green-300 dark:border-green-700 text-green-700 dark:text-green-400 rounded-lg text-sm">
          {successMessage}
        </div>
      )}

      {subscriptionUrl ? (
        <div className="space-y-3">
          {/* Subscription URL Display */}
          <div className="flex items-center gap-2">
            <input
              type="text"
              readOnly
              value={subscriptionUrl}
              className="flex-1 px-3 py-2 text-sm font-mono bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-300"
            />
            <button
              onClick={handleCopy}
              className="px-3 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 transition-colors min-w-[80px]"
            >
              {copied ? 'Copied!' : 'Copy'}
            </button>
          </div>

          {/* Instructions */}
          <div className="text-sm text-gray-500 dark:text-gray-400 space-y-1">
            <p><strong>To subscribe:</strong></p>
            <ul className="list-disc list-inside ml-2 space-y-1">
              <li><strong>Apple Calendar:</strong> File → New Calendar Subscription → Paste URL</li>
              <li><strong>Google Calendar:</strong> Settings → Add calendar → From URL → Paste URL</li>
            </ul>
          </div>

          {/* Direct HTTP link for testing */}
          {httpUrl && (
            <p className="text-xs text-gray-400 dark:text-gray-500">
              Direct link:{' '}
              <a
                href={httpUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-500 hover:underline font-mono"
              >
                {httpUrl}
              </a>
            </p>
          )}

          {/* Actions */}
          <div className="flex gap-2 pt-2">
            <button
              onClick={() => setShowRegenerateConfirm(true)}
              disabled={isCreating}
              className="px-3 py-1.5 text-sm text-amber-600 dark:text-amber-400 hover:text-amber-800 dark:hover:text-amber-300 disabled:opacity-50"
            >
              Regenerate URL
            </button>
            <button
              onClick={() => setShowRevokeConfirm(true)}
              disabled={isCreating}
              className="px-3 py-1.5 text-sm text-red-600 dark:text-red-400 hover:text-red-800 dark:hover:text-red-300 disabled:opacity-50"
            >
              Revoke
            </button>
          </div>
        </div>
      ) : (
        <button
          onClick={handleCreate}
          disabled={isCreating}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 transition-colors"
        >
          {isCreating ? 'Creating...' : 'Create Subscription URL'}
        </button>
      )}

      {/* Regenerate Confirmation */}
      <ConfirmDialog
        isOpen={showRegenerateConfirm}
        title="Regenerate Calendar URL"
        message="This will invalidate the current subscription URL. Anyone using the old URL will need to re-subscribe with the new one."
        confirmLabel="Regenerate"
        isDestructive={false}
        onConfirm={handleRegenerate}
        onCancel={() => setShowRegenerateConfirm(false)}
      />

      {/* Revoke Confirmation */}
      <ConfirmDialog
        isOpen={showRevokeConfirm}
        title="Revoke Calendar Subscription"
        message="This will disable the calendar subscription. Anyone using this URL will no longer receive updates."
        confirmLabel="Revoke"
        onConfirm={handleRevoke}
        onCancel={() => setShowRevokeConfirm(false)}
      />
    </div>
  )
}
