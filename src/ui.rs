/* src/ui.rs */

use crate::app::{App, BottomBarMode, InputContext};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
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
        BottomBarMode::Command | BottomBarMode::Input => {
            let prompt_offset = match app.bottom_bar_mode {
                BottomBarMode::Command => 2, // for "> "
                BottomBarMode::Input => match app.input_context {
                    Some(InputContext::AddPackage) => "Package(s): ".len(),
                    Some(InputContext::RemovePackage) => "Package(s): ".len(),
                    Some(InputContext::CommitMessage) => "Message: ".len(),
                    None => 0,
                },
                _ => 0,
            };

            let cursor_x =
                chunks[2].x + 1 + prompt_offset as u16 + app.command_cursor_position as u16;
            let cursor_y = chunks[2].y + 1;
            frame.set_cursor(cursor_x, cursor_y);
        }
        BottomBarMode::Tips => {
            if let Some((x, y)) = app.terminal.get_cursor_position() {
                let cursor_x = shell_pane_area.x + 1 + x;
                let cursor_y = shell_pane_area.y + 1 + y;
                frame.set_cursor(cursor_x, cursor_y);
            }
        }
        _ => {
            // Do not set cursor, effectively hiding it
        }
    }

    if app.show_help {
        render_help_popup(frame);
    }
}

fn render_shell_pane(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Shell");
    let lines = app.terminal.get_visible_lines();
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_logs_pane(frame: &mut Frame, app: &App, area: Rect) {
    let text: Vec<Line> = app.logs.iter().map(|l| Line::from(l.clone())).collect();
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: true });

    let log_count = app.logs.len();
    let panel_height = area.height.saturating_sub(2) as usize;
    if log_count > panel_height {
        let scroll = (log_count - panel_height) as u16;
        frame.render_widget(paragraph.scroll((scroll, 0)), area);
    } else {
        frame.render_widget(paragraph, area);
    }
}

fn render_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (title, content) = match app.bottom_bar_mode {
        BottomBarMode::Tips => {
            let tips_text = if app.config.is_some() {
                "[/]Cmd [a]Add [R]Remove [r]Run [b]Build [l]Lint [P]Publish [p]Push [m]Commit [i]Install [q]Clean [c]Clear [h]Help [Esc]Quit"
            } else {
                "[/]Cmd [h]Help [Esc]Quit"
            };
            ("Tips", tips_text.to_string())
        }
        BottomBarMode::Command => ("Command", format!("> {}", app.command_input)),
        BottomBarMode::Input => {
            let prompt = match app.input_context {
                Some(InputContext::AddPackage) => "Package(s): ",
                Some(InputContext::RemovePackage) => "Package(s): ",
                Some(InputContext::CommitMessage) => "Message: ",
                None => "",
            };
            ("Input", format!("{}{}", prompt, app.command_input))
        }
        BottomBarMode::Status => ("Status", app.status_message.clone()),
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_help_popup(frame: &mut Frame) {
    let block = Block::default().title("Help").borders(Borders::ALL);

    let text = Text::from(vec![
        Line::from(Span::styled(
            "--- General ---",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("[/]       Enter Command Mode"),
        Line::from("[h]       Toggle this help popup"),
        Line::from("[Up/Down] Scroll shell output"),
        Line::from("[Esc]     Quit application or close popup"),
        Line::from(""),
        Line::from(Span::styled(
            "--- Project (Rust) ---",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("[r]       Run dev build (cargo run)"),
        Line::from("[b]       Build project (cargo build)"),
        Line::from("[l]       Lint & Format (clay lint)"),
        Line::from("[P]       Publish crate (cargo publish)"),
        Line::from("[i]       Install binary (cargo install)"),
        Line::from("[q]       Clean build artifacts (cargo clean)"),
        Line::from(""),
        Line::from(Span::styled(
            "--- Package Management ---",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("[a]       Add dependency (cargo add)"),
        Line::from("[R]       Remove dependency (cargo remove)"),
        Line::from(""),
        Line::from(Span::styled(
            "--- Git ---",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("[m]       Commit all changes"),
        Line::from("[p]       Push to remote"),
        Line::from(""),
        Line::from(Span::styled(
            "--- Interaction ---",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("[c]       Clear shell (Normal) / Cancel script (Running)"),
    ]);

    let area = centered_rect(60, 80, frame.area());
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    frame.render_widget(Clear, area); //this clears the background
    frame.render_widget(paragraph, area);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
