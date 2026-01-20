import { useState } from 'react'
import { useAuth } from './AuthContext'
import { startPasskeyRegistration, finishPasskeyRegistration, base64ToArrayBuffer } from './api'

interface PasskeyRegisterProps {
  onSuccess?: () => void
  onError?: (error: string) => void
}

export function PasskeyRegister({ onSuccess, onError }: PasskeyRegisterProps) {
  const { auth } = useAuth()
  const [isLoading, setIsLoading] = useState(false)
  const [name, setName] = useState('')

  const handleRegister = async () => {
    if (!auth) {
      onError?.('You must be logged in to register a passkey')
      return
    }

    setIsLoading(true)

    try {
      // Check if WebAuthn is supported
      if (!window.PublicKeyCredential) {
        throw new Error('Passkeys are not supported in this browser')
      }

      // Start the registration (uses session cookie)
      const response = await startPasskeyRegistration()
      const options = response.publicKey

      // Build the credential creation options
      const publicKeyOptions: PublicKeyCredentialCreationOptions = {
        challenge: base64ToArrayBuffer(options.challenge),
        rp: {
          id: options.rp.id,
          name: options.rp.name,
        },
        user: {
          id: base64ToArrayBuffer(options.user.id),
          name: options.user.name,
          displayName: options.user.displayName,
        },
        pubKeyCredParams: options.pubKeyCredParams,
        authenticatorSelection: options.authenticatorSelection,
        timeout: options.timeout || 60000,
        attestation: options.attestation || 'none',
        excludeCredentials: options.excludeCredentials?.map((cred) => ({
          id: base64ToArrayBuffer(cred.id),
          type: cred.type,
        })),
      }

      // Create the credential
      const credential = (await navigator.credentials.create({
        publicKey: publicKeyOptions,
      })) as PublicKeyCredential | null

      if (!credential) {
        throw new Error('No credential received')
      }

      // Finish the registration (uses session cookie)
      await finishPasskeyRegistration(credential, name || undefined)

      setName('')
      onSuccess?.()
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to register passkey'
      onError?.(message)
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="space-y-4">
      <div>
        <label htmlFor="passkey-name" className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
          Passkey name (optional)
        </label>
        <input
          type="text"
          id="passkey-name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="e.g., MacBook Pro, iPhone"
          disabled={isLoading}
          className="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
        />
      </div>

      <button
        onClick={handleRegister}
        disabled={isLoading}
        className="w-full flex items-center justify-center gap-2 bg-slate-600 text-white py-3 px-4 rounded-lg hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {isLoading ? (
          <>
            <span className="animate-spin">⏳</span>
            Registering...
          </>
        ) : (
          <>
            <span>🔐</span>
            Register Passkey
          </>
        )}
      </button>
    </div>
  )
}
