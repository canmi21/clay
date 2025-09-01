/* src/main.rs */

mod app;
mod project;
mod shell;
mod terminal;
mod ui;

use crate::app::{App, BottomBarMode};
use crate::shell::ShellProcess;
use crate::ui::ui;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::{io, time::Duration};

fn main() -> Result<()> {
    let config = project::load_or_create_config()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let shell_pane_outer_height = size.height.saturating_sub(10);
    let shell_pane_inner_height = shell_pane_outer_height.saturating_sub(2);
    let shell_pane_inner_width = size.width.saturating_sub(2);

    let mut app = App::new(shell_pane_inner_width, shell_pane_inner_height, config);
    let mut shell_process = ShellProcess::new(shell_pane_inner_height, shell_pane_inner_width)?;

    run_app(&mut terminal, &mut app, &mut shell_process)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    shell_process: &mut ShellProcess,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Some(bytes) = shell_process.read_output_bytes() {
            app.terminal.process_bytes(&bytes);
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if app.bottom_bar_mode != BottomBarMode::Command {
                        app.should_quit = true;
                    }
                }

                match app.bottom_bar_mode {
                    BottomBarMode::Command => handle_command_mode_keys(key, app, shell_process)?,
                    _ => handle_normal_mode_keys(key, app, shell_process)?,
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_command_mode_keys(
    key: event::KeyEvent,
    app: &mut App,
    shell: &mut ShellProcess,
) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            app.terminal.clear();
            let command = format!("{}\n", app.command_input);
            shell.write_to_shell(command.as_bytes())?;
            app.submit_command();
        }
        KeyCode::Char(c) => {
            if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.command_input.clear();
                app.command_cursor_position = 0;
                app.bottom_bar_mode = BottomBarMode::Tips;
            } else {
                app.enter_char(c);
            }
        }
        KeyCode::Backspace => app.delete_char(),
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Esc => app.bottom_bar_mode = BottomBarMode::Tips,
        KeyCode::Up => {
            if !app.command_history.is_empty() {
                app.history_index = app.history_index.saturating_sub(1);
                app.command_input = app.command_history[app.history_index].clone();
                app.command_cursor_position = app.command_input.len();
            }
        }
        KeyCode::Down => {
            if !app.command_history.is_empty() && app.history_index < app.command_history.len() - 1
            {
                app.history_index += 1;
                app.command_input = app.command_history[app.history_index].clone();
                app.command_cursor_position = app.command_input.len();
            } else {
                app.history_index = app.command_history.len();
                app.command_input.clear();
                app.command_cursor_position = 0;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_normal_mode_keys(
    key: event::KeyEvent,
    app: &mut App,
    shell: &mut ShellProcess,
) -> Result<()> {
    if !app.is_script_running {
        match key.code {
            KeyCode::Esc => app.should_quit = true,
            KeyCode::Char('/') => {
                if app.bottom_bar_mode != BottomBarMode::Input {
                    app.bottom_bar_mode = BottomBarMode::Command;
                    app.history_index = app.command_history.len();
                }
            }
            KeyCode::Up => app.scroll_up(),
            KeyCode::Down => app.scroll_down(),
            KeyCode::Char('r') => execute_script(app, shell, "dev", "Running")?,
            KeyCode::Char('b') => execute_script(app, shell, "build", "Building")?,
            KeyCode::Char('l') => execute_script(app, shell, "lint", "Formatting")?,
            KeyCode::Char('p') => execute_script(app, shell, "publish", "Uploading")?,
            KeyCode::Char('i') => execute_script(app, shell, "install", "Installing")?,
            KeyCode::Char('c') => {
                shell.write_to_shell(b"\x03")?;
            }
            _ => {}
        }
    } else if key.code == KeyCode::Char('c') {
        shell.write_to_shell(b"\x03")?; // Send Ctrl+C
        app.finish_script();
    }
    Ok(())
}

fn execute_script(
    app: &mut App,
    shell: &mut ShellProcess,
    script_name: &str,
    status: &str,
) -> Result<()> {
    if let Some(config) = &app.config {
        if let Some(command) = config.scripts.get(script_name) {
            app.terminal.clear();
            shell.write_to_shell(format!("{}\n", command).as_bytes())?;
            let message = format!("{} (Press 'c' to cancel)...", status);
            app.start_script(status, &message);
        }
    }
    Ok(())
}
