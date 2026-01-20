import { Hono } from 'hono'
import { eq, and, gt, isNull } from 'drizzle-orm'
import { randomBytes, randomUUID } from 'crypto'
import {
  db,
  users,
  sessions,
  magicLinks,
  allowedEmails,
  userSettings,
  passkeys,
  groupInvites,
  type NewUser,
  type NewMagicLink,
  type NewAllowedEmail,
  type NewUserSettings,
  type NewGroupInvite,
} from '../db'
import { requireSession, optionalSession, type SessionContext } from '../middleware/session'
import { createSession, revokeSession } from '../lib/session'
import { sendMagicLinkEmail, sendGroupInviteEmail } from '../lib/email'

// Magic link expiry in minutes (default 15)
const MAGIC_LINK_EXPIRY_MINUTES = Number(process.env.MAGIC_LINK_EXPIRY_MINUTES) || 15

// Admin email from environment (always allowed)
const ADMIN_EMAIL = process.env.ADMIN_EMAIL

// Create auth router
export const authRoutes = new Hono()

/**
 * POST /auth/login
 * Start magic link authentication flow
 */
authRoutes.post('/login', async (c) => {
  const body = await c.req.json<{ email: string }>()
  const email = body.email?.toLowerCase().trim()

  if (!email) {
    return c.json({ error: 'Email is required' }, 400)
  }

  // Check if email is allowed (admin email is always allowed)
  const isAdmin = email === ADMIN_EMAIL?.toLowerCase()

  if (!isAdmin) {
    const allowed = await db
      .select()
      .from(allowedEmails)
      .where(and(eq(allowedEmails.email, email), isNull(allowedEmails.revokedAt)))
      .get()

    // Don't reveal whether email is allowed - always return success
    if (!allowed) {
      console.log(`[auth] Login attempt for non-allowed email: ${email}`)
      // Return success to prevent email enumeration
      return c.json({ success: true })
    }
  }

  // Find or create user
  let user = await db.select().from(users).where(eq(users.email, email)).get()

  if (!user) {
    const newUser: NewUser = {
      id: randomUUID(),
      email,
      createdAt: new Date(),
    }
    await db.insert(users).values(newUser)
    user = await db.select().from(users).where(eq(users.email, email)).get()

    // Create initial settings for new user
    const newSettings: NewUserSettings = {
      userId: user!.id,
    }
    await db.insert(userSettings).values(newSettings)

    // If this is the admin email and it's not in allowed_emails, add it
    if (isAdmin) {
      const adminAllowed = await db
        .select()
        .from(allowedEmails)
        .where(eq(allowedEmails.email, email))
        .get()

      if (!adminAllowed) {
        const newAllowed: NewAllowedEmail = {
          email,
          createdAt: new Date(),
        }
        await db.insert(allowedEmails).values(newAllowed)
      }
    }
  }

  // Generate magic link token
  const token = randomBytes(32).toString('hex')
  const expiresAt = new Date(Date.now() + MAGIC_LINK_EXPIRY_MINUTES * 60 * 1000)

  const newMagicLink: NewMagicLink = {
    id: randomUUID(),
    userId: user!.id,
    token,
    expiresAt,
  }
  await db.insert(magicLinks).values(newMagicLink)

  // Send magic link email
  await sendMagicLinkEmail({
    to: email,
    token,
    expiresInMinutes: MAGIC_LINK_EXPIRY_MINUTES,
  })

  return c.json({ success: true })
})

/**
 * GET /auth/verify
 * Complete magic link authentication
 */
authRoutes.get('/verify', async (c) => {
  const token = c.req.query('token')

  if (!token) {
    return c.json({ error: 'Token is required' }, 400)
  }

  // Find valid magic link
  const magicLink = await db
    .select()
    .from(magicLinks)
    .where(
      and(
        eq(magicLinks.token, token),
        gt(magicLinks.expiresAt, new Date()),
        isNull(magicLinks.usedAt)
      )
    )
    .get()

  if (!magicLink) {
    return c.json({ error: 'Invalid or expired token' }, 400)
  }

  // Mark magic link as used
  await db
    .update(magicLinks)
    .set({ usedAt: new Date() })
    .where(eq(magicLinks.id, magicLink.id))

  // Update user last login
  await db
    .update(users)
    .set({ lastLoginAt: new Date() })
    .where(eq(users.id, magicLink.userId))

  // Create session
  await createSession(c, magicLink.userId)

  // Redirect to app
  const publicUrl = process.env.PUBLIC_URL || 'http://localhost:5173'
  return c.redirect(`${publicUrl}/`)
})

/**
 * POST /auth/logout
 * End the current session
 */
authRoutes.post('/logout', requireSession, async (c) => {
  const session = c.get('session') as SessionContext
  await revokeSession(c, session.sessionId)
  return c.json({ success: true })
})

/**
 * GET /auth/me
 * Get current user profile and settings
 */
authRoutes.get('/me', requireSession, async (c) => {
  const session = c.get('session') as SessionContext

  // Get user's passkeys
  const userPasskeys = await db
    .select({
      id: passkeys.id,
      name: passkeys.name,
      createdAt: passkeys.createdAt,
      lastUsedAt: passkeys.lastUsedAt,
    })
    .from(passkeys)
    .where(eq(passkeys.userId, session.user.id))

  const isAdmin = session.user.email === ADMIN_EMAIL?.toLowerCase()

  return c.json({
    user_id: session.user.id,
    email: session.user.email,
    root_doc_id: session.user.rootDocId,
    current_group_id: session.user.currentGroupId,
    is_admin: isAdmin,
    passkeys: userPasskeys.map((p) => ({
      id: p.id,
      name: p.name,
      created_at: p.createdAt?.toISOString(),
      last_used_at: p.lastUsedAt?.toISOString(),
    })),
  })
})

/**
 * GET /auth/allowlist
 * List allowed emails (admin only)
 */
authRoutes.get('/allowlist', requireSession, async (c) => {
  const session = c.get('session') as SessionContext

  // Check if user is admin
  if (session.user.email !== ADMIN_EMAIL?.toLowerCase()) {
    return c.json({ error: 'Forbidden' }, 403)
  }

  const allowed = await db
    .select({
      email: allowedEmails.email,
      createdAt: allowedEmails.createdAt,
      revokedAt: allowedEmails.revokedAt,
    })
    .from(allowedEmails)
    .where(isNull(allowedEmails.revokedAt))

  return c.json({
    emails: allowed.map((a) => ({
      email: a.email,
      created_at: a.createdAt?.toISOString(),
    })),
  })
})

/**
 * POST /auth/allowlist
 * Add an email to the allowlist (admin only)
 */
authRoutes.post('/allowlist', requireSession, async (c) => {
  const session = c.get('session') as SessionContext

  // Check if user is admin
  if (session.user.email !== ADMIN_EMAIL?.toLowerCase()) {
    return c.json({ error: 'Forbidden' }, 403)
  }

  const body = await c.req.json<{ email: string }>()
  const email = body.email?.toLowerCase().trim()

  if (!email) {
    return c.json({ error: 'Email is required' }, 400)
  }

  // Check if already allowed
  const existing = await db
    .select()
    .from(allowedEmails)
    .where(eq(allowedEmails.email, email))
    .get()

  if (existing && !existing.revokedAt) {
    return c.json({ error: 'Email already allowed' }, 400)
  }

  if (existing) {
    // Re-enable previously revoked email
    await db
      .update(allowedEmails)
      .set({ revokedAt: null, addedByUserId: session.user.id })
      .where(eq(allowedEmails.email, email))
  } else {
    // Add new email
    const newAllowed: NewAllowedEmail = {
      email,
      addedByUserId: session.user.id,
      createdAt: new Date(),
    }
    await db.insert(allowedEmails).values(newAllowed)
  }

  return c.json({ success: true })
})

/**
 * DELETE /auth/allowlist/:email
 * Remove an email from the allowlist (admin only)
 */
authRoutes.delete('/allowlist/:email', requireSession, async (c) => {
  const session = c.get('session') as SessionContext

  // Check if user is admin
  if (session.user.email !== ADMIN_EMAIL?.toLowerCase()) {
    return c.json({ error: 'Forbidden' }, 403)
  }

  const email = c.req.param('email')?.toLowerCase().trim()

  if (!email) {
    return c.json({ error: 'Email is required' }, 400)
  }

  // Can't remove admin email
  if (email === ADMIN_EMAIL?.toLowerCase()) {
    return c.json({ error: 'Cannot remove admin email' }, 400)
  }

  await db
    .update(allowedEmails)
    .set({ revokedAt: new Date() })
    .where(eq(allowedEmails.email, email))

  return c.json({ success: true })
})

// ============================================================================
// User Settings
// ============================================================================

/**
 * Validate root doc ID format (bs58check-encoded, should be ~25-35 chars)
 */
function isValidRootDocId(id: string): boolean {
  // Basic validation: bs58check encoded string
  // Should be alphanumeric (base58 chars: 1-9, A-H, J-N, P-Z, a-k, m-z)
  const bs58Regex = /^[1-9A-HJ-NP-Za-km-z]{20,50}$/
  return bs58Regex.test(id)
}

/**
 * PUT /auth/identity
 * Update the user's root doc ID after creating identity documents
 * Used by RepoContext when creating new identity from web app
 */
authRoutes.put('/identity', requireSession, async (c) => {
  const session = c.get('session') as SessionContext
  const body = await c.req.json<{ root_doc_id: string }>()
  const rootDocId = body.root_doc_id?.trim()

  if (!rootDocId) {
    return c.json({ error: 'root_doc_id is required' }, 400)
  }

  if (!isValidRootDocId(rootDocId)) {
    return c.json({ error: 'Invalid root doc ID format' }, 400)
  }

  // Update or create settings
  const existingSettings = await db
    .select()
    .from(userSettings)
    .where(eq(userSettings.userId, session.user.id))
    .get()

  if (existingSettings) {
    await db
      .update(userSettings)
      .set({ rootDocId })
      .where(eq(userSettings.userId, session.user.id))
  } else {
    await db.insert(userSettings).values({
      userId: session.user.id,
      rootDocId,
    })
  }

  console.log(`[auth] Updated root_doc_id for user ${session.user.id}: ${rootDocId}`)

  return c.json({
    success: true,
    root_doc_id: rootDocId,
  })
})

/**
 * PUT /auth/settings/root-doc-id
 * Set or update the user's root doc ID
 */
authRoutes.put('/settings/root-doc-id', requireSession, async (c) => {
  const session = c.get('session') as SessionContext
  const body = await c.req.json<{ root_doc_id: string; confirm?: boolean }>()
  const rootDocId = body.root_doc_id?.trim()

  if (!rootDocId) {
    return c.json({ error: 'root_doc_id is required' }, 400)
  }

  if (!isValidRootDocId(rootDocId)) {
    return c.json({ error: 'Invalid root doc ID format' }, 400)
  }

  // Check if user already has a root doc ID set
  const existingSettings = await db
    .select()
    .from(userSettings)
    .where(eq(userSettings.userId, session.user.id))
    .get()

  // If changing from an existing root doc ID, require confirmation
  if (existingSettings?.rootDocId && existingSettings.rootDocId !== rootDocId) {
    if (!body.confirm) {
      return c.json({
        error: 'Changing root doc ID requires confirmation',
        requires_confirmation: true,
        current_root_doc_id: existingSettings.rootDocId,
      }, 400)
    }
  }

  // Update or create settings
  if (existingSettings) {
    await db
      .update(userSettings)
      .set({ rootDocId })
      .where(eq(userSettings.userId, session.user.id))
  } else {
    await db.insert(userSettings).values({
      userId: session.user.id,
      rootDocId,
    })
  }

  return c.json({
    success: true,
    root_doc_id: rootDocId,
  })
})

/**
 * PUT /auth/settings/current-group
 * Set the user's current group ID
 */
authRoutes.put('/settings/current-group', requireSession, async (c) => {
  const session = c.get('session') as SessionContext
  const body = await c.req.json<{ current_group_id: string }>()
  const currentGroupId = body.current_group_id?.trim()

  if (!currentGroupId) {
    return c.json({ error: 'current_group_id is required' }, 400)
  }

  // Update or create settings
  const existingSettings = await db
    .select()
    .from(userSettings)
    .where(eq(userSettings.userId, session.user.id))
    .get()

  if (existingSettings) {
    await db
      .update(userSettings)
      .set({ currentGroupId })
      .where(eq(userSettings.userId, session.user.id))
  } else {
    await db.insert(userSettings).values({
      userId: session.user.id,
      currentGroupId,
    })
  }

  return c.json({
    success: true,
    current_group_id: currentGroupId,
  })
})

// ============================================================================
// Group Invites
// ============================================================================

// Group invite expiry in hours (default 24)
const GROUP_INVITE_EXPIRY_HOURS = Number(process.env.GROUP_INVITE_EXPIRY_HOURS) || 24

/**
 * POST /auth/groups/invite
 * Send a group invitation email
 */
authRoutes.post('/groups/invite', requireSession, async (c) => {
  const session = c.get('session') as SessionContext
  const body = await c.req.json<{
    email: string
    group_doc_id: string
    group_name: string
  }>()

  const email = body.email?.toLowerCase().trim()
  const groupDocId = body.group_doc_id?.trim()
  const groupName = body.group_name?.trim()

  if (!email) {
    return c.json({ error: 'Email is required' }, 400)
  }

  if (!groupDocId) {
    return c.json({ error: 'group_doc_id is required' }, 400)
  }

  if (!groupName) {
    return c.json({ error: 'group_name is required' }, 400)
  }

  // Validate email format
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
  if (!emailRegex.test(email)) {
    return c.json({ error: 'Invalid email format' }, 400)
  }

  // Don't allow inviting yourself
  if (email === session.user.email.toLowerCase()) {
    return c.json({ error: 'You cannot invite yourself' }, 400)
  }

  // Check if invitee is on the allowlist (admin email is always allowed)
  const isInviteeAdmin = email === ADMIN_EMAIL?.toLowerCase()
  if (!isInviteeAdmin) {
    const allowed = await db
      .select()
      .from(allowedEmails)
      .where(and(eq(allowedEmails.email, email), isNull(allowedEmails.revokedAt)))
      .get()

    if (!allowed) {
      return c.json({ error: 'This email is not on the allowlist. Please add them to the allowlist first.' }, 400)
    }
  }

  // Generate secure token
  const token = randomBytes(32).toString('hex')
  const expiresAt = new Date(Date.now() + GROUP_INVITE_EXPIRY_HOURS * 60 * 60 * 1000)

  // Create invite record
  const newInvite: NewGroupInvite = {
    id: randomUUID(),
    token,
    groupDocId,
    groupName,
    inviterUserId: session.user.id,
    inviteeEmail: email,
    expiresAt,
    createdAt: new Date(),
  }

  await db.insert(groupInvites).values(newInvite)

  // Send invitation email
  await sendGroupInviteEmail({
    to: email,
    token,
    groupName,
    inviterEmail: session.user.email,
    expiresInHours: GROUP_INVITE_EXPIRY_HOURS,
  })

  console.log(`[auth] Group invite sent from ${session.user.email} to ${email} for group "${groupName}"`)

  return c.json({ success: true })
})

/**
 * GET /auth/invite/accept
 * Accept a group invitation via magic link
 * Works like a magic link - automatically logs in the invitee
 */
authRoutes.get('/invite/accept', async (c) => {
  const token = c.req.query('token')
  const publicUrl = process.env.PUBLIC_URL || 'http://localhost:5173'

  if (!token) {
    return c.redirect(`${publicUrl}/invite/error?reason=missing_token`)
  }

  // Find valid invite
  const invite = await db
    .select()
    .from(groupInvites)
    .where(
      and(
        eq(groupInvites.token, token),
        gt(groupInvites.expiresAt, new Date()),
        isNull(groupInvites.acceptedAt)
      )
    )
    .get()

  if (!invite) {
    // Check if it was already used or just expired/invalid
    const anyInvite = await db
      .select()
      .from(groupInvites)
      .where(eq(groupInvites.token, token))
      .get()

    if (anyInvite?.acceptedAt) {
      return c.redirect(`${publicUrl}/invite/error?reason=already_used`)
    } else if (anyInvite && anyInvite.expiresAt < new Date()) {
      return c.redirect(`${publicUrl}/invite/error?reason=expired`)
    }
    return c.redirect(`${publicUrl}/invite/error?reason=invalid`)
  }

  // Find or create user for the invitee email
  let user = await db.select().from(users).where(eq(users.email, invite.inviteeEmail)).get()

  if (!user) {
    // Create new user
    const newUser: NewUser = {
      id: randomUUID(),
      email: invite.inviteeEmail,
      createdAt: new Date(),
    }
    await db.insert(users).values(newUser)
    user = await db.select().from(users).where(eq(users.email, invite.inviteeEmail)).get()

    // Create initial settings for new user
    const newSettings: NewUserSettings = {
      userId: user!.id,
    }
    await db.insert(userSettings).values(newSettings)
  }

  // Update user last login
  await db
    .update(users)
    .set({ lastLoginAt: new Date() })
    .where(eq(users.id, user!.id))

  // Create session for the user (like magic link does)
  await createSession(c, user!.id)

  // Mark invite as accepted
  await db
    .update(groupInvites)
    .set({
      acceptedAt: new Date(),
      acceptedByUserId: user!.id,
    })
    .where(eq(groupInvites.id, invite.id))

  console.log(`[auth] Group invite accepted by ${invite.inviteeEmail} for group "${invite.groupName}"`)

  // Redirect to frontend with success params
  // The frontend will handle adding the group to the identity document
  const params = new URLSearchParams({
    group_doc_id: invite.groupDocId,
    group_name: invite.groupName,
  })
  return c.redirect(`${publicUrl}/invite/success?${params.toString()}`)
})
