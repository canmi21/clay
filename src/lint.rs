/* src/lint.rs */

use crate::project;
use anyhow::Result;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

pub fn run_linter() -> Result<()> {
    let base_path = std::env::current_dir()?;
    println!("Starting linter in: {}", base_path.display());

    // 1. Run user-defined lint command from clay-config.json
    if let Some(project_config) = project::load_config()? {
        if let Some(lint_command) = project_config.scripts.get("lint") {
            println!("- Running user-defined lint command: '{}'...", lint_command);
            let mut parts = lint_command.split_whitespace();
            let program = parts.next().unwrap_or("");
            let args: Vec<&str> = parts.collect();

            if !program.is_empty() {
                let fmt_status = Command::new(program).args(args).status()?;
                if !fmt_status.success() {
                    println!("  User lint command failed. Aborting header updates.");
                    return Ok(());
                }
                println!("  User lint command completed successfully.");
            }
        }
    }

    // 2. Walk directory and update file headers
    println!("- Checking and updating file headers...");
    for entry in WalkDir::new(&base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let file_path = entry.path();
        if let Ok(relative_path) = file_path.strip_prefix(&base_path) {
            update_file_header(file_path, relative_path)?;
        }
    }
    println!("Linting process finished.");
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
