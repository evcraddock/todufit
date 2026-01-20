import { AuthState, PasskeyInfo } from './types'

const API_BASE = '/auth'

// ============================================================================
// Response Types
// ============================================================================

export interface PasskeyStartResponse {
  publicKey: {
    challenge: string
    rpId?: string
    timeout?: number
    userVerification?: 'discouraged' | 'preferred' | 'required'
    allowCredentials?: Array<{
      id: string
      type: 'public-key'
    }>
  }
}

export interface MagicLinkResponse {
  success: boolean
}

export interface MeResponse {
  user_id: string
  email: string
  root_doc_id: string | null
  current_group_id: string | null
  is_admin: boolean
  passkeys: PasskeyInfo[]
}

export interface PasskeyRegistrationResponse {
  publicKey: {
    challenge: string
    rp: {
      id: string
      name: string
    }
    user: {
      id: string
      name: string
      displayName: string
    }
    pubKeyCredParams: Array<{
      type: 'public-key'
      alg: number
    }>
    authenticatorSelection?: {
      authenticatorAttachment?: 'platform' | 'cross-platform'
      residentKey?: 'discouraged' | 'preferred' | 'required'
      requireResidentKey?: boolean
      userVerification?: 'discouraged' | 'preferred' | 'required'
    }
    timeout?: number
    attestation?: 'none' | 'indirect' | 'direct' | 'enterprise'
    excludeCredentials?: Array<{
      id: string
      type: 'public-key'
    }>
  }
}

export interface PasskeyRegistrationFinishResponse {
  id: string
  name: string | null
}

// ============================================================================
// Helper Functions
// ============================================================================

async function parseErrorResponse(response: Response): Promise<string> {
  const text = await response.text()
  try {
    const json = JSON.parse(text)
    return json.message || json.error || text
  } catch {
    return text || 'Unknown error'
  }
}

// ============================================================================
// Auth API Functions
// ============================================================================

/**
 * Request a magic link email for login
 */
export async function requestMagicLink(email: string): Promise<MagicLinkResponse> {
  const response = await fetch(`${API_BASE}/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email }),
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }

  return response.json()
}

/**
 * Fetch current user info (checks session cookie)
 */
export async function fetchMe(): Promise<AuthState> {
  const response = await fetch(`${API_BASE}/me`, {
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }

  const data: MeResponse = await response.json()

  return {
    userId: data.user_id,
    email: data.email,
    rootDocId: data.root_doc_id,
    currentGroupId: data.current_group_id,
    isAdmin: data.is_admin,
    passkeys: data.passkeys,
  }
}

/**
 * Logout (clears session cookie)
 */
export async function logoutApi(): Promise<void> {
  await fetch(`${API_BASE}/logout`, {
    method: 'POST',
    credentials: 'include',
  })
}

// ============================================================================
// Passkey Authentication
// ============================================================================

/**
 * Start passkey authentication (get challenge)
 */
export async function startPasskeyAuth(email?: string): Promise<PasskeyStartResponse> {
  const response = await fetch(`${API_BASE}/passkey/auth/start`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email }),
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }

  return response.json()
}

/**
 * Finish passkey authentication (verify credential, creates session)
 */
export async function finishPasskeyAuth(
  credential: PublicKeyCredential,
  email: string
): Promise<AuthState> {
  const response = credential.response as AuthenticatorAssertionResponse

  const payload = {
    id: credential.id,
    rawId: arrayBufferToBase64Url(credential.rawId),
    type: credential.type,
    response: {
      authenticatorData: arrayBufferToBase64Url(response.authenticatorData),
      clientDataJSON: arrayBufferToBase64Url(response.clientDataJSON),
      signature: arrayBufferToBase64Url(response.signature),
      userHandle: response.userHandle ? arrayBufferToBase64Url(response.userHandle) : null,
    },
    email,
  }

  const finishResponse = await fetch(`${API_BASE}/passkey/auth/finish`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify(payload),
  })

  if (!finishResponse.ok) {
    const error = await parseErrorResponse(finishResponse)
    throw new Error(error)
  }

  // Server sets session cookie and returns user info
  // Fetch full user info from /auth/me
  return fetchMe()
}

// ============================================================================
// Passkey Registration
// ============================================================================

/**
 * Start passkey registration (requires session)
 */
export async function startPasskeyRegistration(): Promise<PasskeyRegistrationResponse> {
  const response = await fetch(`${API_BASE}/passkey/register/start`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }

  return response.json()
}

/**
 * Finish passkey registration (requires session)
 */
export async function finishPasskeyRegistration(
  credential: PublicKeyCredential,
  name?: string
): Promise<PasskeyRegistrationFinishResponse> {
  const response = credential.response as AuthenticatorAttestationResponse

  const payload = {
    id: credential.id,
    rawId: arrayBufferToBase64Url(credential.rawId),
    type: credential.type,
    response: {
      attestationObject: arrayBufferToBase64Url(response.attestationObject),
      clientDataJSON: arrayBufferToBase64Url(response.clientDataJSON),
    },
    name,
  }

  const finishResponse = await fetch(`${API_BASE}/passkey/register/finish`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify(payload),
  })

  if (!finishResponse.ok) {
    const error = await parseErrorResponse(finishResponse)
    throw new Error(error)
  }

  return finishResponse.json()
}

/**
 * Delete a passkey (requires session)
 */
export async function deletePasskey(passkeyId: string): Promise<void> {
  const response = await fetch(`${API_BASE}/passkey/${encodeURIComponent(passkeyId)}`, {
    method: 'DELETE',
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }
}

// ============================================================================
// User Settings
// ============================================================================

export interface SetRootDocIdResponse {
  success: boolean
  root_doc_id: string
}

export interface SetRootDocIdError {
  error: string
  requires_confirmation?: boolean
  current_root_doc_id?: string
}

/**
 * Set or update the user's root doc ID
 */
export async function setRootDocId(
  rootDocId: string,
  confirm?: boolean
): Promise<SetRootDocIdResponse> {
  const response = await fetch(`${API_BASE}/settings/root-doc-id`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ root_doc_id: rootDocId, confirm }),
  })

  if (!response.ok) {
    const data = await response.json()
    if (data.requires_confirmation) {
      const error = new Error(data.error) as Error & SetRootDocIdError
      error.requires_confirmation = true
      error.current_root_doc_id = data.current_root_doc_id
      throw error
    }
    throw new Error(data.error || 'Failed to set root doc ID')
  }

  return response.json()
}

/**
 * Set the user's current group ID
 */
export async function setCurrentGroup(currentGroupId: string): Promise<{ success: boolean }> {
  const response = await fetch(`${API_BASE}/settings/current-group`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ current_group_id: currentGroupId }),
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }

  return response.json()
}

// ============================================================================
// Allowlist Management (Admin Only)
// ============================================================================

export interface AllowedEmail {
  email: string
  created_at: string
}

export interface AllowlistResponse {
  emails: AllowedEmail[]
}

/**
 * Get list of allowed emails (admin only)
 */
export async function getAllowlist(): Promise<AllowlistResponse> {
  const response = await fetch(`${API_BASE}/allowlist`, {
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }

  return response.json()
}

/**
 * Add an email to the allowlist (admin only)
 */
export async function addToAllowlist(email: string): Promise<{ success: boolean }> {
  const response = await fetch(`${API_BASE}/allowlist`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ email }),
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }

  return response.json()
}

/**
 * Remove an email from the allowlist (admin only)
 */
export async function removeFromAllowlist(email: string): Promise<{ success: boolean }> {
  const response = await fetch(`${API_BASE}/allowlist/${encodeURIComponent(email)}`, {
    method: 'DELETE',
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }

  return response.json()
}

// ============================================================================
// Group Invites
// ============================================================================

/**
 * Send a group invitation email
 */
export async function sendGroupInvite(
  email: string,
  groupDocId: string,
  groupName: string
): Promise<{ success: boolean }> {
  const response = await fetch(`${API_BASE}/groups/invite`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({
      email,
      group_doc_id: groupDocId,
      group_name: groupName,
    }),
  })

  if (!response.ok) {
    const error = await parseErrorResponse(response)
    throw new Error(error)
  }

  return response.json()
}

// ============================================================================
// Utility Functions
// ============================================================================

function arrayBufferToBase64Url(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer)
  let binary = ''
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  // Convert to URL-safe base64 (no padding)
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

export function base64ToArrayBuffer(base64: string): ArrayBuffer {
  // Convert URL-safe base64 to standard base64
  const standardBase64 = base64.replace(/-/g, '+').replace(/_/g, '/')
  // Add padding if needed
  const padded = standardBase64 + '=='.slice(0, (4 - (standardBase64.length % 4)) % 4)
  const binary = atob(padded)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes.buffer
}
