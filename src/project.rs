/* src/project.rs */

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectConfig {
    pub scripts: HashMap<String, String>,
}

fn get_default_rust_config() -> ProjectConfig {
    let mut scripts = HashMap::new();
    scripts.insert("dev".to_string(), "cargo run".to_string());
    scripts.insert("build".to_string(), "cargo build".to_string());
    scripts.insert("lint".to_string(), "cargo fmt --all".to_string());
    scripts.insert("publish".to_string(), "cargo publish".to_string());
    scripts.insert("install".to_string(), "cargo install --path .".to_string());
    scripts.insert("clean".to_string(), "cargo clean".to_string());
    scripts.insert("add".to_string(), "cargo add".to_string());
    scripts.insert("remove".to_string(), "cargo remove".to_string());
    ProjectConfig { scripts }
}

/// Attempts to load a config, creating it if it doesn't exist.
pub fn load_or_create_config() -> Result<Option<ProjectConfig>> {
    let current_dir = std::env::current_dir()?;
    // For now, we only detect Rust projects
    if current_dir.join("Cargo.toml").exists() {
        let config_path = current_dir.join("clay-config.json");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            match serde_json::from_str(&content) {
                Ok(config) => return Ok(Some(config)),
                Err(_) => {
                    let backup_path = current_dir.join("clay-config.json.bak");
                    fs::rename(&config_path, backup_path)?;
                    // We don't need to print here, the TUI will show a log message.
                }
            }
        }
        // Create a new config file if it doesn't exist or was invalid
        let default_config = get_default_rust_config();
        let config_json = serde_json::to_string_pretty(&default_config)?;
        fs::write(config_path, config_json)?;
        return Ok(Some(default_config));
    }
    Ok(None)
}

/// Attempts to load a config without creating or modifying files. Used by lint.
pub fn load_config() -> Result<Option<ProjectConfig>> {
    let config_path = std::env::current_dir()?.join("clay-config.json");
    if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        serde_json::from_str(&content)
            .map(Some)
            .map_err(|e| e.into())
    } else {
        Ok(None)
    }
}
