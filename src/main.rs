/* src/main.rs */

mod actions;
mod app;
mod config;
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
    /// Lint and format rust files with header comments
    Lint,
    /// Manage project versioning
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
}

#[derive(Subcommand)]
enum ProjectCommands {
    /// Increment the patch version (e.g., 0.1.0 -> 0.1.1)
    Update,
    /// Increment the minor version and reset patch (e.g., 0.1.0 -> 0.2.0)
    Bump,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Lint) => {
            lint::run_linter()?;
        }
        Some(Commands::Project { command }) => match command {
            ProjectCommands::Update => version::version_update()?,
            ProjectCommands::Bump => version::version_bump()?,
        },
        None => {
            tui::run_tui()?;
        }
    }

    Ok(())
}
