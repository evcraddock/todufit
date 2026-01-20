import { Hono } from 'hono'
import { eq, and, isNull } from 'drizzle-orm'
import { randomUUID } from 'crypto'
import {
  generateRegistrationOptions,
  verifyRegistrationResponse,
  generateAuthenticationOptions,
  verifyAuthenticationResponse,
  type VerifiedRegistrationResponse,
  type VerifiedAuthenticationResponse,
} from '@simplewebauthn/server'
import type {
  RegistrationResponseJSON,
  AuthenticationResponseJSON,
} from '@simplewebauthn/types'
import {
  db,
  users,
  passkeys,
  allowedEmails,
  userSettings,
  type NewUser,
  type NewPasskey,
  type NewUserSettings,
} from '../db'
import { requireSession, type SessionContext } from '../middleware/session'
import { createSession } from '../lib/session'

// WebAuthn configuration
const RP_NAME = process.env.RP_NAME || 'todu-fit'
const RP_ID = process.env.RP_ID || 'localhost'
const RP_ORIGIN = process.env.PUBLIC_URL || 'http://localhost:5173'

// Admin email from environment
const ADMIN_EMAIL = process.env.ADMIN_EMAIL

// In-memory challenge storage (in production, use Redis or DB)
// Maps email -> challenge for authentication
// Maps sessionId -> challenge for registration
const authChallenges = new Map<string, { challenge: string; expiresAt: number }>()
const regChallenges = new Map<string, { challenge: string; expiresAt: number }>()

// Clean up expired challenges periodically
setInterval(() => {
  const now = Date.now()
  for (const [key, value] of authChallenges) {
    if (value.expiresAt < now) authChallenges.delete(key)
  }
  for (const [key, value] of regChallenges) {
    if (value.expiresAt < now) regChallenges.delete(key)
  }
}, 60000) // Every minute

// Create passkey router
export const passkeyRoutes = new Hono()

// ============================================================================
// Passkey Authentication (Login)
// ============================================================================

/**
 * POST /auth/passkey/auth/start
 * Start passkey authentication flow
 */
passkeyRoutes.post('/auth/start', async (c) => {
  const body = await c.req.json<{ email?: string }>()
  const email = body.email?.toLowerCase().trim()

  // Get allowed credentials for the user (if email provided)
  let allowCredentials: { id: string }[] = []

  if (email) {
    const user = await db.select().from(users).where(eq(users.email, email)).get()

    if (user) {
      const userPasskeys = await db
        .select()
        .from(passkeys)
        .where(eq(passkeys.userId, user.id))

      allowCredentials = userPasskeys.map((p) => ({
        id: p.credentialId,
      }))
    }
  }

  const options = await generateAuthenticationOptions({
    rpID: RP_ID,
    allowCredentials: allowCredentials.length > 0 ? allowCredentials : undefined,
    userVerification: 'preferred',
  })

  // Store challenge (keyed by email or a placeholder)
  const key = email || 'anonymous'
  authChallenges.set(key, {
    challenge: options.challenge,
    expiresAt: Date.now() + 5 * 60 * 1000, // 5 minutes
  })

  return c.json({ publicKey: options })
})

/**
 * POST /auth/passkey/auth/finish
 * Complete passkey authentication
 */
passkeyRoutes.post('/auth/finish', async (c) => {
  const body = await c.req.json<{
    email: string
    id: string
    rawId: string
    type: string
    response: {
      authenticatorData: string
      clientDataJSON: string
      signature: string
      userHandle?: string | null
    }
  }>()

  const email = body.email?.toLowerCase().trim()

  if (!email) {
    return c.json({ error: 'Email is required' }, 400)
  }

  // Check if email is allowed
  const isAdmin = email === ADMIN_EMAIL?.toLowerCase()

  if (!isAdmin) {
    const allowed = await db
      .select()
      .from(allowedEmails)
      .where(and(eq(allowedEmails.email, email), isNull(allowedEmails.revokedAt)))
      .get()

    if (!allowed) {
      return c.json({ error: 'Email not allowed' }, 403)
    }
  }

  // Find user
  const user = await db.select().from(users).where(eq(users.email, email)).get()

  if (!user) {
    return c.json({ error: 'User not found' }, 404)
  }

  // Find the credential
  console.log('[passkey] Looking up credential with rawId:', body.rawId)
  
  // Debug: list all passkeys for this user
  const userPasskeys = await db
    .select()
    .from(passkeys)
    .where(eq(passkeys.userId, user.id))
  console.log('[passkey] User passkeys:', userPasskeys.map(p => ({ id: p.id, credentialId: p.credentialId })))
  
  const passkey = await db
    .select()
    .from(passkeys)
    .where(eq(passkeys.credentialId, body.rawId))
    .get()

  if (!passkey || passkey.userId !== user.id) {
    console.log('[passkey] Credential not found. passkey:', passkey, 'userId:', user.id)
    return c.json({ error: 'Credential not found' }, 404)
  }

  // Get stored challenge
  const storedChallenge = authChallenges.get(email) || authChallenges.get('anonymous')

  if (!storedChallenge || storedChallenge.expiresAt < Date.now()) {
    return c.json({ error: 'Challenge expired' }, 400)
  }

  // Build the response object for verification
  const authResponse: AuthenticationResponseJSON = {
    id: body.id,
    rawId: body.rawId,
    type: body.type as 'public-key',
    response: {
      authenticatorData: body.response.authenticatorData,
      clientDataJSON: body.response.clientDataJSON,
      signature: body.response.signature,
      userHandle: body.response.userHandle ?? undefined,
    },
    clientExtensionResults: {},
    authenticatorAttachment: 'platform',
  }

  let verification: VerifiedAuthenticationResponse

  try {
    verification = await verifyAuthenticationResponse({
      response: authResponse,
      expectedChallenge: storedChallenge.challenge,
      expectedOrigin: RP_ORIGIN,
      expectedRPID: RP_ID,
      credential: {
        id: passkey.credentialId,
        publicKey: new Uint8Array(passkey.publicKey),
        counter: passkey.counter,
      },
    })
  } catch (error) {
    console.error('[passkey] Verification error:', error)
    return c.json({ error: 'Verification failed' }, 400)
  }

  if (!verification.verified) {
    return c.json({ error: 'Verification failed' }, 400)
  }

  // Update counter and last used
  await db
    .update(passkeys)
    .set({
      counter: verification.authenticationInfo.newCounter,
      lastUsedAt: new Date(),
    })
    .where(eq(passkeys.id, passkey.id))

  // Update user last login
  await db.update(users).set({ lastLoginAt: new Date() }).where(eq(users.id, user.id))

  // Clean up challenge
  authChallenges.delete(email)
  authChallenges.delete('anonymous')

  // Create session
  await createSession(c, user.id)

  // Get user settings
  const settings = await db
    .select()
    .from(userSettings)
    .where(eq(userSettings.userId, user.id))
    .get()

  return c.json({
    user_id: user.id,
    email: user.email,
    root_doc_id: settings?.rootDocId,
    current_group_id: settings?.currentGroupId,
  })
})

// ============================================================================
// Passkey Registration (requires existing session)
// ============================================================================

/**
 * POST /auth/passkey/register/start
 * Start passkey registration (requires auth)
 */
passkeyRoutes.post('/register/start', requireSession, async (c) => {
  const session = c.get('session') as SessionContext

  // Get existing passkeys for exclusion
  const existingPasskeys = await db
    .select()
    .from(passkeys)
    .where(eq(passkeys.userId, session.user.id))

  const excludeCredentials = existingPasskeys.map((p) => ({
    id: p.credentialId,
  }))

  const options = await generateRegistrationOptions({
    rpID: RP_ID,
    rpName: RP_NAME,
    userID: new TextEncoder().encode(session.user.email),
    userName: session.user.email,
    userDisplayName: session.user.email,
    attestationType: 'none',
    excludeCredentials,
    authenticatorSelection: {
      residentKey: 'preferred',
      userVerification: 'preferred',
    },
  })

  // Store challenge keyed by session
  regChallenges.set(session.sessionId, {
    challenge: options.challenge,
    expiresAt: Date.now() + 5 * 60 * 1000, // 5 minutes
  })

  return c.json({ publicKey: options })
})

/**
 * POST /auth/passkey/register/finish
 * Complete passkey registration (requires auth)
 */
passkeyRoutes.post('/register/finish', requireSession, async (c) => {
  const session = c.get('session') as SessionContext

  const body = await c.req.json<{
    id: string
    rawId: string
    type: string
    response: {
      attestationObject: string
      clientDataJSON: string
    }
    name?: string
  }>()

  // Get stored challenge
  const storedChallenge = regChallenges.get(session.sessionId)

  if (!storedChallenge || storedChallenge.expiresAt < Date.now()) {
    return c.json({ error: 'Challenge expired' }, 400)
  }

  // Build the response object for verification
  const regResponse: RegistrationResponseJSON = {
    id: body.id,
    rawId: body.rawId,
    type: body.type as 'public-key',
    response: {
      attestationObject: body.response.attestationObject,
      clientDataJSON: body.response.clientDataJSON,
    },
    clientExtensionResults: {},
    authenticatorAttachment: 'platform',
  }

  let verification: VerifiedRegistrationResponse

  try {
    verification = await verifyRegistrationResponse({
      response: regResponse,
      expectedChallenge: storedChallenge.challenge,
      expectedOrigin: RP_ORIGIN,
      expectedRPID: RP_ID,
    })
  } catch (error) {
    console.error('[passkey] Registration verification error:', error)
    return c.json({ error: 'Verification failed' }, 400)
  }

  if (!verification.verified || !verification.registrationInfo) {
    return c.json({ error: 'Verification failed' }, 400)
  }

  const { credential } = verification.registrationInfo

  // Store the new passkey
  const newPasskey: NewPasskey = {
    id: randomUUID(),
    userId: session.user.id,
    credentialId: credential.id,  // Already base64url encoded
    publicKey: Buffer.from(credential.publicKey),
    counter: credential.counter,
    createdAt: new Date(),
    name: body.name || null,
  }

  await db.insert(passkeys).values(newPasskey)

  // Clean up challenge
  regChallenges.delete(session.sessionId)

  return c.json({
    id: newPasskey.id,
    name: newPasskey.name,
  })
})

/**
 * DELETE /auth/passkeys/:id
 * Delete a passkey (requires auth)
 */
passkeyRoutes.delete('/:id', requireSession, async (c) => {
  const session = c.get('session') as SessionContext
  const passkeyId = c.req.param('id')

  // Find the passkey
  const passkey = await db.select().from(passkeys).where(eq(passkeys.id, passkeyId)).get()

  if (!passkey) {
    return c.json({ error: 'Passkey not found' }, 404)
  }

  // Ensure user owns this passkey
  if (passkey.userId !== session.user.id) {
    return c.json({ error: 'Forbidden' }, 403)
  }

  // Check if this is the last passkey
  const userPasskeyCount = await db
    .select()
    .from(passkeys)
    .where(eq(passkeys.userId, session.user.id))

  if (userPasskeyCount.length <= 1) {
    return c.json({ error: 'Cannot delete last passkey' }, 400)
  }

  await db.delete(passkeys).where(eq(passkeys.id, passkeyId))

  return c.json({ success: true })
})
