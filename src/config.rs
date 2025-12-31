use serde::Deserialize;
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Path to the SQLite database
    pub database_path: PathBuf,
    /// Default user name for new dishes
    pub created_by: String,
}

impl Default for Config {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self {
            database_path: PathBuf::from(&home).join(".todufit").join("todufit.db"),
            created_by: "default".to_string(),
        }
    }
}

impl Config {
    /// Load configuration with priority: env vars > config file > defaults
    pub fn load(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        // Start with defaults
        let mut config = Self::default();

        // Try to load from config file
        let path = config_path.unwrap_or_else(Self::default_config_path);
        if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| ConfigError::ReadError(path.clone(), e))?;
            config = serde_yaml::from_str(&contents)
                .map_err(|e| ConfigError::ParseError(path.clone(), e))?;
        }

        // Apply environment variable overrides
        if let Ok(db_path) = std::env::var("TODUFIT_DATABASE_PATH") {
            config.database_path = PathBuf::from(db_path);
        }
        if let Ok(created_by) = std::env::var("TODUFIT_CREATED_BY") {
            config.created_by = created_by;
        }

        Ok(config)
    }

    /// Default config file path: ~/.config/todufit/config.yaml
    pub fn default_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".config")
            .join("todufit")
            .join("config.yaml")
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
                write!(
                    f,
                    "Failed to parse config file '{}': {}",
                    path.display(),
                    e
                )
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
        let config = Config::default();
        assert!(config.database_path.to_string_lossy().contains("todufit.db"));
        assert_eq!(config.created_by, "default");
    }

    #[test]
    fn test_load_no_file_uses_defaults() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yaml");

        let config = Config::load(Some(config_path)).unwrap();
        assert_eq!(config.created_by, "default");
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "database_path: /custom/path/db.sqlite").unwrap();
        writeln!(file, "created_by: testuser").unwrap();

        let config = Config::load(Some(config_path)).unwrap();
        assert_eq!(
            config.database_path,
            PathBuf::from("/custom/path/db.sqlite")
        );
        assert_eq!(config.created_by, "testuser");
    }

    #[test]
    fn test_env_var_overrides_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "created_by: fromfile").unwrap();

        // Set env var
        std::env::set_var("TODUFIT_CREATED_BY", "fromenv");

        let config = Config::load(Some(config_path)).unwrap();
        assert_eq!(config.created_by, "fromenv");

        // Clean up
        std::env::remove_var("TODUFIT_CREATED_BY");
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
}
