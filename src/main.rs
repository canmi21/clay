/* src/main.rs */

mod app;
mod project;
mod shell;
mod terminal;
mod ui;

use crate::app::{App, BottomBarMode, ScriptEndStatus};
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

const CMD_FINISHED_MARKER: &str = "CLAY_CMD_FINISHED_MARKER_v1";

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
    if app.config.is_some() {
        app.logs
            .push("Rust project detected. Config loaded.".to_string());
    } else {
        app.logs.push("No project type detected.".to_string());
    }
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
            let mut output = String::from_utf8_lossy(&bytes).to_string();
            let mut script_finished = false;

            if output.contains(CMD_FINISHED_MARKER) {
                script_finished = true;
                output = output.replace(CMD_FINISHED_MARKER, "");
                output = output.trim_end().to_string();
            }

            if !output.is_empty() {
                app.terminal.process_bytes(output.as_bytes());
            }

            if script_finished {
                app.finish_script(ScriptEndStatus::Finished);
            }
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
                    BottomBarMode::Input => handle_input_mode_keys(key, app, shell_process)?,
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

fn handle_input_mode_keys(
    key: event::KeyEvent,
    app: &mut App,
    shell: &mut ShellProcess,
) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            let packages_to_add = app.command_input.trim().to_string();
            app.command_input.clear();
            app.command_cursor_position = 0;

            if !packages_to_add.is_empty() {
                let base_cmd_to_run = if let Some(config) = &app.config {
                    config.scripts.get("add").cloned()
                } else {
                    None
                };

                if let Some(base_cmd) = base_cmd_to_run {
                    let full_cmd = format!("{} {}", base_cmd, packages_to_add);
                    run_shell_command(app, shell, "add", &full_cmd, "Adding dependencies")?;
                }
            } else {
                app.bottom_bar_mode = BottomBarMode::Tips;
            }
        }
        KeyCode::Char(c) => app.enter_char(c),
        KeyCode::Backspace => app.delete_char(),
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Esc => {
            app.command_input.clear();
            app.command_cursor_position = 0;
            app.bottom_bar_mode = BottomBarMode::Tips;
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
            KeyCode::Char('a') => {
                app.bottom_bar_mode = BottomBarMode::Input;
                app.command_input.clear();
                app.command_cursor_position = 0;
            }
            KeyCode::Char('r') => execute_script(app, shell, "dev", "Running")?,
            KeyCode::Char('b') => execute_script(app, shell, "build", "Building")?,
            KeyCode::Char('l') => execute_script(app, shell, "lint", "Formatting")?,
            KeyCode::Char('p') => execute_script(app, shell, "publish", "Uploading")?,
            KeyCode::Char('i') => execute_script(app, shell, "install", "Installing")?,
            KeyCode::Char('q') => execute_script(app, shell, "clean", "Cleaning")?,
            KeyCode::Char('c') => {
                shell.write_to_shell(b"\x03")?;
            }
            _ => {}
        }
    } else if key.code == KeyCode::Char('c') {
        shell.write_to_shell(b"\x03")?;
        app.finish_script(ScriptEndStatus::Cancelled);
    }
    Ok(())
}

fn run_shell_command(
    app: &mut App,
    shell: &mut ShellProcess,
    script_name: &str,
    command: &str,
    status: &str,
) -> Result<()> {
    app.terminal.clear();
    let full_command_with_marker = format!("{}\necho {}\n", command, CMD_FINISHED_MARKER);
    shell.write_to_shell(full_command_with_marker.as_bytes())?;
    let message = format!("{} (Press 'c' to cancel)...", status);
    app.start_script(script_name, &message);
    Ok(())
}

fn execute_script(
    app: &mut App,
    shell: &mut ShellProcess,
    script_name: &str,
    status: &str,
) -> Result<()> {
    let command_to_run = if let Some(config) = &app.config {
        config.scripts.get(script_name).cloned()
    } else {
        None
    };

    if let Some(command) = command_to_run {
        run_shell_command(app, shell, script_name, &command, status)?;
    }
    Ok(())
}
