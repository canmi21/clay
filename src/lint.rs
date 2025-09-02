/* src/lint.rs */

use crate::project;
use anyhow::{Context, Result};
use semver::Version;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

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

pub fn run_linter() -> Result<()> {
    let base_path = std::env::current_dir()?;
    println!("Starting linter in: {}", base_path.display());

    // Step 1: Run user-defined lint command
    run_user_defined_lint(&base_path)?;

    // Step 2 & 3: Run project-specific linters
    let project_type = detect_project_type(&base_path);
    match project_type {
        ProjectType::Rust => run_rust_linter(&base_path)?,
        ProjectType::Unknown => {
            println!("- No project-specific linter found for this project type.");
        }
    }

    println!("Linting process finished.");
    Ok(())
}

fn run_user_defined_lint(_base_path: &Path) -> Result<()> {
    if let Some(config) = project::load_config()? {
        if let Some(lint_command) = config.scripts.get("lint") {
            println!("- Running user-defined lint command: '{}'...", lint_command);
            let mut parts = lint_command.split_whitespace();
            let program = parts.next().unwrap_or("");
            let args: Vec<&str> = parts.collect();

            if !program.is_empty() {
                let fmt_status = Command::new(program).args(args).status()?;
                if !fmt_status.success() {
                    println!("  '{}' failed. Aborting further steps.", lint_command);
                    return Ok(());
                }
                println!("  '{}' completed successfully.", lint_command);
            }
        }
    } else {
        println!("- No clay-config.json found, skipping user-defined lint step.");
    }
    Ok(())
}

fn run_rust_linter(base_path: &Path) -> Result<()> {
    println!("- Running Rust-specific linter...");
    // 2a. Update file headers
    check_rust_headers(base_path)?;
    // 2b. Check dependencies
    check_rust_dependencies(base_path)?;
    Ok(())
}

fn check_rust_headers(base_path: &Path) -> Result<()> {
    println!("- Checking and updating file headers...");
    for entry in WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let file_path = entry.path();
        if let Ok(relative_path) = file_path.strip_prefix(base_path) {
            update_file_header(file_path, relative_path)?;
        }
    }
    Ok(())
}

fn update_file_header(file_path: &Path, relative_path: &Path) -> Result<()> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    let header_comment = format!("/* {} */", relative_path.display());
    let mut needs_update = false;

    if lines.is_empty() {
        lines.insert(0, header_comment);
        lines.insert(1, String::new());
        needs_update = true;
    } else {
        let first_line = &lines[0];
        if first_line != &header_comment {
            if first_line.trim().starts_with("/*") || first_line.trim().starts_with("//") {
                lines.remove(0);
            }
            lines.insert(0, header_comment);
            needs_update = true;
        }

        if lines.len() == 1 {
            lines.push(String::new());
            needs_update = true;
        } else {
            let second_line = lines[1].trim();
            if !second_line.is_empty()
                && !second_line.starts_with("//")
                && !second_line.starts_with("/*")
            {
                lines.insert(1, String::new());
                needs_update = true;
            }
        }
    }

    if needs_update {
        println!("  - Updating header for: {}", relative_path.display());
        let mut file = fs::File::create(file_path)?;
        for line in lines {
            writeln!(file, "{}", line)?;
        }
    }
    Ok(())
}

fn check_rust_dependencies(base_path: &Path) -> Result<()> {
    println!("- Checking and updating Cargo.toml dependencies...");
    let config_path = base_path.join("Cargo.toml");
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;

    let mut new_lines = Vec::new();
    let mut in_dependencies_section = false;
    let mut modified = false;

    for line in content.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.starts_with('[') && trimmed_line.contains("dependencies") {
            in_dependencies_section = true;
            new_lines.push(line.to_string());
            continue;
        } else if trimmed_line.starts_with('[') {
            in_dependencies_section = false;
        }

        if in_dependencies_section {
            if let Some((key, val)) = trimmed_line.split_once('=') {
                let val_trimmed = val.trim();

                // Handle complex dependencies like { version = "x.y.z", ... }
                if val_trimmed.starts_with('{') {
                    if let Some(version_part) = val_trimmed.split("version =").nth(1) {
                        if let Some(version_str) = version_part.split('"').nth(1) {
                            if let Ok(new_line) =
                                get_updated_dependency_line(line, version_str, key)
                            {
                                if line != new_line {
                                    modified = true;
                                }
                                new_lines.push(new_line);
                                continue;
                            }
                        }
                    }
                }
                // Handle simple dependencies like "x.y.z"
                else if val_trimmed.starts_with('"') {
                    let version_str = val_trimmed.trim_matches('"');
                    if let Ok(new_line) = get_updated_dependency_line(line, version_str, key) {
                        if line != new_line {
                            modified = true;
                        }
                        new_lines.push(new_line);
                        continue;
                    }
                }
            }
        }
        new_lines.push(line.to_string());
    }

    if modified {
        println!("  - Updating dependency versions in Cargo.toml.");
        fs::write(config_path, new_lines.join("\n"))?;
    } else {
        println!("  - All dependency versions are already compliant.");
    }
    Ok(())
}

fn get_updated_dependency_line(
    original_line: &str,
    version_str: &str,
    key: &str,
) -> Result<String> {
    if let Ok(version) = Version::parse(version_str) {
        let simplified_version = if version.major != 0 {
            version.major.to_string()
        } else if version.minor != 0 {
            format!("0.{}", version.minor)
        } else {
            format!("0.0.{}", version.patch)
        };

        if version_str != simplified_version {
            println!(
                "    - Linting {} version: {} -> {}",
                key.trim(),
                version_str,
                simplified_version
            );
            return Ok(original_line.replace(version_str, &simplified_version));
        }
    }
    Ok(original_line.to_string())
}
