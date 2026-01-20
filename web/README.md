# Todu Fit Web

A meal planning and nutrition tracking web application.

## Features

- **Dishes** - Create and manage recipes with ingredients and nutritional info
- **Meal Plans** - Plan meals by date and meal type (breakfast, lunch, dinner, snack)
- **Food Log** - Track what you actually ate
- **Shopping Lists** - Generate shopping lists from meal plans

## How It Works

The app uses a local-first architecture:

- Data is stored locally in your browser (IndexedDB)
- Changes sync automatically via WebSocket when online
- Works offline - edits save locally and sync when reconnected
- Multiple devices stay in sync through a shared identity

Authentication uses magic links (passwordless email) or passkeys.

## Tech Stack

- React 19 + TypeScript
- Automerge (CRDT-based sync)
- Hono (auth server)
- TailwindCSS

See [AGENTS.md](../AGENTS.md) for development setup.
