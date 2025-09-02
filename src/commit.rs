/* src/commit.rs */

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::process::{Command, Stdio};

#[derive(Deserialize, Debug)]
struct AiCommitResponse {
    commits: Vec<FileCommit>,
}

#[derive(Deserialize, Debug)]
struct FileCommit {
    file: String,
    message: String,
}

/// Runs the full AI-powered commit and version bump process.
pub fn run_ai_commit() -> Result<()> {
    println!("- Step 1: Generating AI commit messages...");
    let llm_output = Command::new(std::env::current_exe()?)
        .arg("llm")
        .arg("commit")
        .output()
        .context("Failed to run 'clay llm commit'")?;

    if !llm_output.status.success() {
        let stderr = String::from_utf8_lossy(&llm_output.stderr);
        bail!("'clay llm commit' failed:\n{}", stderr);
    }

    let llm_json_str = String::from_utf8_lossy(&llm_output.stdout);
    if let Ok(commit_data) = serde_json::from_str::<AiCommitResponse>(&llm_json_str) {
        if commit_data.commits.is_empty() {
            println!(
                "  - No changes detected by LLM. Checking for other changes before versioning..."
            );
        } else {
            println!("- Step 2: Committing changes based on AI suggestions...");
            for commit in commit_data.commits {
                println!("  - Committing '{}': {}", commit.file, commit.message);

                // git add <file>
                let add_status = Command::new("git")
                    .arg("add")
                    .arg(&commit.file)
                    .status()
                    .with_context(|| format!("Failed to execute 'git add {}'", commit.file))?;
                if !add_status.success() {
                    bail!("'git add {}' failed.", commit.file);
                }

                // git commit -m <message>
                Command::new("git")
                    .arg("commit")
                    .arg("-m")
                    .arg(&commit.message)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .ok();
            }
        }
    } else {
        println!("  - Could not parse LLM response, skipping individual commits.");
    }

    println!("- Step 3: Bumping project version...");
    let version_output = Command::new(std::env::current_exe()?)
        .arg("project")
        .arg("update")
        .output()
        .context("Failed to run 'clay project update'")?;

    if !version_output.status.success() {
        let stderr = String::from_utf8_lossy(&version_output.stderr);
        bail!("'clay project update' failed:\n{}", stderr);
    }

    let version_update_str = String::from_utf8_lossy(&version_output.stdout);
    let versions: Vec<&str> = version_update_str.split_whitespace().collect();
    let (old_version, new_version) =
        if versions.len() == 4 && versions[0] == "Version:" && versions[2] == "->" {
            (versions[1], versions[3])
        } else {
            ("version", "new_version")
        };

    println!("- Step 4: Creating final version commit...");
    Command::new("git")
        .arg("add")
        .arg(".")
        .status()
        .context("Failed to stage final changes")?;

    let final_commit_message = format!("chore: update {} -> {}", old_version, new_version);
    let final_commit_status = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(&final_commit_message)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if final_commit_status.success() {
        println!("  - {}", final_commit_message);
    } else {
        println!("  - No remaining changes to commit for version update.");
    }

    println!("AI commit process finished successfully.");
    Ok(())
}

/// Runs the AI commit process and then pushes to the remote.
pub fn run_ai_push() -> Result<()> {
    run_ai_commit()?;

    println!("- Step 5: Pushing to remote...");
    let push_status = Command::new("git")
        .arg("push")
        .status()
        .context("Failed to execute 'git push'")?;

    if !push_status.success() {
        bail!("'git push' failed.");
    }

    println!("Push successful.");
    Ok(())
}
