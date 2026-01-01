//! Temporary token storage for magic link authentication.
//!
//! Tokens are stored in memory and expire after a configurable time.
//! They are single-use - deleted after successful verification.

use rand::Rng;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Data associated with a token.
#[derive(Debug, Clone)]
pub struct TokenData {
    /// Email address the token was issued for.
    pub email: String,
    /// URL to redirect to after verification.
    pub callback_url: String,
    /// When the token was created.
    pub created_at: Instant,
    /// When the token expires.
    pub expires_at: Instant,
}

/// In-memory token store with expiry.
///
/// Thread-safe via internal RwLock.
#[derive(Debug)]
pub struct TokenStore {
    /// Tokens indexed by token string.
    tokens: RwLock<HashMap<String, TokenData>>,
    /// Default expiry duration.
    default_expiry: Duration,
}

impl TokenStore {
    /// Creates a new token store with the specified default expiry in minutes.
    pub fn new(expiry_minutes: u64) -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
            default_expiry: Duration::from_secs(expiry_minutes * 60),
        }
    }

    /// Creates a new token for the given email and callback URL.
    ///
    /// Returns the token string (32 bytes, base64url encoded).
    pub fn create_token(&self, email: &str, callback_url: &str) -> String {
        self.create_token_with_expiry(email, callback_url, self.default_expiry)
    }

    /// Creates a new token with a custom expiry duration.
    pub fn create_token_with_expiry(
        &self,
        email: &str,
        callback_url: &str,
        expiry: Duration,
    ) -> String {
        let token = generate_token();
        let now = Instant::now();

        let data = TokenData {
            email: email.to_string(),
            callback_url: callback_url.to_string(),
            created_at: now,
            expires_at: now + expiry,
        };

        let mut tokens = self.tokens.write().unwrap();
        tokens.insert(token.clone(), data);

        token
    }

    /// Verifies a token and returns its data if valid.
    ///
    /// The token is consumed (deleted) on successful verification.
    /// Returns `None` if the token is unknown or expired.
    pub fn verify_token(&self, token: &str) -> Option<TokenData> {
        let mut tokens = self.tokens.write().unwrap();

        // Remove the token (consume it)
        let data = tokens.remove(token)?;

        // Check if expired
        if Instant::now() > data.expires_at {
            return None;
        }

        Some(data)
    }

    /// Removes all expired tokens.
    ///
    /// Returns the number of tokens removed.
    pub fn cleanup_expired(&self) -> usize {
        let mut tokens = self.tokens.write().unwrap();
        let now = Instant::now();

        let before = tokens.len();
        tokens.retain(|_, data| data.expires_at > now);
        let after = tokens.len();

        before - after
    }

    /// Returns the number of tokens currently stored.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.tokens.read().unwrap().len()
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new(10) // 10 minutes default
    }
}

/// Generates a secure random token.
///
/// Returns 32 random bytes encoded as base64url (no padding).
fn generate_token() -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_create_token_returns_unique() {
        let store = TokenStore::new(10);

        let token1 = store.create_token("a@example.com", "http://localhost/cb");
        let token2 = store.create_token("b@example.com", "http://localhost/cb");

        assert_ne!(token1, token2);
        assert_eq!(token1.len(), 43); // 32 bytes base64url = 43 chars
    }

    #[test]
    fn test_verify_valid_token() {
        let store = TokenStore::new(10);

        let token = store.create_token("test@example.com", "http://localhost/callback");
        let data = store.verify_token(&token).unwrap();

        assert_eq!(data.email, "test@example.com");
        assert_eq!(data.callback_url, "http://localhost/callback");
    }

    #[test]
    fn test_verify_unknown_token() {
        let store = TokenStore::new(10);

        let result = store.verify_token("nonexistent-token");

        assert!(result.is_none());
    }

    #[test]
    fn test_verify_expired_token() {
        let store = TokenStore::new(10);

        // Create token that expires immediately
        let token =
            store.create_token_with_expiry("test@example.com", "http://cb", Duration::from_secs(0));

        // Small sleep to ensure expiry
        thread::sleep(Duration::from_millis(10));

        let result = store.verify_token(&token);

        assert!(result.is_none());
    }

    #[test]
    fn test_token_is_single_use() {
        let store = TokenStore::new(10);

        let token = store.create_token("test@example.com", "http://cb");

        // First verification succeeds
        let result1 = store.verify_token(&token);
        assert!(result1.is_some());

        // Second verification fails (token consumed)
        let result2 = store.verify_token(&token);
        assert!(result2.is_none());
    }

    #[test]
    fn test_cleanup_expired() {
        let store = TokenStore::new(10);

        // Create some tokens
        store.create_token_with_expiry("a@example.com", "http://cb", Duration::from_secs(0));
        store.create_token_with_expiry("b@example.com", "http://cb", Duration::from_secs(0));
        store.create_token("c@example.com", "http://cb"); // not expired

        // Wait for expiry
        thread::sleep(Duration::from_millis(10));

        assert_eq!(store.len(), 3);

        let removed = store.cleanup_expired();

        assert_eq!(removed, 2);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_generate_token_format() {
        let token = generate_token();

        // Should be base64url, 43 characters (32 bytes)
        assert_eq!(token.len(), 43);
        assert!(token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
