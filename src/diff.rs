/* src/diff.rs */

use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::fs;
use std::process::Command;

#[derive(Serialize, Debug)]
struct CompactFileDiff {
    file: String,
    additions: Vec<String>,
    deletions: Vec<String>,
}

pub fn run_diff() -> Result<()> {
    // 1. Execute git diff for tracked files
    let diff_output_result = Command::new("git")
        .args(["diff", "--unified=0"])
        .output()
        .context("Failed to execute 'git diff'")?;

    if !diff_output_result.status.success() {
        let stderr = String::from_utf8_lossy(&diff_output_result.stderr);
        bail!("'git diff' command failed: {}", stderr);
    }

    let diff_output = String::from_utf8_lossy(&diff_output_result.stdout);
    let mut parsed_diffs = parse_diff_to_compact_format(&diff_output);

    // 2. Find and process untracked files
    let untracked_output_result = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .output()
        .context("Failed to execute 'git ls-files'")?;

    if untracked_output_result.status.success() {
        let untracked_output = String::from_utf8_lossy(&untracked_output_result.stdout);
        for file_path in untracked_output.lines() {
            if let Ok(content) = fs::read_to_string(file_path) {
                let additions = content.lines().map(String::from).collect();
                let untracked_diff = CompactFileDiff {
                    file: file_path.to_string(),
                    additions,
                    deletions: vec!["// New untracked file".to_string()],
                };
                parsed_diffs.push(untracked_diff);
            }
        }
    }

    // 3. Serialize to JSON and print
    if !parsed_diffs.is_empty() {
        let json_output = serde_json::to_string_pretty(&parsed_diffs)?;
        println!("{}", json_output);
    }

    Ok(())
}

/// Applies truncation rules and pushes a finalized diff to the results vector.
fn finalize_and_push_diff(diff_option: Option<CompactFileDiff>, diffs: &mut Vec<CompactFileDiff>) {
    if let Some(mut diff) = diff_option {
        if diff.additions.is_empty() && diff.deletions.is_empty() {
            return;
        }

        // Rule 1: Truncate lock files if they exceed 10 lines.
        if diff.file.ends_with(".lock") || diff.file.contains("lock") {
            if (diff.additions.len() + diff.deletions.len()) > 10 {
                diff.additions.truncate(5);
                diff.deletions.truncate(5);
                diff.additions
                    .push("... (truncated lock file diff)".to_string());
            }
        }
        // Rule 2: General truncation for any file exceeding 300 lines.
        else if (diff.additions.len() + diff.deletions.len()) > 300 {
            diff.additions.truncate(150);
            diff.deletions.truncate(150);
            diff.additions
                .push("... (truncated large diff)".to_string());
        }

        diffs.push(diff);
    }
}

fn parse_diff_to_compact_format(output: &str) -> Vec<CompactFileDiff> {
    let mut diffs = Vec::new();
    let mut current_file_diff: Option<CompactFileDiff> = None;

    for line in output.lines() {
        if line.starts_with("diff --git") {
            // A new file section has started. Finalize and save the previous one.
            finalize_and_push_diff(current_file_diff.take(), &mut diffs);

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
            if let Some(ref mut diff) = current_file_diff {
                if !line.starts_with("+++") {
                    diff.additions.push(stripped.to_string());
                }
            }
        } else if let Some(stripped) = line.strip_prefix('-') {
            if let Some(ref mut diff) = current_file_diff {
                if !line.starts_with("---") {
                    diff.deletions.push(stripped.to_string());
                }
            }
        }
    }

    // Finalize and add the last file diff if it exists
    finalize_and_push_diff(current_file_diff.take(), &mut diffs);

    diffs
}
