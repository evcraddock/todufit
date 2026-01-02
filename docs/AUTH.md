# Authentication System

## Overview

Authentication is centralized in the sync server. All clients (CLI, Web, iOS) authenticate against the same endpoints and receive API keys for sync operations.

## Auth Methods

### Magic Link (All Clients)

Email-based passwordless authentication:

1. User provides email
2. Server sends email with magic link
3. User clicks link
4. Client receives API key

**Pros:** Works everywhere, no password to manage
**Cons:** Requires email round-trip, depends on email delivery

### Passkeys (Web + iOS)

WebAuthn/FIDO2 authentication:

1. User initiates login
2. Server sends challenge
3. Device authenticates (Face ID, Touch ID, Windows Hello, etc.)
4. Server verifies signature, returns API key

**Pros:** Instant, phishing-resistant, excellent UX
**Cons:** Requires modern browser/device, needs initial registration

## Recommended Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      Login Flow                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. User enters email                                       │
│                          │                                  │
│                          ▼                                  │
│  2. Check: Does user have passkey registered?               │
│                          │                                  │
│              ┌───────────┴───────────┐                      │
│              │                       │                      │
│              ▼                       ▼                      │
│         Has passkey             No passkey                  │
│              │                       │                      │
│              ▼                       ▼                      │
│    3a. Prompt for passkey    3b. Send magic link           │
│        authentication             email                     │
│              │                       │                      │
│              ▼                       ▼                      │
│    4a. Verify passkey        4b. User clicks link          │
│              │                       │                      │
│              └───────────┬───────────┘                      │
│                          │                                  │
│                          ▼                                  │
│  5. Server returns API key                                  │
│                          │                                  │
│                          ▼                                  │
│  6. Client stores API key (config/session/keychain)         │
│                                                             │
│  Post-login (magic link users):                             │
│  7. Prompt: "Add passkey for faster login next time?"       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## API Endpoints

### Magic Link

```
POST /auth/login
Request:
{
  "email": "user@example.com",
  "callback_url": "http://localhost:9999/callback"  // CLI
                  "https://app.todufit.com/auth/callback"  // Web
                  "todufit://auth"  // iOS (Universal Link)
}

Response:
{
  "message": "Magic link sent",
  "expires_in": 600
}
```

```
GET /auth/verify?token=xxx
(Redirect mode - for CLI callback)

Response: 302 Redirect to callback_url?key=xxx&user=yyy
```

```
POST /auth/verify
(API mode - for Web/iOS)

Request:
{
  "token": "magic-link-token"
}

Response:
{
  "api_key": "ak_xxxxxxxxxxxx",
  "user_id": "usr_xxxxxxxxxxxx",
  "email": "user@example.com"
}
```

### Passkey Registration

```
POST /auth/passkey/register/start
Headers: Authorization: Bearer <api_key>

Response:
{
  "challenge": "base64-encoded-challenge",
  "rp": {
    "name": "Todufit",
    "id": "todufit.example.com"
  },
  "user": {
    "id": "base64-user-id",
    "name": "user@example.com",
    "displayName": "user@example.com"
  },
  "pubKeyCredParams": [...],
  "timeout": 60000,
  "attestation": "none",
  "authenticatorSelection": {
    "authenticatorAttachment": "platform",
    "residentKey": "preferred",
    "userVerification": "preferred"
  }
}
```

```
POST /auth/passkey/register/finish
Headers: Authorization: Bearer <api_key>

Request:
{
  "id": "credential-id",
  "rawId": "base64-raw-id",
  "response": {
    "clientDataJSON": "base64",
    "attestationObject": "base64"
  },
  "type": "public-key"
}

Response:
{
  "success": true,
  "credential_id": "cred_xxxxxxxxxxxx"
}
```

### Passkey Authentication

```
POST /auth/passkey/auth/start
Request:
{
  "email": "user@example.com"
}

Response:
{
  "challenge": "base64-encoded-challenge",
  "timeout": 60000,
  "rpId": "todufit.example.com",
  "allowCredentials": [
    {
      "type": "public-key",
      "id": "base64-credential-id"
    }
  ],
  "userVerification": "preferred"
}
```

```
POST /auth/passkey/auth/finish
Request:
{
  "id": "credential-id",
  "rawId": "base64-raw-id",
  "response": {
    "clientDataJSON": "base64",
    "authenticatorData": "base64",
    "signature": "base64"
  },
  "type": "public-key"
}

Response:
{
  "api_key": "ak_xxxxxxxxxxxx",
  "user_id": "usr_xxxxxxxxxxxx",
  "email": "user@example.com"
}
```

## Client-Specific Details

### CLI

- Uses redirect-based magic link flow (local callback server)
- Stores API key in `~/.config/fit/config.yaml`
- No passkey support (terminal limitation)
- API key used directly in WebSocket URL

### Web

- Uses API-mode magic link (`POST /auth/verify`)
- Supports passkeys via WebAuthn browser API
- Creates server-side session after auth
- Stores session ID in HTTP-only secure cookie
- Session maps to API key for sync operations

**Session structure:**
```
sessions table:
  - session_id: string (random, in cookie)
  - user_id: string
  - email: string
  - api_key: string (for sync)
  - created_at: timestamp
  - expires_at: timestamp
```

### iOS

- Uses Universal Links for magic link callback (`todufit://auth`)
- Supports passkeys via `ASAuthorizationController`
- Stores API key in iOS Keychain
- API key used directly in WebSocket URL
- Passkeys sync automatically via iCloud Keychain

## Security Considerations

### API Keys

- Generated server-side, cryptographically random
- Format: `ak_` prefix + 32 random bytes (base64)
- Stored hashed in database (bcrypt or argon2)
- Can be revoked per-user
- Consider: expiration with refresh mechanism

### Magic Links

- Single-use tokens
- Short expiration (10 minutes)
- Rate-limited per email
- Stored hashed, deleted after use

### Passkeys

- Private keys never leave device
- Public keys stored server-side
- Challenge-response prevents replay
- Counter prevents cloning attacks

### Sessions (Web)

- HTTP-only, Secure, SameSite=Strict cookies
- Session ID is random, not derived from user data
- Server-side storage (not JWT)
- Expiration: 7-30 days (configurable)
- Invalidate on logout

## Database Schema (Sync Server)

```sql
-- Users
CREATE TABLE users (
  id TEXT PRIMARY KEY,           -- usr_xxxxxxxxxxxx
  email TEXT UNIQUE NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- API Keys
CREATE TABLE api_keys (
  id TEXT PRIMARY KEY,           -- ak_xxxxxxxxxxxx
  user_id TEXT NOT NULL REFERENCES users(id),
  key_hash TEXT NOT NULL,        -- hashed API key
  name TEXT,                     -- optional: "CLI", "iPhone", etc.
  last_used_at TEXT,
  created_at TEXT NOT NULL,
  expires_at TEXT,               -- optional expiration
  revoked_at TEXT                -- soft delete
);

-- Magic Links
CREATE TABLE magic_links (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  token_hash TEXT NOT NULL,
  callback_url TEXT NOT NULL,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  used_at TEXT                   -- NULL until used
);

-- Passkey Credentials
CREATE TABLE passkey_credentials (
  id TEXT PRIMARY KEY,           -- cred_xxxxxxxxxxxx
  user_id TEXT NOT NULL REFERENCES users(id),
  credential_id BLOB NOT NULL,   -- WebAuthn credential ID
  public_key BLOB NOT NULL,      -- COSE public key
  counter INTEGER NOT NULL,      -- signature counter
  transports TEXT,               -- JSON array: ["internal", "hybrid"]
  created_at TEXT NOT NULL,
  last_used_at TEXT
);

-- Indexes
CREATE INDEX idx_api_keys_user ON api_keys(user_id);
CREATE INDEX idx_magic_links_token ON magic_links(token_hash);
CREATE INDEX idx_passkey_user ON passkey_credentials(user_id);
CREATE UNIQUE INDEX idx_passkey_credential ON passkey_credentials(credential_id);
```

## Implementation Notes

### Rust Libraries

- **webauthn-rs** - WebAuthn/passkey implementation for sync server
- **argon2** - Password/key hashing
- **rand** - Secure random generation

### Swift (iOS)

- **ASAuthorizationController** - System passkey API
- **Security.framework** - Keychain access

### Browser (Web)

- Native WebAuthn API (`navigator.credentials`)
- Optional: `@simplewebauthn/browser` for convenience
