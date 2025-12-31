//! ToduFit Sync Server
//!
//! A sync server for ToduFit that stores Automerge documents and enables
//! multi-device synchronization.
//!
//! # Configuration
//!
//! Environment variables:
//! - `TODUFIT_PORT`: Port to listen on (default: 8080)
//! - `TODUFIT_DATA_DIR`: Directory to store documents (default: ~/.local/share/todufit-server)
//! - `TODUFIT_CONFIG`: Path to config file (default: ~/.config/todufit-server/config.yaml)
//!
//! # Config File Format
//!
//! ```yaml
//! api_keys:
//!   - key: "your-secret-key-here"
//!     user_id: "user1"
//!     group_id: "family1"
//! ```
//!
//! # Endpoints
//!
//! - `GET /health`: Health check endpoint (no auth required)
//! - `GET /me`: Returns current user info (auth required)
//! - `GET /docs/:doc_type`: Get document bytes (auth required)
//! - `PUT /docs/:doc_type`: Save document bytes (auth required)

use axum::{
    body::Bytes,
    extract::{Path, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use todufit::server::{ServerStorage, ServerStorageError};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// ============================================================================
// Configuration
// ============================================================================

/// API key entry in config
#[derive(Debug, Clone, Deserialize)]
struct ApiKeyEntry {
    key: String,
    user_id: String,
    group_id: String,
}

/// Config file structure
#[derive(Debug, Clone, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    api_keys: Vec<ApiKeyEntry>,
}

/// Server configuration
#[derive(Debug, Clone)]
struct Config {
    /// Port to listen on
    port: u16,
    /// Directory to store Automerge documents
    data_dir: PathBuf,
    /// Path to config file
    config_path: PathBuf,
}

impl Config {
    /// Load configuration from environment variables
    fn from_env() -> Self {
        let port = std::env::var("TODUFIT_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let data_dir = std::env::var("TODUFIT_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("todufit-server")
            });

        let config_path = std::env::var("TODUFIT_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("todufit-server")
                    .join("config.yaml")
            });

        Self {
            port,
            data_dir,
            config_path,
        }
    }
}

// ============================================================================
// Authentication
// ============================================================================

/// Authenticated user info, added to request extensions after auth
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    pub group_id: String,
}

/// API key store - maps key -> AuthUser
#[derive(Debug, Clone)]
struct ApiKeyStore {
    keys: HashMap<String, AuthUser>,
}

impl ApiKeyStore {
    /// Load API keys from config file
    fn load(config_path: &PathBuf) -> Self {
        let keys = match std::fs::read_to_string(config_path) {
            Ok(contents) => match serde_yaml::from_str::<ConfigFile>(&contents) {
                Ok(config) => {
                    let mut map = HashMap::new();
                    for entry in config.api_keys {
                        map.insert(
                            entry.key,
                            AuthUser {
                                user_id: entry.user_id,
                                group_id: entry.group_id,
                            },
                        );
                    }
                    tracing::info!("Loaded {} API key(s)", map.len());
                    map
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config file: {}", e);
                    HashMap::new()
                }
            },
            Err(e) => {
                tracing::warn!(
                    "Failed to read config file {}: {}",
                    config_path.display(),
                    e
                );
                tracing::warn!("No API keys loaded - all authenticated requests will fail");
                HashMap::new()
            }
        };

        Self { keys }
    }

    /// Validate an API key and return the associated user
    fn validate(&self, key: &str) -> Option<AuthUser> {
        self.keys.get(key).cloned()
    }
}

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    api_keys: Arc<ApiKeyStore>,
    storage: Arc<RwLock<ServerStorage>>,
}

/// Auth error response
#[derive(Serialize)]
struct AuthError {
    error: &'static str,
    message: &'static str,
}

/// Authentication middleware
async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let api_key = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        Some(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "invalid_auth",
                    message: "Authorization header must use Bearer scheme",
                }),
            )
                .into_response();
        }
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "missing_auth",
                    message: "Authorization header required",
                }),
            )
                .into_response();
        }
    };

    // Validate API key
    match state.api_keys.validate(api_key) {
        Some(user) => {
            // Add user info to request extensions
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(AuthError {
                error: "invalid_key",
                message: "Invalid API key",
            }),
        )
            .into_response(),
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Health check endpoint (no auth required)
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Current user response
#[derive(Serialize)]
struct MeResponse {
    user_id: String,
    group_id: String,
}

/// Get current user info (auth required)
async fn me(Extension(user): Extension<AuthUser>) -> Json<MeResponse> {
    Json(MeResponse {
        user_id: user.user_id,
        group_id: user.group_id,
    })
}

/// Error response for document operations
#[derive(Serialize)]
struct DocError {
    error: String,
    message: String,
}

impl DocError {
    fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
        }
    }
}

/// Get a document (returns raw Automerge bytes)
async fn get_doc(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(doc_type): Path<String>,
) -> Response {
    let storage = state.storage.read().await;

    match storage.load_by_name(&user.group_id, &doc_type) {
        Ok(Some(doc)) => {
            // Return raw bytes with appropriate content type
            let mut doc = doc;
            let bytes = doc.save();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/octet-stream")],
                bytes,
            )
                .into_response()
        }
        Ok(None) => {
            // No document yet - return 404
            (
                StatusCode::NOT_FOUND,
                Json(DocError::new("not_found", "Document not found")),
            )
                .into_response()
        }
        Err(ServerStorageError::InvalidDocType(t)) => (
            StatusCode::BAD_REQUEST,
            Json(DocError::new(
                "invalid_doc_type",
                format!("Invalid document type: {}", t),
            )),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to load document: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DocError::new("storage_error", "Failed to load document")),
            )
                .into_response()
        }
    }
}

/// Save a document (accepts raw Automerge bytes)
async fn put_doc(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(doc_type): Path<String>,
    body: Bytes,
) -> Response {
    // Validate the bytes are a valid Automerge document
    let mut doc = match automerge::AutoCommit::load(&body) {
        Ok(doc) => doc,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(DocError::new(
                    "invalid_document",
                    format!("Invalid Automerge document: {}", e),
                )),
            )
                .into_response();
        }
    };

    let storage = state.storage.write().await;

    match storage.save_by_name(&user.group_id, &doc_type, &mut doc) {
        Ok(()) => {
            tracing::info!(
                "Saved {} for group {} by user {}",
                doc_type,
                user.group_id,
                user.user_id
            );
            (StatusCode::NO_CONTENT, ()).into_response()
        }
        Err(ServerStorageError::InvalidDocType(t)) => (
            StatusCode::BAD_REQUEST,
            Json(DocError::new(
                "invalid_doc_type",
                format!("Invalid document type: {}", t),
            )),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to save document: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DocError::new("storage_error", "Failed to save document")),
            )
                .into_response()
        }
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "todufit_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env();

    // Ensure data directory exists
    if let Err(e) = std::fs::create_dir_all(&config.data_dir) {
        tracing::error!("Failed to create data directory: {}", e);
        std::process::exit(1);
    }

    tracing::info!("Data directory: {}", config.data_dir.display());
    tracing::info!("Config file: {}", config.config_path.display());

    // Load API keys
    let api_keys = Arc::new(ApiKeyStore::load(&config.config_path));

    // Create storage
    let storage = Arc::new(RwLock::new(ServerStorage::new(&config.data_dir)));

    // Build app state
    let state = AppState { api_keys, storage };

    // Build router
    // Public routes (no auth)
    let public_routes = Router::new().route("/health", get(health));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route("/me", get(me))
        .route("/docs/{doc_type}", get(get_doc).put(put_doc))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
