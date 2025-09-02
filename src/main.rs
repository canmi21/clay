/* src/main.rs */

mod app;
mod lint;
mod project;
mod shell;
mod terminal;
mod tui;
mod ui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "clay",
    version = "1.0",
    about = "A TUI-based project assistant"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser)]
enum Commands {
    /// Formats the project files with the clay linter
    Lint,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Lint) => lint::run_linter(&std::env::current_dir()?),
        None => tui::run_tui(),
    }
}
