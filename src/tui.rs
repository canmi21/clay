/* src/tui.rs */

use crate::actions::Action;
use crate::app::{App, BottomBarMode, HelpConflictDialogSelection, InputContext, ScriptEndStatus};
use crate::config::{Config, Keybind};
use crate::project;
use crate::shell::ShellProcess;
use crate::ui::ui;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::{collections::HashMap, time::Duration};
use strum::IntoEnumIterator;

const CMD_FINISHED_MARKER: &str = "CLAY_CMD_FINISHED_MARKER_v1";

/// Initializes and runs the terminal user interface.
pub fn run_tui() -> Result<()> {
    let config = Config::new()?;
    let project_config = project::load_or_create_config()?;

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();

    // Clear the terminal before entering alternate screen
    execute!(stdout, Clear(ClearType::All))?;
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let size = terminal.size()?;
    let shell_pane_outer_height = size.height.saturating_sub(10);
    let shell_pane_inner_height = shell_pane_outer_height.saturating_sub(2);
    let shell_pane_inner_width = size.width.saturating_sub(2);

    let mut app = App::new(
        shell_pane_inner_width,
        shell_pane_inner_height,
        config,
        project_config,
    );
    if app.project_config.is_some() {
        app.logs
            .push("Rust project detected. Config loaded.".to_string());
    } else {
        app.logs.push("No project type detected.".to_string());
    }

    let mut shell_process = ShellProcess::new(shell_pane_inner_height, shell_pane_inner_width)?;

    let result = run_app(&mut terminal, &mut app, &mut shell_process);

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
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
                if app.show_conflict_dialog {
                    handle_conflict_dialog_keys(key, app)?;
                } else if app.is_editing_keybinding {
                    handle_help_edit_mode_keys(key, app);
                } else if app.show_help {
                    handle_help_mode_keys(key, app)?;
                } else {
                    handle_main_view_keys(key, app, shell_process)?;
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_main_view_keys(
    key: event::KeyEvent,
    app: &mut App,
    shell: &mut ShellProcess,
) -> Result<()> {
    match app.bottom_bar_mode {
        BottomBarMode::Tips => {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.should_quit = true;
                return Ok(());
            }

            match key.code {
                KeyCode::Char('h') => {
                    app.show_help = true;
                }
                KeyCode::Char('/') => {
                    app.bottom_bar_mode = BottomBarMode::Command;
                    app.reset_history_navigation();
                }
                KeyCode::Up => app.scroll_up(),
                KeyCode::Down => app.scroll_down(),
                KeyCode::Esc => {
                    app.should_quit = true;
                }
                KeyCode::Char(c) => {
                    if let Some(action) = app.config.get_action_for_key(c) {
                        dispatch_action(action, app, shell)?;
                    }
                }
                _ => {}
            }
        }
        BottomBarMode::Status => {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                shell.write_to_shell(b"\x03")?;
                app.finish_script(ScriptEndStatus::Cancelled);
            }
        }
        BottomBarMode::Command => {
            handle_command_mode_keys(key, app, shell)?;
        }
        BottomBarMode::Input => {
            handle_input_mode_keys(key, app, shell)?;
        }
    }
    Ok(())
}

fn handle_command_mode_keys(
    key: event::KeyEvent,
    app: &mut App,
    shell: &mut ShellProcess,
) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            let input = app.command_input.trim().to_string();
            app.submit_command();

            if input.starts_with('/') {
                let parts = input.split_whitespace();
                let command_str = parts.into_iter().next().unwrap_or("");

                let action_map: HashMap<&str, Action> =
                    Action::iter().map(|a| (a.command_str(), a)).collect();

                if let Some(action) = action_map.get(command_str) {
                    dispatch_action(*action, app, shell)?;
                } else if command_str == "/exit" {
                    dispatch_action(Action::Quit, app, shell)?;
                }
            } else {
                app.terminal.clear();
                let command = format!("{}\n", input);
                shell.write_to_shell(command.as_bytes())?;
            }
        }
        KeyCode::Up => {
            app.navigate_history_up();
        }
        KeyCode::Down => {
            app.navigate_history_down();
        }
        KeyCode::Char(c) => {
            app.reset_history_navigation();
            app.enter_char(c);
        }
        KeyCode::Backspace => {
            app.reset_history_navigation();
            app.delete_char();
        }
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Esc => {
            app.reset_history_navigation();
            app.bottom_bar_mode = BottomBarMode::Tips;
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
            let user_input = app.command_input.trim().to_string();
            let context = app.input_context.take();

            app.command_input.clear();
            app.command_cursor_position = 0;
            app.bottom_bar_mode = BottomBarMode::Tips;

            if user_input.is_empty() {
                return Ok(());
            }

            if let Some(context) = context {
                let (script_name, status) = match context {
                    InputContext::AddPackage => ("add", "Adding dependencies"),
                    InputContext::RemovePackage => ("remove", "Removing dependencies"),
                    InputContext::CommitMessage => ("commit", "Committing"),
                };

                if context == InputContext::CommitMessage {
                    let command = format!(r#"git add . && git commit -m "{}""#, user_input);
                    run_shell_command(app, shell, "commit", &command, status)?;
                } else {
                    let command_to_run = app
                        .project_config
                        .as_ref()
                        .and_then(|c| c.scripts.get(script_name).cloned())
                        .map(|base_cmd| format!("{} {}", base_cmd, user_input));

                    if let Some(command) = command_to_run {
                        run_shell_command(app, shell, script_name, &command, status)?;
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

fn handle_help_mode_keys(key: event::KeyEvent, app: &mut App) -> Result<()> {
    let num_actions = app.sorted_actions.len();

    match key.code {
        KeyCode::Up => {
            app.help_selected_action_index = app.help_selected_action_index.saturating_sub(1);
        }
        KeyCode::Down => {
            app.help_selected_action_index =
                (app.help_selected_action_index + 1).min(num_actions - 1);
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let selected_action = app.sorted_actions[app.help_selected_action_index];
            if selected_action.is_editable() {
                app.is_editing_keybinding = true;
            }
        }
        KeyCode::Esc | KeyCode::Char('h') => {
            attempt_close_help(app)?;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            attempt_close_help(app)?;
        }
        _ => {}
    }
    Ok(())
}

fn attempt_close_help(app: &mut App) -> Result<()> {
    if app.validate_and_prepare_to_close_help() {
        if let Err(e) = app.config.save() {
            app.logs
                .push(format!("Warning: Failed to save config: {}", e));
        }
    }
    Ok(())
}

fn handle_help_edit_mode_keys(key: event::KeyEvent, app: &mut App) {
    let selected_action = app.sorted_actions[app.help_selected_action_index];

    let new_keybind = match key.code {
        KeyCode::Char(c) if c.is_ascii_alphanumeric() => Some(Keybind::Char(c)),
        KeyCode::Esc => Some(Keybind::None),
        _ => None,
    };

    if let Some(keybind) = new_keybind {
        app.config.set_keybind(selected_action, keybind);
        app.is_editing_keybinding = false;
    }
}

fn handle_conflict_dialog_keys(key: event::KeyEvent, app: &mut App) -> Result<()> {
    match key.code {
        KeyCode::Left => app.conflict_dialog_selection = HelpConflictDialogSelection::Unbind,
        KeyCode::Right => app.conflict_dialog_selection = HelpConflictDialogSelection::Inspect,
        KeyCode::Enter => match app.conflict_dialog_selection {
            HelpConflictDialogSelection::Unbind => {
                app.unbind_conflicting_keys();
                app.show_conflict_dialog = false;
                app.show_help = false;
                if let Err(e) = app.config.save() {
                    app.logs
                        .push(format!("Warning: Failed to save config: {}", e));
                }
            }
            HelpConflictDialogSelection::Inspect => {
                app.show_conflict_dialog = false;
            }
        },
        KeyCode::Esc => {
            app.show_conflict_dialog = false;
        }
        _ => {}
    }
    Ok(())
}

fn dispatch_action(action: Action, app: &mut App, shell: &mut ShellProcess) -> Result<()> {
    match action {
        Action::Quit => app.should_quit = true,
        Action::ToggleHelp => app.show_help = true,
        Action::ScrollUp => app.scroll_up(),
        Action::ScrollDown => app.scroll_down(),
        Action::EnterCommandMode => {
            app.bottom_bar_mode = BottomBarMode::Command;
            app.reset_history_navigation();
        }
        Action::ClearShell => app.terminal.clear(),
        Action::AddPackage => {
            app.bottom_bar_mode = BottomBarMode::Input;
            app.input_context = Some(InputContext::AddPackage);
        }
        Action::RemovePackage => {
            app.bottom_bar_mode = BottomBarMode::Input;
            app.input_context = Some(InputContext::RemovePackage);
        }
        Action::Commit => {
            app.bottom_bar_mode = BottomBarMode::Input;
            app.input_context = Some(InputContext::CommitMessage);
        }
        Action::Lint => run_shell_command(app, shell, "lint", "clay lint", "Formatting")?,
        Action::Push => run_shell_command(app, shell, "push", "git push", "Pushing")?,
        Action::LlmPush => {
            run_shell_command(app, shell, "llm-push", "clay llm push", "AI Pushing")?
        }
        Action::ShowDiff => run_shell_command(app, shell, "diff", "clay diff", "Diffing")?,
        Action::GenerateMessage => {
            run_shell_command(app, shell, "message", "clay llm commit", "Generating")?
        }
        Action::VersionUpdate => run_shell_command(
            app,
            shell,
            "ver-update",
            "clay project update",
            "Versioning",
        )?,

        Action::Run => execute_project_script(app, shell, "dev", "Running")?,
        Action::Build => execute_project_script(app, shell, "build", "Building")?,
        Action::Publish => execute_project_script(app, shell, "publish", "Publishing")?,
        Action::Install => execute_project_script(app, shell, "install", "Installing")?,
        Action::Clean => execute_project_script(app, shell, "clean", "Cleaning")?,
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
    let message = format!("{} (Press Ctrl+c to cancel)...", status);
    app.start_script(script_name, &message);
    Ok(())
}

fn execute_project_script(
    app: &mut App,
    shell: &mut ShellProcess,
    script_name: &str,
    status: &str,
) -> Result<()> {
    let command_to_run = if let Some(config) = &app.project_config {
        config.scripts.get(script_name).cloned()
    } else {
        None
    };

    if let Some(command) = command_to_run {
        run_shell_command(app, shell, script_name, &command, status)?;
    }
    Ok(())
}
