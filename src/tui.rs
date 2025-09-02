/* src/tui.rs */

use crate::app::{App, BottomBarMode, InputContext, ScriptEndStatus};
use crate::project;
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

/// Initializes and runs the terminal user interface.
pub fn run_tui() -> Result<()> {
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
        app.logs.push("Rust detected. Config loaded.".to_string());
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

/// The main application loop.
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

                if app.show_help {
                    // When help is shown, only specific keys close it
                    if (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                        || key.code == KeyCode::Esc
                        || key.code == KeyCode::Char('h')
                    {
                        app.show_help = false;
                    }
                } else if key.code == KeyCode::Char('c')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    // Global quit hotkey, only active when help is not shown
                    app.should_quit = true;
                } else {
                    // Regular key handling when help is not shown
                    match app.bottom_bar_mode {
                        BottomBarMode::Command => {
                            handle_command_mode_keys(key, app, shell_process)?
                        }
                        BottomBarMode::Input => handle_input_mode_keys(key, app, shell_process)?,
                        _ => handle_normal_mode_keys(key, app, shell_process)?,
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// Handles key events when in command mode.
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
            app.enter_char(c);
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

/// Handles key events when in input mode.
fn handle_input_mode_keys(
    key: event::KeyEvent,
    app: &mut App,
    shell: &mut ShellProcess,
) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            let user_input = app.command_input.trim().to_string();
            let context = app.input_context.take();

            app.command_input.clear();
            app.command_cursor_position = 0;
            app.bottom_bar_mode = BottomBarMode::Tips;

            if user_input.is_empty() {
                return Ok(());
            }

            if let Some(context) = context {
                match context {
                    InputContext::AddPackage => {
                        let command_to_run = app
                            .config
                            .as_ref()
                            .and_then(|c| c.scripts.get("add").cloned())
                            .map(|base_cmd| format!("{} {}", base_cmd, user_input));

                        if let Some(command) = command_to_run {
                            run_shell_command(app, shell, "add", &command, "Adding dependencies")?;
                        }
                    }
                    InputContext::RemovePackage => {
                        let command_to_run = app
                            .config
                            .as_ref()
                            .and_then(|c| c.scripts.get("remove").cloned())
                            .map(|base_cmd| format!("{} {}", base_cmd, user_input));

                        if let Some(command) = command_to_run {
                            run_shell_command(
                                app,
                                shell,
                                "remove",
                                &command,
                                "Removing dependencies",
                            )?;
                        }
                    }
                    InputContext::CommitMessage => {
                        let command = format!(r#"git add . && git commit -m "{}""#, user_input);
                        run_shell_command(app, shell, "commit", &command, "Committing")?;
                    }
                }
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
            app.input_context = None;
        }
        _ => {}
    }
    Ok(())
}

/// Handles key events when in normal (tips) mode.
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
            KeyCode::Char('h') => app.show_help = true,
            KeyCode::Char('a') => {
                app.bottom_bar_mode = BottomBarMode::Input;
                app.input_context = Some(InputContext::AddPackage);
                app.command_input.clear();
                app.command_cursor_position = 0;
            }
            KeyCode::Char('R') => {
                app.bottom_bar_mode = BottomBarMode::Input;
                app.input_context = Some(InputContext::RemovePackage);
                app.command_input.clear();
                app.command_cursor_position = 0;
            }
            KeyCode::Char('m') => {
                app.bottom_bar_mode = BottomBarMode::Input;
                app.input_context = Some(InputContext::CommitMessage);
                app.command_input.clear();
                app.command_cursor_position = 0;
            }
            KeyCode::Char('r') => execute_script(app, shell, "dev", "Running")?,
            KeyCode::Char('b') => execute_script(app, shell, "build", "Building")?,
            KeyCode::Char('l') => run_shell_command(app, shell, "lint", "clay lint", "Formatting")?,
            KeyCode::Char('P') => execute_script(app, shell, "publish", "Publishing")?,
            KeyCode::Char('p') => run_shell_command(app, shell, "push", "git push", "Pushing")?,
            KeyCode::Char('i') => execute_script(app, shell, "install", "Installing")?,
            KeyCode::Char('q') => execute_script(app, shell, "clean", "Cleaning")?,
            KeyCode::Char('c') => {
                app.terminal.clear();
            }
            _ => {}
        }
    } else if key.code == KeyCode::Char('c') {
        shell.write_to_shell(b"\x03")?;
        app.finish_script(ScriptEndStatus::Cancelled);
    }
    Ok(())
}

/// A helper function to run a command in the shell.
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

/// A helper function to execute a script from the config file.
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
