/* src/commit.rs */

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::io;
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
    println!("Starting LLM diff...");
    let llm_output = Command::new(std::env::current_exe()?)
        .arg("llm")
        .arg("commit")
        .output()
        .context("Failed to run 'clay llm commit'")?;

    if !llm_output.status.success() {
        let stderr = String::from_utf8_lossy(&llm_output.stderr);
        bail!("'clay llm commit' failed:\n{}", stderr);
    }

    let llm_output_str = String::from_utf8_lossy(&llm_output.stdout);

    println!("Analysing commit messages...");
    let commit_data_result: Result<AiCommitResponse, _> =
        if let (Some(start), Some(end)) = (llm_output_str.find('{'), llm_output_str.rfind('}')) {
            let json_str = &llm_output_str[start..=end];
            serde_json::from_str(json_str)
        } else {
            Err(serde_json::Error::io(io::Error::new(
                io::ErrorKind::InvalidData,
                "Could not find JSON object in LLM response",
            )))
        };

    if let Ok(commit_data) = commit_data_result {
        if !commit_data.commits.is_empty() {
            println!("Calling Git toolchain...");
            for commit in commit_data.commits {
                println!("  {} -> {}", commit.file, commit.message);

                let add_status = Command::new("git")
                    .arg("add")
                    .arg(&commit.file)
                    .status()
                    .with_context(|| format!("Failed to execute 'git add {}'", commit.file))?;
                if !add_status.success() {
                    bail!("'git add {}' failed.", commit.file);
                }

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
    }

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
    println!("Bumping version {} -> {}", old_version, new_version);

    println!("Creating version commit...");
    Command::new("git")
        .arg("add")
        .arg(".")
        .status()
        .context("Failed to stage final changes")?;

    let final_commit_message = format!("chore: update {} -> {}", old_version, new_version);
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(&final_commit_message)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    Ok(())
}

/// Runs the AI commit process and then pushes to the remote.
pub fn run_ai_push() -> Result<()> {
    run_ai_commit()?;

    println!("Pushing changes to remote...");
    let push_status = Command::new("git")
        .arg("push")
        .status()
        .context("Failed to execute 'git push'")?;

    if !push_status.success() {
        bail!("'git push' failed.");
    }

    Ok(())
}
