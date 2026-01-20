import { useState, FormEvent, useEffect, useRef } from 'react'
import { useNavigate, useLocation } from 'react-router-dom'
import { useAuth } from './AuthContext'
import { PasskeyLogin } from './PasskeyLogin'
import { PasskeyRegister } from './PasskeyRegister'
import { requestMagicLink } from './api'

export function Login() {
  const { isAuthenticated, isLoading, auth, refreshAuth } = useAuth()
  const navigate = useNavigate()
  const location = useLocation()

  const [email, setEmail] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [magicLinkSent, setMagicLinkSent] = useState(false)
  const [showPasskeyPrompt, setShowPasskeyPrompt] = useState(false)
  const [justLoggedIn, setJustLoggedIn] = useState(false)
  const emailInputRef = useRef<HTMLInputElement>(null)

  // Get the intended destination from state, or default to home
  const from = (location.state as { from?: Location })?.from?.pathname || '/'

  // Check if we just came from a magic link verification
  // The server redirects to / after successful verification, so check if we're authenticated
  useEffect(() => {
    // If authenticated and has no passkeys, show passkey prompt
    if (isAuthenticated && auth && auth.passkeys.length === 0 && justLoggedIn) {
      setShowPasskeyPrompt(true)
    } else if (isAuthenticated && !showPasskeyPrompt) {
      navigate(from, { replace: true })
    }
  }, [isAuthenticated, auth, justLoggedIn, showPasskeyPrompt, navigate, from])

  // Handle programmatic input changes (for browser automation/testing)
  // React's onChange doesn't fire when input.value is set directly followed by dispatchEvent
  useEffect(() => {
    const input = emailInputRef.current
    if (!input) return

    const handleNativeInput = (e: Event) => {
      setEmail((e.target as HTMLInputElement).value)
    }

    input.addEventListener('input', handleNativeInput)
    return () => input.removeEventListener('input', handleNativeInput)
  }, [])

  const handleMagicLinkSubmit = async (e?: FormEvent) => {
    e?.preventDefault()
    if (!email.trim()) return

    setIsSubmitting(true)
    setError(null)

    try {
      await requestMagicLink(email)
      setMagicLinkSent(true)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send magic link')
    } finally {
      setIsSubmitting(false)
    }
  }

  const handlePasskeySuccess = async () => {
    // Refresh auth state after passkey login
    await refreshAuth()
    setJustLoggedIn(true)
  }

  const handlePasskeyError = (message: string) => {
    setError(message)
  }

  // Show loading while checking auth state
  if (isLoading) {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-md transition-colors">
        <div className="text-center">
          <div className="animate-spin text-4xl mb-4">⏳</div>
          <p className="text-gray-600 dark:text-gray-400">Checking authentication...</p>
        </div>
      </div>
    )
  }

  if (showPasskeyPrompt) {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-md transition-colors">
        <div className="text-center">
          <div className="text-5xl mb-4">🔐</div>
          <h2 className="text-2xl font-semibold mb-2 text-gray-900 dark:text-gray-100">Set up faster login</h2>
          <p className="text-gray-600 dark:text-gray-400 mb-6">
            Register a passkey to sign in instantly next time — no email required.
          </p>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-400 rounded-lg">
            {error}
          </div>
        )}

        <PasskeyRegister
          onSuccess={() => navigate(from, { replace: true })}
          onError={(err) => setError(err)}
        />

        <button
          onClick={() => navigate(from, { replace: true })}
          className="w-full mt-4 text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
        >
          Skip for now
        </button>
      </div>
    )
  }

  if (magicLinkSent) {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-md transition-colors">
        <div className="text-center">
          <div className="text-5xl mb-4">📧</div>
          <h2 className="text-2xl font-semibold mb-2 text-gray-900 dark:text-gray-100">Check your email</h2>
          <p className="text-gray-600 dark:text-gray-400 mb-4">
            We sent a login link to <strong>{email}</strong>
          </p>
          <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
            Click the link in the email to sign in. The link will expire in 15 minutes.
          </p>
          <button
            onClick={() => {
              setMagicLinkSent(false)
              setEmail('')
            }}
            className="text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300"
          >
            Use a different email
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-md mx-auto mt-12 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-md transition-colors">
      <h2 className="text-2xl font-semibold text-center mb-6 text-gray-900 dark:text-gray-100">Sign in to Todu Fit</h2>

      {error && (
        <div className="mb-4 p-3 bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-400 rounded-lg">
          {error}
        </div>
      )}

      {/* Passkey Login - no email needed */}
      <div className="mb-4">
        <PasskeyLogin onSuccess={handlePasskeySuccess} onError={handlePasskeyError} />
      </div>

      <div className="relative mb-4">
        <div className="absolute inset-0 flex items-center">
          <div className="w-full border-t border-gray-300 dark:border-gray-600"></div>
        </div>
        <div className="relative flex justify-center text-sm">
          <span className="px-2 bg-white dark:bg-gray-800 text-gray-500 dark:text-gray-400">or use email</span>
        </div>
      </div>

      {/* Email Input for Magic Link */}
      <div className="mb-4">
        <label htmlFor="email" className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
          Email address
        </label>
        <input
          ref={emailInputRef}
          type="email"
          id="email"
          data-testid="email-input"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          onInput={(e) => setEmail((e.target as HTMLInputElement).value)}
          placeholder="you@example.com"
          disabled={isSubmitting}
          className="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
        />
      </div>

      {/* Magic Link Button */}
      <button
        onClick={handleMagicLinkSubmit}
        disabled={isSubmitting || !email.trim()}
        data-testid="send-magic-link-button"
        className="w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {isSubmitting ? 'Sending...' : 'Send Magic Link'}
      </button>
    </div>
  )
}
