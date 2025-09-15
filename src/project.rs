/* src/project.rs */

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

// Added function for pnpm config
fn get_default_pnpm_config() -> ProjectConfig {
    let mut scripts = HashMap::new();
    scripts.insert("dev".to_string(), "pnpm dev".to_string());
    scripts.insert("build".to_string(), "pnpm build".to_string());
    scripts.insert("lint".to_string(), "pnpm lint".to_string());
    scripts.insert("publish".to_string(), "pnpm publish".to_string());
    scripts.insert("install".to_string(), "pnpm install".to_string());
    scripts.insert("clean".to_string(), "pnpm clean".to_string()); // You might want a more specific clean script
    scripts.insert("add".to_string(), "pnpm add".to_string());
    scripts.insert("remove".to_string(), "pnpm remove".to_string());
    ProjectConfig { scripts }
}

/// Helper function to handle config creation and saving.
fn create_and_save_config(
    config_path: &Path,
    default_config: ProjectConfig,
) -> Result<Option<ProjectConfig>> {
    let config_json = serde_json::to_string_pretty(&default_config)?;
    fs::write(config_path, config_json)?;
    Ok(Some(default_config))
}

/// Attempts to load a config, creating it if it doesn't exist.
pub fn load_or_create_config() -> Result<Option<ProjectConfig>> {
    let current_dir = std::env::current_dir()?;
    let config_path = current_dir.join("clay-config.json");

    // Handle existing config first
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        match serde_json::from_str(&content) {
            Ok(config) => return Ok(Some(config)),
            Err(_) => {
                let backup_path = current_dir.join("clay-config.json.bak");
                fs::rename(&config_path, backup_path)?;
                // TUI will show a log message.
            }
        }
    }

    // Detect project type and create a new config if it doesn't exist or was invalid
    if current_dir.join("pnpm-lock.yaml").exists() {
        let default_config = get_default_pnpm_config();
        return create_and_save_config(&config_path, default_config);
    } else if current_dir.join("Cargo.toml").exists() {
        let default_config = get_default_rust_config();
        return create_and_save_config(&config_path, default_config);
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
