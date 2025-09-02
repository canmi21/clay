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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Lint) => {
            lint::run_linter()?;
        }
        None => {
            tui::run_tui()?;
        }
    }

    Ok(())
}
