// src/project.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum ProjectType {
    Rust,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClayConfig {
    pub scripts: HashMap<String, String>,
}

pub fn detect_project_type() -> ProjectType {
    if Path::new("Cargo.toml").exists() {
        ProjectType::Rust
    } else {
        ProjectType::Unknown
    }
}

fn get_default_rust_config() -> ClayConfig {
    let mut scripts = HashMap::new();
    scripts.insert("dev".to_string(), "cargo run".to_string());
    scripts.insert("build".to_string(), "cargo build".to_string());
    scripts.insert("lint".to_string(), "cargo fmt --all".to_string());
    scripts.insert("publish".to_string(), "cargo publish".to_string());
    scripts.insert("install".to_string(), "cargo install --path .".to_string());
    scripts.insert("clean".to_string(), "cargo clean".to_string());
    ClayConfig { scripts }
}

fn create_and_write_config(path: &str, config: &ClayConfig) -> anyhow::Result<()> {
    let config_str = serde_json::to_string_pretty(config)?;
    fs::write(path, config_str)?;
    Ok(())
}

pub fn load_or_create_config() -> anyhow::Result<Option<ClayConfig>> {
    let project_type = detect_project_type();
    if project_type == ProjectType::Unknown {
        return Ok(None);
    }

    let config_path = "clay-config.json";

    if !Path::new(config_path).exists() {
        println!("'clay-config.json' not found, creating a default for Rust project.");
        let config = get_default_rust_config();
        create_and_write_config(config_path, &config)?;
        return Ok(Some(config));
    }

    match fs::read_to_string(config_path) {
        Ok(content) => match serde_json::from_str::<ClayConfig>(&content) {
            Ok(config) => Ok(Some(config)),
            Err(_) => {
                println!("Failed to parse 'clay-config.json'. Backing up and creating a new one.");
                fs::rename(config_path, "clay-config.json.d")?;
                let config = get_default_rust_config();
                create_and_write_config(config_path, &config)?;
                Ok(Some(config))
            }
        },
        Err(e) => Err(anyhow::anyhow!("Failed to read 'clay-config.json': {}", e)),
    }
}
