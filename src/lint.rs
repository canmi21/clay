/* src/lint.rs */

use crate::project;
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

/// Runs the full linting process.
pub fn run_linter(base_path: &Path) -> Result<()> {
    // Execute the user-defined lint command from clay-config.json
    println!("Executing user-defined lint command...");
    if let Ok(Some(config)) = project::load_config() {
        if let Some(lint_command) = config.scripts.get("lint") {
            println!("> {}", lint_command);
            let mut parts = lint_command.split_whitespace();
            if let Some(program) = parts.next() {
                let status = Command::new(program)
                    .args(parts)
                    .current_dir(base_path)
                    .status()?;
                if !status.success() {
                    eprintln!(
                        "Warning: User-defined lint command failed with status: {}",
                        status
                    );
                }
            }
        } else {
            println!("No 'lint' script found in clay-config.json.");
        }
    } else {
        println!(
            "Could not load clay-config.json or it's invalid, skipping user-defined lint command."
        );
    }

    // Run the custom file header linter
    println!("\nRunning custom file header linter...");
    for entry in WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.path().to_string_lossy().contains("target/"))
    {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(relative_path) = path.strip_prefix(base_path) {
                process_file(path, relative_path)?;
            }
        }
    }
    println!("Linting complete.");
    Ok(())
}

/// Processes a single file to ensure it has the correct header comment and a blank line.
fn process_file(file_path: &Path, relative_path: &Path) -> Result<()> {
    let content = fs::read_to_string(file_path)?;
    let mut new_lines: Vec<String> = content.lines().map(String::from).collect();

    // Use forward slashes for consistency, even on Windows
    let expected_comment = format!(
        "/* {} */",
        relative_path.display().to_string().replace('\\', "/")
    );

    let mut needs_write = false;

    // Check and fix the header comment on the first line
    if new_lines.is_empty() || new_lines[0] != expected_comment {
        if !new_lines.is_empty() && new_lines[0].starts_with("/*") && new_lines[0].ends_with("*/") {
            // It looks like a header comment, but the content is wrong. Replace it.
            new_lines[0] = expected_comment;
        } else {
            // No header comment found, insert it at the beginning.
            new_lines.insert(0, expected_comment);
        }
        needs_write = true;
    }

    // Check for a blank line after the header comment
    let needs_blank_line = match new_lines.get(1) {
        None => true, // File only had a header (or was empty), so it needs a blank line.
        Some(line) => {
            // It needs a blank line if it's not empty and not a comment.
            !line.trim().is_empty()
                && !line.trim().starts_with("//")
                && !line.trim().starts_with("/*")
        }
    };

    if needs_blank_line {
        new_lines.insert(1, String::new());
        needs_write = true;
    }

    if needs_write {
        println!("  - Updating header for: {}", file_path.display());
        let new_content = new_lines.join("\n");
        // Always end the file with a newline for POSIX compliance
        fs::write(file_path, new_content + "\n")?;
    }

    Ok(())
}
