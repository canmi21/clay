/* src/main.rs */

mod actions;
mod app;
mod commit;
mod config;
mod diff;
mod history;
mod lint;
mod llm;
mod project;
mod shell;
mod terminal;
mod tui;
mod ui;
mod version;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint and format project files
    Lint,
    /// Show the git diff as JSON
    Diff,
    /// Manage project versioning
    #[command(subcommand)]
    Project(ProjectCommands),
    /// Interact with LLM for commit message generation
    #[command(subcommand)]
    Llm(LlmCommands),
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Increment patch version (e.g., 1.0.0 -> 1.0.1)
    Update,
    /// Increment minor version (e.g., 1.0.1 -> 1.1.0)
    Bump,
}

#[derive(Subcommand)]
pub enum LlmCommands {
    /// Set the Gemini API token
    Token,
    /// Generate commit messages based on git diff
    Commit,
    /// Generate and apply AI commits, then bump version
    Git,
    /// Run the AI commit process and push to remote
    Push,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Lint) => lint::run_linter()?,
        Some(Commands::Diff) => diff::run_diff()?,
        Some(Commands::Project(project_cmd)) => match project_cmd {
            ProjectCommands::Update => version::version_update()?,
            ProjectCommands::Bump => version::version_bump()?,
        },
        Some(Commands::Llm(llm_cmd)) => match llm_cmd {
            LlmCommands::Token => llm::set_token()?,
            LlmCommands::Commit => llm::generate_commit_messages()?,
            LlmCommands::Git => commit::run_ai_commit()?,
            LlmCommands::Push => commit::run_ai_push()?,
        },
        None => {
            tui::run_tui()?;
        }
    }

    Ok(())
}
