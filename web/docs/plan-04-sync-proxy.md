# Plan 04: WebSocket Sync Proxy

## Scope
Route Automerge sync traffic through todu-fit-web, which proxies to the internal Automerge sync server and enforces session-based access.

## Requirements
- Browser connects to todu-fit-web via WS (e.g., `/sync`).
- The server authorizes the WS connection using the session cookie.
- The server proxies messages to the internal sync server (official `automerge-repo-sync-server`).
- Maintain a constant WS connection and support reconnect.

## Functional Details
- Add a WS endpoint to the Hono server.
- Reject WS connections without a valid session.
- Proxy raw WS frames bidirectionally.
- Support configuration of the internal sync server URL via environment variables.

## Non-Functional Requirements
- Low latency proxying; minimal buffering.
- Stable reconnect behavior when either side drops.

## Acceptance Criteria
- Authenticated users can sync via the proxy WS route.
- Unauthenticated users cannot open a sync connection.
- The app holds a persistent WS connection as it does today.
