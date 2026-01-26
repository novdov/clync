use std::env;
use std::fs;
use std::path::PathBuf;

use crate::error::ClaudyError;
use crate::Result;

use super::Config;

pub fn config_dir() -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".claudy")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn claude_dir() -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".claude")
}

pub fn load_config() -> Result<Config> {
    let path = config_path();

    if !path.exists() {
        return Ok(Config::new());
    }

    let content = fs::read_to_string(&path).map_err(|e| {
        ClaudyError::ConfigParse(format!("Failed to read config file: {}", e))
    })?;

    toml::from_str(&content).map_err(|e| {
        ClaudyError::ConfigParse(format!("Failed to parse config file: {}", e))
    })
}

pub fn save_config(config: &Config) -> Result<()> {
    let dir = config_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let content = toml::to_string_pretty(config).map_err(|e| {
        ClaudyError::ConfigParse(format!("Failed to serialize config: {}", e))
    })?;

    fs::write(config_path(), content)?;

    Ok(())
}
