/* src/llm.rs */

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Content,
}

#[derive(Deserialize, Debug)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Deserialize, Debug)]
struct Part {
    text: String,
}

fn get_token_path() -> Result<PathBuf> {
    let base_dirs = directories::BaseDirs::new().context("Could not find home directory")?;
    Ok(base_dirs.home_dir().join(".clay/token"))
}

pub fn set_token() -> Result<()> {
    print!("Please enter your Gemini API Token: ");
    io::stdout().flush()?;

    let mut token = String::new();
    io::stdin().read_line(&mut token)?;
    let token = token.trim();

    if token.is_empty() {
        bail!("Token cannot be empty.");
    }

    let token_path = get_token_path()?;
    if let Some(parent) = token_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&token_path, token)?;
    println!("Token saved successfully to {}", token_path.display());
    Ok(())
}

fn get_token() -> Result<String> {
    let token_path = get_token_path()?;
    fs::read_to_string(token_path)
        .context("Failed to read token. Please run 'clay llm token' to set it.")
}

pub fn generate_commit_messages() -> Result<()> {
    let token = get_token()?;

    // 1. Capture the output of `clay diff`
    let diff_output = Command::new(std::env::current_exe()?)
        .arg("diff")
        .output()
        .context("Failed to execute 'clay diff' command")?;

    if !diff_output.status.success() {
        let stderr = String::from_utf8_lossy(&diff_output.stderr);
        bail!("'clay diff' command failed: {}", stderr);
    }

    let diff_json = String::from_utf8_lossy(&diff_output.stdout);
    if diff_json.trim() == "[]" {
        println!("No git changes detected to generate commit messages for.");
        return Ok(());
    }

    // 2. Prepare the prompt and API call
    let prompt = format!(
        r#"You are a GIT helper API. Your task is to generate a concise, one-sentence commit message summary for each file in the provided JSON diff. Follow the Conventional Commits specification (Angular convention).

Each message must start with one of the following types in lowercase: [feat:, fix:, chore:, refactor:, docs:, style:, test:, perf:, ci:]. While the type must be lowercase, you can use capitalization for proper nouns within the message itself.

Provide your response as a JSON object in the following format. Your entire output should only be the JSON text, without any markdown code blocks.

{{
  "commits": [
    {{
      "file": "path/to/file1.type",
      "message": "type: your commit message here"
    }},
    {{
      "file": "path/to/file2.type",
      "message": "type: your other commit message here"
    }}
  ]
}}

Here is the git diff JSON:
{}"#,
        diff_json
    );

    let client = reqwest::blocking::Client::new();
    let api_url =
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent";

    let request_body = serde_json::json!({
        "contents": [{
            "parts": [{ "text": prompt }]
        }]
    });

    println!("Sending request to Gemini API...");

    let res = client
        .post(api_url)
        .header("Content-Type", "application/json")
        .header("X-goog-api-key", token.trim())
        .json(&request_body)
        .send()
        .context("Failed to send request to Gemini API")?;

    if !res.status().is_success() {
        let status = res.status();
        let error_body = res
            .text()
            .unwrap_or_else(|_| "Could not read error body".to_string());
        bail!(
            "Gemini API request failed with status: {}\nBody: {}",
            status,
            error_body
        );
    }

    let response_body: GeminiResponse =
        res.json().context("Failed to parse Gemini API response")?;

    // 3. Extract, validate, and print the response
    if let Some(candidate) = response_body.candidates.get(0) {
        if let Some(part) = candidate.content.parts.get(0) {
            let llm_text = &part.text;

            // Find the start and end of the JSON object
            if let (Some(start), Some(end)) = (llm_text.find('{'), llm_text.rfind('}')) {
                let json_str = &llm_text[start..=end];
                match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(json_val) => {
                        println!("{}", serde_json::to_string_pretty(&json_val)?);
                    }
                    Err(e) => {
                        bail!(
                            "Failed to parse JSON extracted from LLM response: {}\nExtracted text:\n{}",
                            e,
                            json_str
                        );
                    }
                }
            } else {
                bail!("LLM returned a non-JSON response:\n{}", llm_text);
            }
        }
    } else {
        bail!("LLM response did not contain any candidates.");
    }

    Ok(())
}
