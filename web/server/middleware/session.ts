import { createMiddleware } from 'hono/factory'
import { getCookie } from 'hono/cookie'
import { eq, and, gt, isNull } from 'drizzle-orm'
import { db, sessions, users, userSettings } from '../db'

// Session cookie name
export const SESSION_COOKIE_NAME = 'session_id'

// Types for session context
export interface SessionUser {
  id: string
  email: string
  rootDocId: string | null
  currentGroupId: string | null
}

export interface SessionContext {
  user: SessionUser
  sessionId: string
}

// Extend Hono's context variables
declare module 'hono' {
  interface ContextVariableMap {
    session: SessionContext
  }
}

/**
 * Middleware that validates session cookie and attaches user to context.
 * Returns 401 if session is invalid or missing.
 */
export const requireSession = createMiddleware(async (c, next) => {
  const sessionId = getCookie(c, SESSION_COOKIE_NAME)

  if (!sessionId) {
    return c.json({ error: 'Unauthorized' }, 401)
  }

  // Find valid session (not expired, not revoked)
  const session = await db
    .select()
    .from(sessions)
    .where(
      and(
        eq(sessions.id, sessionId),
        gt(sessions.expiresAt, new Date()),
        isNull(sessions.revokedAt)
      )
    )
    .get()

  if (!session) {
    return c.json({ error: 'Session expired' }, 401)
  }

  // Get user and settings
  const user = await db
    .select()
    .from(users)
    .where(eq(users.id, session.userId))
    .get()

  if (!user) {
    return c.json({ error: 'User not found' }, 401)
  }

  const settings = await db
    .select()
    .from(userSettings)
    .where(eq(userSettings.userId, user.id))
    .get()

  // Attach session context
  c.set('session', {
    user: {
      id: user.id,
      email: user.email,
      rootDocId: settings?.rootDocId ?? null,
      currentGroupId: settings?.currentGroupId ?? null,
    },
    sessionId,
  })

  await next()
})

/**
 * Optional session middleware - doesn't require auth but attaches user if present.
 * Useful for routes that work for both authenticated and unauthenticated users.
 */
export const optionalSession = createMiddleware(async (c, next) => {
  const sessionId = getCookie(c, SESSION_COOKIE_NAME)

  if (sessionId) {
    const session = await db
      .select()
      .from(sessions)
      .where(
        and(
          eq(sessions.id, sessionId),
          gt(sessions.expiresAt, new Date()),
          isNull(sessions.revokedAt)
        )
      )
      .get()

    if (session) {
      const user = await db
        .select()
        .from(users)
        .where(eq(users.id, session.userId))
        .get()

      if (user) {
        const settings = await db
          .select()
          .from(userSettings)
          .where(eq(userSettings.userId, user.id))
          .get()

        c.set('session', {
          user: {
            id: user.id,
            email: user.email,
            rootDocId: settings?.rootDocId ?? null,
            currentGroupId: settings?.currentGroupId ?? null,
          },
          sessionId,
        })
      }
    }
  }

  await next()
})
