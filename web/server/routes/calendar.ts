/**
 * Calendar subscription routes.
 *
 * Provides iCalendar feed for meal plans via token-based authentication.
 */

import { Hono } from 'hono'
import { eq, and, isNull } from 'drizzle-orm'
import { randomBytes, randomUUID } from 'crypto'
import {
  db,
  groupCalendarTokens,
  type NewGroupCalendarToken,
} from '../db'
import { requireSession, type SessionContext } from '../middleware/session'
import { loadGroupMealPlans, loadGroupDishes } from '../lib/automerge'
import { generateICalendar } from '../lib/icalendar'

// Token length in bytes (32 bytes = 64 hex chars)
const TOKEN_LENGTH = 32

// Create calendar router
export const calendarRoutes = new Hono()

/**
 * GET /api/calendar/t/:token.ics
 * Public endpoint - returns iCalendar feed for a group's meal plans.
 * No session required - token is the credential.
 */
calendarRoutes.get('/t/:token', async (c) => {
  const tokenParam = c.req.param('token')

  // Strip .ics extension if present
  const token = tokenParam.replace(/\.ics$/, '')

  if (!token) {
    return c.text('Token required', 400)
  }

  // Find the active token
  const calendarToken = await db
    .select()
    .from(groupCalendarTokens)
    .where(
      and(
        eq(groupCalendarTokens.token, token),
        isNull(groupCalendarTokens.revokedAt)
      )
    )
    .get()

  if (!calendarToken) {
    return c.text('Invalid or revoked token', 404)
  }

  const groupDocId = calendarToken.groupDocId

  // Load meal plans from Automerge
  const mealPlans = await loadGroupMealPlans(groupDocId)
  if (mealPlans === null) {
    console.error(`[calendar] Failed to load meal plans for group: ${groupDocId}`)
    return c.text('Failed to load meal plans', 500)
  }

  // Load dishes for richer event descriptions
  const dishes = await loadGroupDishes(groupDocId)

  // Generate iCalendar
  const appUrl = process.env.PUBLIC_URL || 'http://localhost:5173'
  const icalendar = generateICalendar(mealPlans, {
    calendarName: 'Todu Fit',
    groupName: 'Todu Fit',
    appUrl,
    dishes: dishes || undefined,
  })

  // Set appropriate headers
  c.header('Content-Type', 'text/calendar; charset=utf-8')
  c.header('Content-Disposition', 'attachment; filename="mealplans.ics"')
  // Cache for 5 minutes, allow revalidation
  c.header('Cache-Control', 'public, max-age=300, must-revalidate')

  return c.text(icalendar)
})

/**
 * GET /api/calendar/token
 * Get the current calendar subscription token for a group.
 * Requires authentication.
 */
calendarRoutes.get('/token', requireSession, async (c) => {
  const session = c.get('session') as SessionContext
  const groupDocId = c.req.query('group_doc_id')

  if (!groupDocId) {
    return c.json({ error: 'group_doc_id is required' }, 400)
  }

  // Find active token for this group
  const existingToken = await db
    .select()
    .from(groupCalendarTokens)
    .where(
      and(
        eq(groupCalendarTokens.groupDocId, groupDocId),
        isNull(groupCalendarTokens.revokedAt)
      )
    )
    .get()

  if (!existingToken) {
    return c.json({ token: null })
  }

  // Build the subscription URL
  const publicUrl = process.env.PUBLIC_URL || 'http://localhost:5173'
  // Replace http with webcal for calendar subscriptions
  const webcalUrl = publicUrl.replace(/^https?:/, 'webcal:')
  const subscriptionUrl = `${webcalUrl}/api/calendar/t/${existingToken.token}.ics`
  const httpUrl = `${publicUrl}/api/calendar/t/${existingToken.token}.ics`

  return c.json({
    token: existingToken.token,
    subscription_url: subscriptionUrl,
    http_url: httpUrl,
    created_at: existingToken.createdAt?.toISOString(),
  })
})

/**
 * POST /api/calendar/token
 * Create or regenerate a calendar subscription token for a group.
 * Requires authentication. Regenerating revokes the previous token.
 */
calendarRoutes.post('/token', requireSession, async (c) => {
  const session = c.get('session') as SessionContext
  const body = await c.req.json<{ group_doc_id: string; regenerate?: boolean }>()
  const groupDocId = body.group_doc_id?.trim()

  if (!groupDocId) {
    return c.json({ error: 'group_doc_id is required' }, 400)
  }

  // Check for existing active token
  const existingToken = await db
    .select()
    .from(groupCalendarTokens)
    .where(
      and(
        eq(groupCalendarTokens.groupDocId, groupDocId),
        isNull(groupCalendarTokens.revokedAt)
      )
    )
    .get()

  if (existingToken && !body.regenerate) {
    return c.json({
      error: 'Token already exists. Set regenerate: true to create a new one.',
      has_existing_token: true,
    }, 400)
  }

  // Revoke existing token if regenerating
  if (existingToken) {
    await db
      .update(groupCalendarTokens)
      .set({ revokedAt: new Date() })
      .where(eq(groupCalendarTokens.id, existingToken.id))

    console.log(`[calendar] Revoked token for group ${groupDocId}`)
  }

  // Generate new token
  const token = randomBytes(TOKEN_LENGTH).toString('hex')
  const newToken: NewGroupCalendarToken = {
    id: randomUUID(),
    groupDocId,
    token,
    createdByUserId: session.user.id,
    createdAt: new Date(),
  }

  await db.insert(groupCalendarTokens).values(newToken)

  console.log(`[calendar] Created token for group ${groupDocId} by user ${session.user.id}`)

  // Build the subscription URL
  const publicUrl = process.env.PUBLIC_URL || 'http://localhost:5173'
  const webcalUrl = publicUrl.replace(/^https?:/, 'webcal:')
  const subscriptionUrl = `${webcalUrl}/api/calendar/t/${token}.ics`
  const httpUrl = `${publicUrl}/api/calendar/t/${token}.ics`

  return c.json({
    token,
    subscription_url: subscriptionUrl,
    http_url: httpUrl,
    created_at: newToken.createdAt?.toISOString(),
    regenerated: !!existingToken,
  })
})

/**
 * DELETE /api/calendar/token
 * Revoke the calendar subscription token for a group.
 * Requires authentication.
 */
calendarRoutes.delete('/token', requireSession, async (c) => {
  const session = c.get('session') as SessionContext
  const groupDocId = c.req.query('group_doc_id')

  if (!groupDocId) {
    return c.json({ error: 'group_doc_id is required' }, 400)
  }

  // Find and revoke active token
  const existingToken = await db
    .select()
    .from(groupCalendarTokens)
    .where(
      and(
        eq(groupCalendarTokens.groupDocId, groupDocId),
        isNull(groupCalendarTokens.revokedAt)
      )
    )
    .get()

  if (!existingToken) {
    return c.json({ error: 'No active token found' }, 404)
  }

  await db
    .update(groupCalendarTokens)
    .set({ revokedAt: new Date() })
    .where(eq(groupCalendarTokens.id, existingToken.id))

  console.log(`[calendar] Revoked token for group ${groupDocId} by user ${session.user.id}`)

  return c.json({ success: true })
})
