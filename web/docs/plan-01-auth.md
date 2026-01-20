# Plan 01: Auth + Sessions + SQLite

## Scope
Establish first-party auth for todu-fit-web, backed by SQLite and session cookies. Keep magic link + passkey flows, and remove API key usage.

## Requirements
- Use Hono server to host auth endpoints.
- Authenticate via magic link and passkeys (same UX as today).
- Restrict login to an allowlist of email addresses managed in-app.
- Bootstrap allowlist with a single admin email from env vars; that user can add other allowed emails.
- Use HTTP-only session cookies; no API keys.
- Persist users, sessions, and auth artifacts in SQLite.
- Support logout and session invalidation.
- Ensure auth routes are usable by the existing frontend components with minimal changes.

## Functional Details
- Endpoints (minimum):
  - `POST /auth/login` to start magic link flow (allowed emails only).
  - `GET /auth/callback` or `POST /auth/verify` to complete magic link and set session.
  - `POST /auth/passkey/start` and `POST /auth/passkey/finish` for passkey login.
  - `POST /auth/passkey/register/start` and `POST /auth/passkey/register/finish` for passkey registration.
  - `POST /auth/logout` to end session.
  - `GET /auth/me` to return user profile and settings.
  - `GET /auth/allowlist` and `POST /auth/allowlist` to view/add allowed emails (admin only).
- Login should return a session cookie; no API key is returned to the client.
- `GET /auth/me` returns user id (email), plus settings (root_doc_id, current_group_id).
- The admin email from env vars is always allowed; once logged in, they can add other allowed emails.

## Email Delivery Requirements
- Send magic link emails via SMTP using environment-configured URL/credentials (no hard-coded secrets).
- Configure a From name/address, SMTP settings, and a public base URL for link generation (all via env vars).
- Magic links are single-use and time-limited; email copy should mention expiration.
- Avoid user enumeration: respond the same whether the email exists or is allowed.
- Provide a dev/test mode that logs email content instead of sending.

## Data Model (SQLite)
- `users`: `id`, `email`, `created_at`, `last_login_at`.
- `sessions`: `id`, `user_id`, `expires_at`, `created_at`, `revoked_at`.
- `magic_links`: `id`, `user_id`, `token`, `expires_at`, `used_at`.
- `allowed_emails`: `email`, `added_by_user_id`, `created_at`, `revoked_at`.
- `passkeys`: `id`, `user_id`, `credential_id`, `public_key`, `created_at`, `last_used_at`, `name`.
- `user_settings`: `user_id`, `root_doc_id`, `current_group_id`.

## Security Requirements
- Session cookies are HTTP-only and same-site (Lax or Strict, to be decided).
- Magic links and sessions expire.
- Rate-limit or throttle login attempts.

## UX Requirements
- Preserve the existing magic link and passkey UI paths.
- Errors are displayed clearly on login/register failures.

## Acceptance Criteria
- Users can log in with magic link or passkey if their email is allowed; a session cookie is set.
- Authenticated calls to `GET /auth/me` succeed; unauthenticated calls return 401.
- Logout clears the session and client becomes unauthenticated.
- No API key is stored or used anywhere in the app.
