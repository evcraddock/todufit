# Sync Server Changes

## Overview

This document describes the changes needed to the todu-sync server to support Web and iOS clients in addition to the existing CLI.

## Current State

The sync server currently provides:
- Magic link authentication (with redirect-based verification)
- API key issuance and validation
- WebSocket-based Automerge sync

## Required Changes

### 1. Add POST /auth/verify Endpoint

**Why:** Web and iOS need to verify magic links via API call (not redirect).

**Current flow (CLI):**
```
Server verifies token → Redirects to callback_url?key=xxx&user=yyy
```

**New flow (Web/iOS):**
```
POST /auth/verify
Request:  { "token": "magic-link-token" }
Response: { "api_key": "xxx", "user_id": "yyy", "email": "zzz" }
```

**Implementation:**

```rust
// In auth routes
async fn verify_magic_link_api(
    State(state): State<AppState>,
    Json(input): Json<VerifyInput>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Reuse existing verification logic
    let (user, api_key) = state.auth_service
        .verify_magic_link(&input.token)
        .await?;
    
    Ok(Json(AuthResponse {
        api_key: api_key.key,
        user_id: user.id,
        email: user.email,
    }))
}

#[derive(Deserialize)]
struct VerifyInput {
    token: String,
}

#[derive(Serialize)]
struct AuthResponse {
    api_key: String,
    user_id: String,
    email: String,
}

// Add route
Router::new()
    .route("/auth/verify", get(verify_magic_link_redirect))  // existing
    .route("/auth/verify", post(verify_magic_link_api))      // new
```

### 2. Add Passkey Support

**Why:** Better UX for Web and iOS, no email round-trip needed.

**New endpoints:**

```
POST /auth/passkey/register/start    # Begin registration (requires existing auth)
POST /auth/passkey/register/finish   # Complete registration
POST /auth/passkey/auth/start        # Begin authentication
POST /auth/passkey/auth/finish       # Complete authentication, return API key
```

**Database changes:**

```sql
CREATE TABLE passkey_credentials (
    id TEXT PRIMARY KEY,                -- cred_xxxxxxxxxxxx
    user_id TEXT NOT NULL REFERENCES users(id),
    credential_id BLOB NOT NULL UNIQUE, -- WebAuthn credential ID
    public_key BLOB NOT NULL,           -- COSE public key
    counter INTEGER NOT NULL DEFAULT 0, -- Signature counter
    transports TEXT,                    -- JSON: ["internal", "hybrid"]
    created_at TEXT NOT NULL,
    last_used_at TEXT
);

CREATE INDEX idx_passkey_user ON passkey_credentials(user_id);
CREATE INDEX idx_passkey_credential ON passkey_credentials(credential_id);
```

**Implementation using webauthn-rs:**

```rust
use webauthn_rs::prelude::*;

pub struct PasskeyService {
    webauthn: Webauthn,
    db: SqlitePool,
}

impl PasskeyService {
    pub fn new(db: SqlitePool, config: &Config) -> Self {
        let rp_id = config.rp_id.clone();  // e.g., "todufit.example.com"
        let rp_origin = Url::parse(&config.rp_origin).unwrap();
        let builder = WebauthnBuilder::new(&rp_id, &rp_origin)
            .unwrap()
            .rp_name("Todufit");
        let webauthn = builder.build().unwrap();
        
        Self { webauthn, db }
    }
    
    // Registration
    pub async fn start_registration(
        &self,
        user: &User,
    ) -> Result<(CreationChallengeResponse, PasskeyRegistration), Error> {
        // Get existing credentials to exclude
        let existing = self.get_user_credentials(&user.id).await?;
        let exclude: Vec<CredentialID> = existing
            .iter()
            .map(|c| c.credential_id.clone())
            .collect();
        
        let (challenge, reg_state) = self.webauthn
            .start_passkey_registration(
                Uuid::parse_str(&user.id)?,
                &user.email,
                &user.email,
                Some(exclude),
            )?;
        
        Ok((challenge, reg_state))
    }
    
    pub async fn finish_registration(
        &self,
        user: &User,
        reg_state: &PasskeyRegistration,
        response: &RegisterPublicKeyCredential,
    ) -> Result<PasskeyCredential, Error> {
        let passkey = self.webauthn
            .finish_passkey_registration(response, reg_state)?;
        
        // Store in database
        let credential = PasskeyCredential {
            id: format!("cred_{}", generate_id()),
            user_id: user.id.clone(),
            credential_id: passkey.cred_id().to_vec(),
            public_key: passkey.cred_pk().to_vec(),
            counter: 0,
            transports: serde_json::to_string(&passkey.transports())?,
            created_at: Utc::now(),
            last_used_at: None,
        };
        
        self.save_credential(&credential).await?;
        
        Ok(credential)
    }
    
    // Authentication
    pub async fn start_authentication(
        &self,
        email: &str,
    ) -> Result<(RequestChallengeResponse, PasskeyAuthentication), Error> {
        let user = self.get_user_by_email(email).await?
            .ok_or(Error::UserNotFound)?;
        
        let credentials = self.get_user_credentials(&user.id).await?;
        if credentials.is_empty() {
            return Err(Error::NoPasskeys);
        }
        
        // Convert to webauthn-rs format
        let passkeys: Vec<Passkey> = credentials
            .iter()
            .map(|c| c.to_passkey())
            .collect();
        
        let (challenge, auth_state) = self.webauthn
            .start_passkey_authentication(&passkeys)?;
        
        Ok((challenge, auth_state))
    }
    
    pub async fn finish_authentication(
        &self,
        auth_state: &PasskeyAuthentication,
        response: &PublicKeyCredential,
    ) -> Result<(User, ApiKey), Error> {
        let auth_result = self.webauthn
            .finish_passkey_authentication(response, auth_state)?;
        
        // Update counter
        self.update_credential_counter(
            &auth_result.cred_id(),
            auth_result.counter(),
        ).await?;
        
        // Get user and create API key
        let credential = self.get_credential_by_id(&auth_result.cred_id()).await?;
        let user = self.get_user(&credential.user_id).await?;
        let api_key = self.create_api_key(&user).await?;
        
        Ok((user, api_key))
    }
}
```

**Route handlers:**

```rust
// Registration (requires auth)
async fn passkey_register_start(
    State(state): State<AppState>,
    auth: AuthenticatedUser,  // Extract from API key
) -> Result<Json<CreationChallengeResponse>, ApiError> {
    let (challenge, reg_state) = state.passkey_service
        .start_registration(&auth.user)
        .await?;
    
    // Store reg_state in session/cache (keyed by user_id)
    state.reg_states.insert(auth.user.id.clone(), reg_state);
    
    Ok(Json(challenge))
}

async fn passkey_register_finish(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(response): Json<RegisterPublicKeyCredential>,
) -> Result<Json<CredentialCreated>, ApiError> {
    let reg_state = state.reg_states
        .remove(&auth.user.id)
        .ok_or(ApiError::InvalidState)?;
    
    let credential = state.passkey_service
        .finish_registration(&auth.user, &reg_state, &response)
        .await?;
    
    Ok(Json(CredentialCreated {
        credential_id: credential.id,
    }))
}

// Authentication (no auth required)
async fn passkey_auth_start(
    State(state): State<AppState>,
    Json(input): Json<PasskeyAuthStartInput>,
) -> Result<Json<RequestChallengeResponse>, ApiError> {
    let (challenge, auth_state) = state.passkey_service
        .start_authentication(&input.email)
        .await?;
    
    // Store auth_state (keyed by challenge)
    let challenge_id = challenge.public_key.challenge.to_string();
    state.auth_states.insert(challenge_id, auth_state);
    
    Ok(Json(challenge))
}

async fn passkey_auth_finish(
    State(state): State<AppState>,
    Json(input): Json<PasskeyAuthFinishInput>,
) -> Result<Json<AuthResponse>, ApiError> {
    // Extract challenge from clientDataJSON to look up state
    let challenge_id = extract_challenge(&input.response)?;
    let auth_state = state.auth_states
        .remove(&challenge_id)
        .ok_or(ApiError::InvalidState)?;
    
    let (user, api_key) = state.passkey_service
        .finish_authentication(&auth_state, &input.response)
        .await?;
    
    Ok(Json(AuthResponse {
        api_key: api_key.key,
        user_id: user.id,
        email: user.email,
    }))
}
```

### 3. Universal Links / App Links Support

**Why:** iOS needs to handle magic link callbacks via Universal Links.

**Add endpoint:**

```
GET /.well-known/apple-app-site-association
```

**Implementation:**

```rust
async fn apple_app_site_association() -> Json<Value> {
    Json(json!({
        "applinks": {
            "apps": [],
            "details": [{
                "appID": "TEAMID.com.example.todufit",
                "paths": ["/auth/*"]
            }]
        },
        "webcredentials": {
            "apps": ["TEAMID.com.example.todufit"]
        }
    }))
}

// Add route (must be served without Content-Type charset)
Router::new()
    .route("/.well-known/apple-app-site-association", get(apple_app_site_association))
```

**Note:** The file must be served with `Content-Type: application/json` (no charset).

### 4. CORS Support (for Web)

**Why:** Web app may be on different origin during development.

```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin(Any)  // Or specific origins in production
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

let app = Router::new()
    // ... routes
    .layer(cors);
```

### 5. Session State Storage

**Why:** Passkey flows require storing challenge state between requests.

**Options:**

1. **In-memory cache (simple):**
```rust
use dashmap::DashMap;
use std::time::{Duration, Instant};

struct StateCache<T> {
    map: DashMap<String, (T, Instant)>,
    ttl: Duration,
}

impl<T> StateCache<T> {
    fn insert(&self, key: String, value: T) {
        self.map.insert(key, (value, Instant::now()));
    }
    
    fn remove(&self, key: &str) -> Option<T> {
        self.map.remove(key)
            .filter(|(_, (_, created))| created.elapsed() < self.ttl)
            .map(|(_, (value, _))| value)
    }
}
```

2. **Redis (for multi-instance deployments):**
```rust
// If you need horizontal scaling
use redis::AsyncCommands;

async fn store_state(redis: &mut Connection, key: &str, state: &PasskeyRegistration) {
    let serialized = serde_json::to_string(state).unwrap();
    redis.set_ex(key, serialized, 300).await.unwrap();  // 5 min TTL
}
```

For homelab k3s with single instance, in-memory is fine.

## Configuration Changes

```yaml
# config.yaml additions

# Relying Party configuration (for passkeys)
rp_id: "todufit.example.com"  # Domain, no scheme
rp_origin: "https://todufit.example.com"  # Full origin with scheme

# Apple App Site Association
apple_team_id: "XXXXXXXXXX"
apple_bundle_id: "com.example.todufit"

# CORS (optional, for development)
cors_origins:
  - "http://localhost:3000"
  - "https://todufit.example.com"
```

## Migration Plan

### Phase 1: Non-Breaking Additions
1. Add `POST /auth/verify` endpoint
2. Add passkey database tables
3. Add passkey endpoints
4. Add apple-app-site-association endpoint
5. Add CORS support

All changes are additive—existing CLI continues to work.

### Phase 2: Web Client
- Web client uses new endpoints
- Test passkey flow end-to-end

### Phase 3: iOS Client
- Configure Universal Links
- Test magic link callback flow
- Test passkey flow

## API Summary

| Endpoint | Method | Auth Required | Used By |
|----------|--------|---------------|---------|
| `/auth/login` | POST | No | All |
| `/auth/verify` | GET | No | CLI |
| `/auth/verify` | POST | No | Web, iOS |
| `/auth/passkey/register/start` | POST | Yes (API key) | Web, iOS |
| `/auth/passkey/register/finish` | POST | Yes (API key) | Web, iOS |
| `/auth/passkey/auth/start` | POST | No | Web, iOS |
| `/auth/passkey/auth/finish` | POST | No | Web, iOS |
| `/sync/:doc_type` | WebSocket | Yes (query param) | All |
| `/.well-known/apple-app-site-association` | GET | No | iOS |

## Dependencies to Add

```toml
# Cargo.toml

[dependencies]
webauthn-rs = { version = "0.5", features = ["danger-allow-state-serialisation"] }
dashmap = "5"  # For in-memory state cache
tower-http = { version = "0.5", features = ["cors"] }
```

The `danger-allow-state-serialisation` feature is needed if you want to serialize PasskeyRegistration/PasskeyAuthentication states (e.g., for Redis). For in-memory storage, you can avoid it by keeping the state in memory directly.
