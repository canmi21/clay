/* src/ui.rs */

use crate::app::{App, BottomBarMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};

pub fn ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(7),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let shell_pane_area = chunks[0];
    render_shell_pane(frame, app, shell_pane_area);
    render_logs_pane(frame, app, chunks[1]);
    render_bottom_bar(frame, app, chunks[2]);

    match app.bottom_bar_mode {
        BottomBarMode::Command => {
            let cursor_x = chunks[2].x + 2 + app.command_cursor_position as u16;
            let cursor_y = chunks[2].y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
        BottomBarMode::Tips => {
            if let Some((x, y)) = app.terminal.get_cursor_position() {
                let cursor_x = shell_pane_area.x + 1 + x;
                let cursor_y = shell_pane_area.y + 1 + y;
                frame.set_cursor_position((cursor_x, cursor_y));
            } else {
                frame.set_cursor_position((frame.area().width, frame.area().height));
            }
        }
        _ => {
            frame.set_cursor_position((frame.area().width, frame.area().height));
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
            "Press '/' for command mode. 'Esc' to quit. Use Up/Down to scroll.".to_string(),
        ),
        BottomBarMode::Command => ("Command", format!("> {}", app.command_input)),
        BottomBarMode::Input => ("Input", String::new()),
        BottomBarMode::Progress => ("Progress", "Loading...".to_string()),
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}
