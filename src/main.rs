/* src/main.rs */

mod actions;
mod app;
mod config;
mod diff;
mod history;
mod lint;
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
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Increment patch version (e.g., 1.0.0 -> 1.0.1)
    Update,
    /// Increment minor version (e.g., 1.0.1 -> 1.1.0)
    Bump,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Lint) => {
            lint::run_linter()?;
        }
        Some(Commands::Diff) => {
            diff::run_diff()?;
        }
        Some(Commands::Project(project_cmd)) => match project_cmd {
            ProjectCommands::Update => version::version_update()?,
            ProjectCommands::Bump => version::version_bump()?,
        },
        None => {
            tui::run_tui()?;
        }
    }

    Ok(())
}
