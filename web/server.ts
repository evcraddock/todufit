import { Hono } from 'hono'
import { serve } from '@hono/node-server'
import { serveStatic } from '@hono/node-server/serve-static'
import { readFile } from 'fs/promises'
import { readFileSync } from 'fs'
import { join } from 'path'
import { authRoutes } from './server/routes/auth'
import { passkeyRoutes } from './server/routes/passkey'
import { calendarRoutes } from './server/routes/calendar'

// Read version from package.json at startup
const packageJson = JSON.parse(readFileSync(join(process.cwd(), 'package.json'), 'utf-8'))
const VERSION = packageJson.version
const START_TIME = Date.now()

const app = new Hono()

// Health check endpoint (no auth required)
app.get('/health', (c) => {
  return c.json({
    status: 'ok',
    version: VERSION,
    uptime: Math.floor((Date.now() - START_TIME) / 1000),
    timestamp: new Date().toISOString(),
  })
})

// Mount auth routes
app.route('/auth', authRoutes)
app.route('/auth/passkey', passkeyRoutes)

// Mount calendar routes
app.route('/api/calendar', calendarRoutes)

// Serve static files from dist/
app.use('/*', serveStatic({ root: './dist' }))

// SPA fallback - serve index.html for any unmatched route
app.get('*', async (c) => {
  const indexPath = join(process.cwd(), 'dist', 'index.html')
  const html = await readFile(indexPath, 'utf-8')
  return c.html(html)
})

const port = Number(process.env.PORT) || 3000

console.log(`[hono] TODU_ENV=${process.env.TODU_ENV || 'unknown'}`)
console.log(`[hono] Server running on http://localhost:${port}`)
console.log(`[hono] Auth endpoints: /auth/*`)
console.log(`[hono] Passkey endpoints: /auth/passkey/*`)
console.log(`[hono] Calendar endpoints: /api/calendar/*`)

if (process.env.NODE_ENV !== 'production') {
  const adminEmail = process.env.ADMIN_EMAIL || 'you@example.com'
  console.log('')
  const publicUrl = process.env.PUBLIC_URL || 'http://localhost:5173'
  console.log('=== DEV LOGIN ===')
  console.log(`1. Go to ${publicUrl}/login`)
  console.log(`2. Enter: ${adminEmail}`)
  console.log('3. Check this log for the magic link')
  console.log('=================')
  console.log('')
}

serve({
  fetch: app.fetch,
  port,
})
