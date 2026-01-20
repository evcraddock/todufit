import Database from 'better-sqlite3'
import { drizzle } from 'drizzle-orm/better-sqlite3'
import { migrate } from 'drizzle-orm/better-sqlite3/migrator'
import * as schema from './schema'
import { existsSync, mkdirSync } from 'fs'
import { dirname, join } from 'path'

// Get database path from environment or use default
const DATABASE_PATH = process.env.DATABASE_PATH || './data/dev/todu-fit.sqlite'

// Ensure the directory exists
const dbDir = dirname(DATABASE_PATH)
if (!existsSync(dbDir)) {
  mkdirSync(dbDir, { recursive: true })
}

// Create SQLite connection
const sqlite = new Database(DATABASE_PATH)

// Enable WAL mode for better concurrent access
sqlite.pragma('journal_mode = WAL')

// Create Drizzle instance with schema
export const db = drizzle(sqlite, { schema })

// Run migrations on startup
const migrationsFolder = join(process.cwd(), 'server', 'db', 'migrations')
if (existsSync(migrationsFolder)) {
  console.log('[db] Running migrations...')
  try {
    migrate(db, { migrationsFolder })
    console.log('[db] Migrations complete')
  } catch (error) {
    console.error('[db] Migration failed:', error)
    throw error
  }
} else {
  console.warn('[db] Migrations folder not found:', migrationsFolder)
}

// Export the raw sqlite connection for migrations
export { sqlite }

// Export schema for convenience
export * from './schema'
