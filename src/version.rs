/* src/version.rs */

use anyhow::{Context, Result, anyhow, bail};
use semver::Version;
use std::fs;
use std::path::Path;
use toml::{Table, Value};

// Add Pnpm to the enum for project types
enum ProjectType {
    Rust,
    Pnpm,
    Unknown,
}

// Update the detection logic to include pnpm projects (by checking for package.json)
fn detect_project_type(base_path: &Path) -> ProjectType {
    if base_path.join("Cargo.toml").exists() {
        ProjectType::Rust
    } else if base_path.join("package.json").exists() {
        ProjectType::Pnpm
    } else {
        ProjectType::Unknown
    }
}

pub fn version_update() -> Result<()> {
    change_version(VersionChange::Update)
}

pub fn version_bump() -> Result<()> {
    change_version(VersionChange::Bump)
}

enum VersionChange {
    Update, // patch + 1
    Bump,   // minor + 1, patch = 0
}

// Helper function to find and update version in a Cargo.toml file
fn update_cargo_toml_version(config_path: &Path, change: &VersionChange) -> Result<bool> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut version_line_index: Option<usize> = None;
    let mut old_version_str = String::new();
    let mut new_version_str = String::new();
    let mut in_package_section = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed_line = line.trim();
        if trimmed_line == "[package]" {
            in_package_section = true;
            continue;
        }

        if in_package_section && trimmed_line.starts_with('[') {
            break;
        }

        if in_package_section && trimmed_line.starts_with("version") {
            if let Some(version_val) = trimmed_line.split('=').nth(1) {
                let version_str = version_val.trim().trim_matches('"');
                let mut version = Version::parse(version_str)
                    .with_context(|| format!("Failed to parse version: '{}'", version_str))?;

                old_version_str = version.to_string();

                match change {
                    VersionChange::Update => version.patch += 1,
                    VersionChange::Bump => {
                        version.minor += 1;
                        version.patch = 0;
                        version.pre = semver::Prerelease::EMPTY;
                        version.build = semver::BuildMetadata::EMPTY;
                    }
                }
                new_version_str = version.to_string();
                version_line_index = Some(i);
                break;
            }
        }
    }

    if let Some(index) = version_line_index {
        // Replace the version string directly to preserve formatting
        lines[index] = lines[index].replace(&old_version_str, &new_version_str);

        fs::write(config_path, lines.join("\n"))
            .with_context(|| format!("Failed to write to {}", config_path.display()))?;

        println!(
            "Version: {} -> {} in {}",
            old_version_str,
            new_version_str,
            config_path.display()
        );
        Ok(true)
    } else {
        Ok(false)
    }
}

fn change_version(change: VersionChange) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let clay_toml_path = current_dir.join("clay.toml");

    if clay_toml_path.exists() {
        let content = fs::read_to_string(&clay_toml_path)
            .with_context(|| format!("Failed to read {}", clay_toml_path.display()))?;

        let mut toml_value: Value = if content.trim().is_empty() {
            Value::Table(Table::new())
        } else {
            toml::from_str(&content)
                .with_context(|| format!("Failed to parse {}", clay_toml_path.display()))?
        };

        let root_table = toml_value
            .as_table_mut()
            .ok_or_else(|| anyhow!("clay.toml root is not a table"))?;

        if !root_table.contains_key("version") {
            let mut version_table = Table::new();
            version_table.insert("bump".to_string(), Value::Boolean(false));
            root_table.insert("version".to_string(), Value::Table(version_table));

            fs::write(&clay_toml_path, toml::to_string_pretty(&toml_value)?)
                .with_context(|| format!("Failed to write to {}", clay_toml_path.display()))?;
            println!(
                "Added `[version]` with `bump = false` to clay.toml. Skipping version change."
            );
            return Ok(());
        }

        let version_table = root_table
            .get_mut("version")
            .unwrap()
            .as_table_mut()
            .ok_or_else(|| anyhow!("[version] is not a table in clay.toml"))?;

        if let Some(bump_value) = version_table.get("bump") {
            if !bump_value.as_bool().unwrap_or(false) {
                // 如果 bump = false，则不执行任何操作
                println!("`bump` is false in clay.toml. Skipping version change.");
                return Ok(());
            }
            // 如果 bump = true，则继续执行下面的版本更新逻辑
        } else {
            // 如果 bump 字段不存在，则添加 bump = false 并退出
            version_table.insert("bump".to_string(), Value::Boolean(false));

            fs::write(&clay_toml_path, toml::to_string_pretty(&toml_value)?)
                .with_context(|| format!("Failed to write to {}", clay_toml_path.display()))?;
            println!(
                "Added `bump = false` to clay.toml under `[version]`. Skipping version change."
            );
            return Ok(());
        }
    }

    let project_type = detect_project_type(&current_dir);

    match project_type {
        ProjectType::Rust => {
            let config_path = current_dir.join("Cargo.toml");

            // Try to update version in the current directory's Cargo.toml
            let updated = update_cargo_toml_version(&config_path, &change)?;

            if !updated {
                // If no version found in root Cargo.toml, it might be a workspace
                // Search for Cargo.toml files in immediate subdirectories
                println!("No version found in root Cargo.toml, searching subdirectories...");

                let mut found_any = false;

                // Read all entries in the current directory
                if let Ok(entries) = fs::read_dir(&current_dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                let sub_cargo_path = entry.path().join("Cargo.toml");
                                if sub_cargo_path.exists() {
                                    // Try to update version in this subdirectory's Cargo.toml
                                    if update_cargo_toml_version(&sub_cargo_path, &change)? {
                                        found_any = true;
                                    }
                                }
                            }
                        }
                    }
                }

                if !found_any {
                    bail!(
                        "Could not find 'version' in any Cargo.toml files (root or subdirectories)"
                    )
                }

                Ok(())
            } else {
                Ok(())
            }
        }
        // Add logic for pnpm projects
        ProjectType::Pnpm => {
            let config_path = current_dir.join("package.json");
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {}", config_path.display()))?;

            let mut lines: Vec<String> = content.lines().map(String::from).collect();
            let mut version_line_index: Option<usize> = None;
            let mut old_version_str = String::new();
            let mut new_version_str = String::new();

            for (i, line) in lines.iter().enumerate() {
                let trimmed_line = line.trim();
                if trimmed_line.starts_with("\"version\":") {
                    if let Some(version_val) = trimmed_line.split(':').nth(1) {
                        // Trim whitespace, quotes, and trailing commas
                        let version_str = version_val.trim().trim_matches(|c| c == '"' || c == ',');
                        let mut version = Version::parse(version_str).with_context(|| {
                            format!("Failed to parse version: '{}'", version_str)
                        })?;

                        old_version_str = version.to_string();

                        match change {
                            VersionChange::Update => version.patch += 1,
                            VersionChange::Bump => {
                                version.minor += 1;
                                version.patch = 0;
                                version.pre = semver::Prerelease::EMPTY;
                                version.build = semver::BuildMetadata::EMPTY;
                            }
                        }
                        new_version_str = version.to_string();
                        version_line_index = Some(i);
                        break;
                    }
                }
            }

            if let Some(index) = version_line_index {
                // Replace the version string directly to preserve the line's original formatting
                lines[index] = lines[index].replace(&old_version_str, &new_version_str);

                fs::write(&config_path, lines.join("\n"))
                    .with_context(|| format!("Failed to write to {}", config_path.display()))?;

                println!("Version: {} -> {}", old_version_str, new_version_str);
                Ok(())
            } else {
                bail!("Could not find 'version' key in package.json")
            }
        }
        ProjectType::Unknown => {
            bail!("No supported project type found in the current directory.")
        }
    }
}
