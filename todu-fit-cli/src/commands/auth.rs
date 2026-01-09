//! Authentication commands for the ToduFit CLI.
//!
//! Provides login, logout, and status commands for magic link authentication.

use crate::config::Config;
use axum::{extract::Query, response::Html, routing::get, Router};
use clap::{Args, Subcommand};
use serde::Deserialize;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::oneshot;

/// Authentication commands
#[derive(Args)]
pub struct AuthCommand {
    #[command(subcommand)]
    command: AuthSubcommand,
}

#[derive(Subcommand)]
enum AuthSubcommand {
    /// Log in with email (magic link authentication)
    Login,
    /// Log out (remove API key from config)
    Logout,
    /// Show authentication status
    Status,
}

impl AuthCommand {
    pub fn run(&self, config: &Config) -> Result<(), AuthError> {
        // Use tokio runtime for async operations
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| AuthError::ConfigError(format!("Failed to create runtime: {}", e)))?;

        match &self.command {
            AuthSubcommand::Login => rt.block_on(login(config)),
            AuthSubcommand::Logout => logout(config),
            AuthSubcommand::Status => status(config),
        }
    }
}

/// Errors that can occur during authentication
#[derive(Debug)]
pub enum AuthError {
    /// I/O error
    IoError(io::Error),
    /// HTTP request error
    HttpError(String),
    /// Server returned an error
    ServerError { error: String, message: String },
    /// Config file error
    ConfigError(String),
    /// Timeout waiting for callback
    Timeout,
    /// Server not configured
    NotConfigured,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::IoError(e) => write!(f, "I/O error: {}", e),
            AuthError::HttpError(e) => write!(f, "HTTP error: {}", e),
            AuthError::ServerError { error, message } => {
                write!(f, "{}: {}", error, message)
            }
            AuthError::ConfigError(e) => write!(f, "Config error: {}", e),
            AuthError::Timeout => write!(f, "Timed out waiting for authentication"),
            AuthError::NotConfigured => {
                write!(
                    f,
                    "Sync server not configured. Set sync.server_url in config."
                )
            }
        }
    }
}

impl std::error::Error for AuthError {}

impl From<io::Error> for AuthError {
    fn from(e: io::Error) -> Self {
        AuthError::IoError(e)
    }
}

/// Interactive login flow
async fn login(config: &Config) -> Result<(), AuthError> {
    // Check if server_url is configured
    let server_url = config
        .sync
        .server_url
        .as_ref()
        .ok_or(AuthError::NotConfigured)?;

    // Convert ws:// to http:// for auth endpoints
    let http_url = server_url
        .replace("ws://", "http://")
        .replace("wss://", "https://");

    // Prompt for email
    print!("Enter your email: ");
    io::stdout().flush()?;
    let mut email = String::new();
    io::stdin().read_line(&mut email)?;
    let email = email.trim().to_string();

    if email.is_empty() {
        return Err(AuthError::IoError(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Email cannot be empty",
        )));
    }

    // Create channel to receive the result
    let (tx, rx) = oneshot::channel::<CallbackResult>();
    let tx = Arc::new(std::sync::Mutex::new(Some(tx)));

    // Start local callback server using tokio's async TcpListener
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let local_port = listener.local_addr()?.port();
    let callback_url = format!("http://127.0.0.1:{}/callback", local_port);

    // Start the callback server
    let tx_clone = tx.clone();
    let server_handle = tokio::spawn(async move {
        let app = Router::new().route(
            "/callback",
            get(move |Query(params): Query<CallbackParams>| {
                let tx = tx_clone.clone();
                async move {
                    // Send result through channel
                    if let Some(tx) = tx.lock().unwrap().take() {
                        let _ = tx.send(CallbackResult {
                            api_key: params.key,
                            user: params.user,
                        });
                    }

                    Html(
                        r#"<!DOCTYPE html>
<html>
<head><title>Todu Fit - Success</title></head>
<body>
<h1>Authentication successful!</h1>
<p>You can close this window and return to the terminal.</p>
</body>
</html>"#,
                    )
                }
            }),
        );

        axum::serve(listener, app).await.unwrap();
    });

    // Request magic link from server
    println!("Requesting magic link...");

    let client = reqwest::Client::new();
    let login_url = format!("{}/auth/login", http_url);

    let response = client
        .post(&login_url)
        .json(&serde_json::json!({
            "email": email,
            "callback_url": callback_url
        }))
        .send()
        .await
        .map_err(|e| AuthError::HttpError(e.to_string()))?;

    if !response.status().is_success() {
        let error: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AuthError::HttpError(e.to_string()))?;

        return Err(AuthError::ServerError {
            error: error["error"].as_str().unwrap_or("unknown").to_string(),
            message: error["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string(),
        });
    }

    println!("Magic link sent to {}. Check your inbox...", email);
    println!("Waiting for you to click the link (timeout: 5 minutes)");

    // Wait for callback with timeout
    let result = tokio::time::timeout(std::time::Duration::from_secs(300), rx).await;

    // Shutdown server
    server_handle.abort();

    match result {
        Ok(Ok(callback)) => {
            // Save API key to config
            let config_path = config
                .config_file
                .clone()
                .unwrap_or_else(Config::default_config_path);
            save_api_key(&callback.api_key, &config_path)?;
            println!("Authenticated as {}", callback.user);
            Ok(())
        }
        Ok(Err(_)) => Err(AuthError::Timeout),
        Err(_) => Err(AuthError::Timeout),
    }
}

/// Callback parameters from the verify redirect
#[derive(Deserialize)]
struct CallbackParams {
    key: String,
    user: String,
}

/// Result from callback
struct CallbackResult {
    api_key: String,
    user: String,
}

/// Save API key to config file
fn save_api_key(api_key: &str, config_path: &std::path::Path) -> Result<(), AuthError> {
    // Read existing config or create new
    let mut config: serde_yaml::Value = if config_path.exists() {
        let contents = std::fs::read_to_string(config_path)
            .map_err(|e| AuthError::ConfigError(e.to_string()))?;
        serde_yaml::from_str(&contents).map_err(|e| AuthError::ConfigError(e.to_string()))?
    } else {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    };

    // Ensure sync section exists
    let mapping = config
        .as_mapping_mut()
        .ok_or_else(|| AuthError::ConfigError("Invalid config format".to_string()))?;

    let sync_key = serde_yaml::Value::String("sync".to_string());
    if !mapping.contains_key(&sync_key) {
        mapping.insert(
            sync_key.clone(),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
    }

    // Set api_key
    if let Some(sync) = mapping.get_mut(&sync_key) {
        if let Some(sync_mapping) = sync.as_mapping_mut() {
            sync_mapping.insert(
                serde_yaml::Value::String("api_key".to_string()),
                serde_yaml::Value::String(api_key.to_string()),
            );
        }
    }

    // Create config directory if needed
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AuthError::ConfigError(e.to_string()))?;
    }

    // Write config
    let yaml = serde_yaml::to_string(&config).map_err(|e| AuthError::ConfigError(e.to_string()))?;
    std::fs::write(config_path, yaml).map_err(|e| AuthError::ConfigError(e.to_string()))?;

    Ok(())
}

/// Remove API key from config
fn logout(config: &Config) -> Result<(), AuthError> {
    let config_path = config
        .config_file
        .clone()
        .unwrap_or_else(Config::default_config_path);

    if !config_path.exists() {
        println!("Already logged out (no config file).");
        return Ok(());
    }

    // Read existing config
    let contents =
        std::fs::read_to_string(&config_path).map_err(|e| AuthError::ConfigError(e.to_string()))?;
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&contents).map_err(|e| AuthError::ConfigError(e.to_string()))?;

    // Remove api_key from sync section
    if let Some(mapping) = yaml.as_mapping_mut() {
        let sync_key = serde_yaml::Value::String("sync".to_string());
        if let Some(sync) = mapping.get_mut(&sync_key) {
            if let Some(sync_mapping) = sync.as_mapping_mut() {
                sync_mapping.remove(serde_yaml::Value::String("api_key".to_string()));
            }
        }
    }

    // Write config
    let yaml_str =
        serde_yaml::to_string(&yaml).map_err(|e| AuthError::ConfigError(e.to_string()))?;
    std::fs::write(&config_path, yaml_str).map_err(|e| AuthError::ConfigError(e.to_string()))?;

    println!("Logged out. Sync disabled until you log in again.");
    Ok(())
}

/// Show authentication status
fn status(config: &Config) -> Result<(), AuthError> {
    if config.sync.is_configured() {
        let key = config.sync.api_key.as_ref().unwrap();
        // Mask the key for display
        let masked = if key.len() > 8 {
            format!("{}...{}", &key[..4], &key[key.len() - 4..])
        } else {
            "****".to_string()
        };
        println!("Logged in (API key: {})", masked);
    } else if config.sync.server_url.is_some() {
        println!("Not logged in. Run 'fit auth login' to authenticate.");
    } else {
        println!("Not configured. Set sync.server_url in config first.");
    }
    Ok(())
}
