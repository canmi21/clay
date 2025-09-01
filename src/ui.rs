/* src/ui.rs */

use crate::app::{App, BottomBarMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    // 关键改动：移除了不再使用的 Wrap
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_shell_pane(frame, app, chunks[0]);
    render_logs_pane(frame, app, chunks[1]);
    render_bottom_bar(frame, app, chunks[2]);

    match app.bottom_bar_mode {
        BottomBarMode::Command => {
            let cursor_x = chunks[2].x + 2 + app.command_cursor_position as u16;
            let cursor_y = chunks[2].y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
        _ => {
            let area = frame.area();
            frame.set_cursor_position((area.width, area.height));
        }
    }
}

fn render_shell_pane(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Shell");
    let lines = app.terminal.get_visible_lines();
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_logs_pane(frame: &mut Frame, app: &App, area: Rect) {
    let text = app.logs.join("\n");
    let block = Block::default().borders(Borders::ALL).title("Logs");
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

fn render_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (title, content) = match app.bottom_bar_mode {
        BottomBarMode::Tips => (
            "Tips",
            "Press '/' for command mode. 'Esc' to quit.".to_string(),
        ),
        BottomBarMode::Command => ("Command", format!("> {}", app.command_input)),
        BottomBarMode::Input => ("Input", String::new()),
        BottomBarMode::Progress => ("Progress", "Loading...".to_string()),
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}