/* src/main.rs */

mod app;
mod shell;
mod ui;
// 添加新模块
mod terminal;

use crate::app::{App, BottomBarMode};
use crate::shell::ShellProcess;
use crate::ui::ui;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{io, time::Duration};

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (cols, rows) = terminal.size().map(|r| (r.width, r.height))?;
    // App::new 现在需要尺寸
    let mut app = App::new(cols, rows.saturating_sub(8)); // 减去日志和命令栏高度
    let mut shell_process = ShellProcess::new(rows.saturating_sub(8), cols)?;

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

        // 关键改动：读取字节并喂给 VTE
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
                    _ => handle_normal_mode_keys(key, app),
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

// ... handle_command_mode_keys 保持不变 ...
fn handle_command_mode_keys(
    key: event::KeyEvent,
    app: &mut App,
    shell: &mut ShellProcess,
) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
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
            if !app.command_history.is_empty() && app.history_index < app.command_history.len() - 1 {
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

// 关键改动：Up/Down 不再需要控制滚动
fn handle_normal_mode_keys(key: event::KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('/') => {
            if app.bottom_bar_mode != BottomBarMode::Input && app.bottom_bar_mode != BottomBarMode::Progress {
                app.bottom_bar_mode = BottomBarMode::Command;
                app.history_index = app.command_history.len();
            }
        }
        _ => {} // Up/Down 不再有作用
    }
}