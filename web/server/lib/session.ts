import { Context } from 'hono'
import { setCookie, deleteCookie } from 'hono/cookie'
import { eq } from 'drizzle-orm'
import { db, sessions, type NewSession } from '../db'
import { SESSION_COOKIE_NAME } from '../middleware/session'
import { randomBytes } from 'crypto'

// Session duration: 30 days
const SESSION_MAX_AGE_MS = 30 * 24 * 60 * 60 * 1000
const SESSION_MAX_AGE_SECONDS = 30 * 24 * 60 * 60

/**
 * Generate a secure random session ID
 */
export function generateSessionId(): string {
  return randomBytes(32).toString('hex')
}

/**
 * Create a new session for a user and set the session cookie
 */
export async function createSession(c: Context, userId: string): Promise<string> {
  const sessionId = generateSessionId()
  const now = new Date()
  const expiresAt = new Date(now.getTime() + SESSION_MAX_AGE_MS)

  const newSession: NewSession = {
    id: sessionId,
    userId,
    expiresAt,
    createdAt: now,
  }

  await db.insert(sessions).values(newSession)

  // Set HTTP-only session cookie
  setCookie(c, SESSION_COOKIE_NAME, sessionId, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    sameSite: 'Lax',
    path: '/',
    maxAge: SESSION_MAX_AGE_SECONDS,
  })

  return sessionId
}

/**
 * Revoke a session and clear the cookie
 */
export async function revokeSession(c: Context, sessionId: string): Promise<void> {
  // Mark session as revoked
  await db
    .update(sessions)
    .set({ revokedAt: new Date() })
    .where(eq(sessions.id, sessionId))

  // Clear the cookie
  deleteCookie(c, SESSION_COOKIE_NAME, {
    path: '/',
  })
}

/**
 * Revoke all sessions for a user
 */
export async function revokeAllUserSessions(userId: string): Promise<void> {
  await db
    .update(sessions)
    .set({ revokedAt: new Date() })
    .where(eq(sessions.userId, userId))
}
