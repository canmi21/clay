/* src/version.rs */

use anyhow::{Context, Result, bail};
use semver::Version;
use std::fs;
use std::path::Path;

enum ProjectType {
    Rust,
    Unknown,
}

fn detect_project_type(base_path: &Path) -> ProjectType {
    if base_path.join("Cargo.toml").exists() {
        ProjectType::Rust
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
    Update, // c + 1
    Bump,   // b + 1, c = 0
}

fn change_version(change: VersionChange) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let project_type = detect_project_type(&current_dir);

    match project_type {
        ProjectType::Rust => {
            let config_path = current_dir.join("Cargo.toml");
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {}", config_path.display()))?;

            let mut lines: Vec<String> = content.lines().map(String::from).collect();
            let mut in_package_section = false;
            let mut version_line_index: Option<usize> = None;
            let mut old_version_str = String::new();
            let mut new_version_str = String::new();

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
                        let mut version = Version::parse(version_str).with_context(|| {
                            format!("Failed to parse version string '{}'", version_str)
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
                let original_line = &lines[index];
                let leading_whitespace = original_line
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect::<String>();
                lines[index] = format!("{}version = \"{}\"", leading_whitespace, new_version_str);

                let new_content = lines.join("\n");
                fs::write(&config_path, new_content)
                    .with_context(|| format!("Failed to write to {}", config_path.display()))?;

                println!("Version: {} -> {}", old_version_str, new_version_str);
                Ok(())
            } else {
                bail!("Could not find 'version' key in [package] section of Cargo.toml")
            }
        }
        ProjectType::Unknown => {
            bail!("No supported project type found in the current directory.")
        }
    }
}
