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
//!
//! # Endpoints
//!
//! - `GET /health`: Health check endpoint

use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Server configuration
#[derive(Debug, Clone)]
struct Config {
    /// Port to listen on
    port: u16,
    /// Directory to store Automerge documents
    data_dir: std::path::PathBuf,
}

impl Config {
    /// Load configuration from environment variables
    fn from_env() -> Self {
        let port = std::env::var("TODUFIT_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let data_dir = std::env::var("TODUFIT_DATA_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("todufit-server")
            });

        Self { port, data_dir }
    }
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Health check endpoint
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

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

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
