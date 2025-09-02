/* src/diff.rs */

use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::process::Command;

#[derive(Serialize, Debug)]
struct CompactFileDiff {
    file: String,
    additions: Vec<String>,
    deletions: Vec<String>,
}

pub fn run_diff() -> Result<()> {
    // 1. Execute git diff
    let output = Command::new("git")
        .args(["diff", "--unified=0"])
        .output()
        .context("Failed to execute 'git diff'")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("'git diff' command failed: {}", stderr);
    }

    let diff_output = String::from_utf8_lossy(&output.stdout);

    // 2. Parse the output into the compact format
    let parsed_diffs = parse_diff_to_compact_format(&diff_output);

    // 3. Serialize to JSON and print
    if !parsed_diffs.is_empty() {
        let json_output = serde_json::to_string_pretty(&parsed_diffs)?;
        println!("{}", json_output);
    }

    Ok(())
}

fn parse_diff_to_compact_format(output: &str) -> Vec<CompactFileDiff> {
    let mut diffs = Vec::new();
    let mut current_file_diff: Option<CompactFileDiff> = None;

    for line in output.lines() {
        if line.starts_with("diff --git") {
            // A new file section has started. Save the previous one if it exists.
            if let Some(diff) = current_file_diff.take() {
                // Only add if there are actual changes
                if !diff.additions.is_empty() || !diff.deletions.is_empty() {
                    diffs.push(diff);
                }
            }

            // Start a new FileDiff
            if let Some(file_path) = line.split_whitespace().nth(2) {
                let clean_path = file_path
                    .strip_prefix("a/")
                    .unwrap_or(file_path)
                    .to_string();
                current_file_diff = Some(CompactFileDiff {
                    file: clean_path,
                    additions: Vec::new(),
                    deletions: Vec::new(),
                });
            }
        } else if let Some(stripped) = line.strip_prefix('+') {
            // Additions
            if let Some(ref mut diff) = current_file_diff {
                // Ignore the `+++ b/file.rs` line
                if !line.starts_with("+++") {
                    diff.additions.push(stripped.to_string());
                }
            }
        } else if let Some(stripped) = line.strip_prefix('-') {
            // Deletions
            if let Some(ref mut diff) = current_file_diff {
                // Ignore the `--- a/file.rs` line
                if !line.starts_with("---") {
                    diff.deletions.push(stripped.to_string());
                }
            }
        }
    }

    // Add the last file diff if it exists
    if let Some(diff) = current_file_diff.take() {
        if !diff.additions.is_empty() || !diff.deletions.is_empty() {
            diffs.push(diff);
        }
    }

    diffs
}
