use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Source of a configuration value
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigSource {
    Default,
    File,
    Environment,
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSource::Default => write!(f, "default"),
            ConfigSource::File => write!(f, "file"),
            ConfigSource::Environment => write!(f, "environment"),
        }
    }
}

/// A configuration value with its source
#[derive(Debug, Clone, Serialize)]
pub struct ConfigValue<T> {
    pub value: T,
    pub source: ConfigSource,
}

impl<T> ConfigValue<T> {
    pub fn new(value: T, source: ConfigSource) -> Self {
        Self { value, source }
    }
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncConfig {
    /// Server URL (e.g., "ws://localhost:8080" or "wss://sync.example.com")
    pub server_url: Option<String>,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Enable automatic sync after writes (default: false)
    #[serde(default)]
    pub auto_sync: bool,
}

impl SyncConfig {
    /// Returns true if sync is configured (has both server_url and api_key)
    pub fn is_configured(&self) -> bool {
        self.server_url.is_some() && self.api_key.is_some()
    }
}

/// Application configuration with source tracking
#[derive(Debug, Clone, Serialize)]
pub struct Config {
    /// Path to the SQLite database
    pub database_path: ConfigValue<PathBuf>,
    /// Default user name for new dishes
    pub created_by: ConfigValue<String>,
    /// Config file path used (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_file: Option<PathBuf>,
    /// Sync configuration
    pub sync: SyncConfig,
}

/// Internal struct for deserializing config file
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ConfigFile {
    database_path: Option<PathBuf>,
    created_by: Option<String>,
    sync: Option<SyncConfig>,
}

impl Config {
    /// Load configuration with priority: env vars > config file > defaults
    pub fn load(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let default_db_path = Self::default_data_dir().join("fit.db");
        let default_created_by = "default".to_string();

        // Start with defaults
        let mut database_path = ConfigValue::new(default_db_path.clone(), ConfigSource::Default);
        let mut created_by = ConfigValue::new(default_created_by.clone(), ConfigSource::Default);
        let mut config_file = None;
        let mut sync = SyncConfig::default();

        // Try to load from config file
        let path = config_path.unwrap_or_else(Self::default_config_path);
        if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| ConfigError::ReadError(path.clone(), e))?;
            let file_config: ConfigFile = serde_yaml::from_str(&contents)
                .map_err(|e| ConfigError::ParseError(path.clone(), e))?;

            config_file = Some(path.clone());

            if let Some(db_path) = file_config.database_path {
                // Resolve relative paths against config file's directory
                let resolved_path = if db_path.is_relative() {
                    path.parent().map(|p| p.join(&db_path)).unwrap_or(db_path)
                } else {
                    db_path
                };
                database_path = ConfigValue::new(resolved_path, ConfigSource::File);
            }
            if let Some(user) = file_config.created_by {
                created_by = ConfigValue::new(user, ConfigSource::File);
            }
            if let Some(sync_config) = file_config.sync {
                sync = sync_config;
            }
        }

        // Apply environment variable overrides
        if let Ok(db_path) = std::env::var("FIT_DATABASE_PATH") {
            database_path = ConfigValue::new(PathBuf::from(db_path), ConfigSource::Environment);
        }
        if let Ok(user) = std::env::var("FIT_CREATED_BY") {
            created_by = ConfigValue::new(user, ConfigSource::Environment);
        }
        // Sync env var overrides
        if let Ok(url) = std::env::var("FIT_SYNC_URL") {
            sync.server_url = Some(url);
        }
        if let Ok(key) = std::env::var("FIT_SYNC_API_KEY") {
            sync.api_key = Some(key);
        }

        Ok(Self {
            database_path,
            created_by,
            config_file,
            sync,
        })
    }

    /// Default config directory (platform-specific):
    /// - Linux: ~/.config/fit/
    /// - macOS: ~/Library/Application Support/fit/
    /// - Windows: %APPDATA%/fit/
    pub fn default_config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("fit")
    }

    /// Default data directory (platform-specific):
    /// - Linux: ~/.local/share/fit/
    /// - macOS: ~/Library/Application Support/fit/
    /// - Windows: %APPDATA%/fit/
    pub fn default_data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("fit")
    }

    /// Default config file path (platform-specific config dir + config.yaml)
    pub fn default_config_path() -> PathBuf {
        Self::default_config_dir().join("config.yaml")
    }
}

#[derive(Debug)]
pub enum ConfigError {
    ReadError(PathBuf, std::io::Error),
    ParseError(PathBuf, serde_yaml::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ReadError(path, e) => {
                write!(f, "Failed to read config file '{}': {}", path.display(), e)
            }
            ConfigError::ParseError(path, e) => {
                write!(f, "Failed to parse config file '{}': {}", path.display(), e)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yaml");

        let config = Config::load(Some(config_path)).unwrap();
        assert!(config
            .database_path
            .value
            .to_string_lossy()
            .contains("fit.db"));
        assert_eq!(config.database_path.source, ConfigSource::Default);
        assert_eq!(config.created_by.value, "default");
        assert_eq!(config.created_by.source, ConfigSource::Default);
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "database_path: /custom/path/db.sqlite").unwrap();
        writeln!(file, "created_by: testuser").unwrap();

        let config = Config::load(Some(config_path.clone())).unwrap();
        assert_eq!(
            config.database_path.value,
            PathBuf::from("/custom/path/db.sqlite")
        );
        assert_eq!(config.database_path.source, ConfigSource::File);
        assert_eq!(config.created_by.value, "testuser");
        assert_eq!(config.created_by.source, ConfigSource::File);
        assert_eq!(config.config_file, Some(config_path));
    }

    #[test]
    #[ignore] // Run with --ignored; env vars can pollute parallel tests
    fn test_env_var_overrides_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "created_by: fromfile").unwrap();

        // Set env var
        std::env::set_var("FIT_CREATED_BY", "fromenv");

        let config = Config::load(Some(config_path)).unwrap();
        assert_eq!(config.created_by.value, "fromenv");
        assert_eq!(config.created_by.source, ConfigSource::Environment);

        // Clean up
        std::env::remove_var("FIT_CREATED_BY");
    }

    #[test]
    fn test_invalid_yaml_error() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "invalid: yaml: content: [").unwrap();

        let result = Config::load(Some(config_path));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to parse config file"));
    }

    #[test]
    fn test_partial_file_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "created_by: fileuser").unwrap();
        // database_path not specified

        let config = Config::load(Some(config_path)).unwrap();
        assert_eq!(config.database_path.source, ConfigSource::Default);
        assert_eq!(config.created_by.value, "fileuser");
        assert_eq!(config.created_by.source, ConfigSource::File);
    }
}
