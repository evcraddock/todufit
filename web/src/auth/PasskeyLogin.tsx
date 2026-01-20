import { useState } from 'react'
import { startPasskeyAuth, finishPasskeyAuth, base64ToArrayBuffer } from './api'

interface PasskeyLoginProps {
  onSuccess?: () => void
  onError?: (error: string) => void
}

export function PasskeyLogin({ onSuccess, onError }: PasskeyLoginProps) {
  const [isLoading, setIsLoading] = useState(false)

  const handlePasskeyLogin = async () => {
    setIsLoading(true)

    try {
      // Check if WebAuthn is supported
      if (!window.PublicKeyCredential) {
        throw new Error('Passkeys are not supported in this browser')
      }

      // Start passkey authentication - server returns challenge
      const response = await startPasskeyAuth()
      const options = response.publicKey

      // Build the credential request options
      const publicKeyOptions: PublicKeyCredentialRequestOptions = {
        challenge: base64ToArrayBuffer(options.challenge),
        rpId: options.rpId,
        allowCredentials: options.allowCredentials?.map((cred) => ({
          id: base64ToArrayBuffer(cred.id),
          type: cred.type,
        })),
        userVerification: options.userVerification || 'preferred',
        timeout: options.timeout || 60000,
      }

      // Request the credential from the browser
      const credential = (await navigator.credentials.get({
        publicKey: publicKeyOptions,
      })) as PublicKeyCredential | null

      if (!credential) {
        throw new Error('No credential received')
      }

      // Extract email from userHandle
      const assertionResponse = credential.response as AuthenticatorAssertionResponse
      if (!assertionResponse.userHandle) {
        throw new Error('No user handle in passkey response')
      }

      const email = new TextDecoder().decode(assertionResponse.userHandle)
      if (!email) {
        throw new Error('Could not extract email from passkey')
      }

      // Finish the authentication - this sets session cookie
      await finishPasskeyAuth(credential, email)
      onSuccess?.()
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Passkey login failed'
      onError?.(message)
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <button
      onClick={handlePasskeyLogin}
      disabled={isLoading}
      className="w-full flex items-center justify-center gap-2 bg-gray-800 dark:bg-gray-700 text-white py-3 px-4 rounded-lg hover:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
    >
      {isLoading ? (
        <>
          <span className="animate-spin">⏳</span>
          Authenticating...
        </>
      ) : (
        <>
          <span>🔐</span>
          Login with Passkey
        </>
      )}
    </button>
  )
}
