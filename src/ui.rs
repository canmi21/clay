/* src/ui.rs */

use crate::actions::Action;
use crate::app::{App, BottomBarMode, HelpConflictDialogSelection, InputContext};
use crate::config::Keybind;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
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

    render_shell_pane(frame, app, chunks[0]);
    render_logs_pane(frame, app, chunks[1]);
    render_bottom_bar(frame, app, chunks[2]);
    update_cursor(frame, app, chunks[0], chunks[2]);

    if app.show_help {
        if app.show_conflict_dialog {
            render_conflict_dialog(frame, app);
        } else {
            render_help_settings_screen(frame, app);
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
            let mut tips = vec!["[/]Cmd".to_string()];
            let tip_map = [
                (Action::AddPackage, "Add"),
                (Action::RemovePackage, "Remove"),
                (Action::Run, "Run"),
                (Action::Build, "Build"),
                (Action::Lint, "Lint"),
                (Action::Publish, "Publish"),
                (Action::Push, "Push"),
                (Action::Commit, "Commit"),
                (Action::Install, "Install"),
                (Action::Clean, "Clean"),
            ];

            for (action, name) in tip_map {
                let key_char = app
                    .config
                    .get_keybind(action)
                    .and_then(|kb| {
                        if let Keybind::Char(c) = kb {
                            Some(*c)
                        } else {
                            None
                        }
                    })
                    .map_or(' ', |c| c);
                tips.push(format!("[{}]{}", key_char, name));
            }

            // Fixed shortcuts at the end
            tips.push("[c]Cancel".to_string());
            tips.push("[h]Help".to_string());
            tips.push("[Esc]Quit".to_string());

            ("Tips", tips.join(" "))
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

fn update_cursor(frame: &mut Frame, app: &App, shell_area: Rect, bottom_bar_area: Rect) {
    if app.show_help {
        return; // No cursor in help mode
    }

    match app.bottom_bar_mode {
        BottomBarMode::Command | BottomBarMode::Input => {
            let prompt_offset = match app.bottom_bar_mode {
                BottomBarMode::Command => 2, // for "> "
                BottomBarMode::Input => match app.input_context {
                    Some(InputContext::AddPackage) | Some(InputContext::RemovePackage) => {
                        "Package(s): ".len()
                    }
                    Some(InputContext::CommitMessage) => "Message: ".len(),
                    None => 0,
                },
                _ => 0,
            };

            let cursor_x =
                bottom_bar_area.x + 1 + prompt_offset as u16 + app.command_cursor_position as u16;
            let cursor_y = bottom_bar_area.y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
        BottomBarMode::Tips => {
            if let Some((x, y)) = app.terminal.get_cursor_position() {
                let cursor_x = shell_area.x + 1 + x;
                let cursor_y = shell_area.y + 1 + y;
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
        _ => {}
    }
}

fn render_help_settings_screen(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 90, frame.area());

    let header_cells = ["Command", "Description", "Keybinding"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells).height(1);

    let rows = app.sorted_actions.iter().enumerate().map(|(i, &action)| {
        let is_selected = i == app.help_selected_action_index;

        if !action.is_editable() {
            let keybind_str = action.fixed_keybinding_display().unwrap_or("[N/A]");
            Row::new(vec![
                Cell::from(action.command_str()),
                Cell::from(action.description()),
                Cell::from(Span::styled(
                    keybind_str,
                    Style::default().fg(Color::DarkGray),
                )),
            ])
        } else {
            let keybind = app.config.get_keybind(action).unwrap_or(&Keybind::None);
            let keybind_str = match keybind {
                Keybind::Char(c) => format!("[{}]", c),
                Keybind::None => "[None]".to_string(),
            };

            let mut keybind_style = Style::default().fg(Color::Cyan);
            if let Keybind::Char(c) = keybind {
                if app.key_conflicts.contains(c) {
                    keybind_style = keybind_style.fg(Color::Red).add_modifier(Modifier::BOLD);
                }
            }

            if is_selected && app.is_editing_keybinding {
                keybind_style = keybind_style.bg(Color::White).fg(Color::Black);
            }

            Row::new(vec![
                Cell::from(action.command_str()),
                Cell::from(action.description()),
                Cell::from(Span::styled(keybind_str, keybind_style)),
            ])
        }
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(15),
            Constraint::Min(40),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Help & Keybindings"),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_widget(Clear, area);
    let mut table_state =
        ratatui::widgets::TableState::default().with_selected(app.help_selected_action_index);
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_conflict_dialog(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 30, frame.area());
    let block = Block::default()
        .title("Keybinding Conflicts Detected")
        .borders(Borders::ALL);

    let conflict_keys: Vec<String> = app.key_conflicts.iter().map(|c| c.to_string()).collect();
    let conflicts_text = if conflict_keys.is_empty() {
        "No conflicts".to_string()
    } else {
        format!("Conflicting keys: {}", conflict_keys.join(", "))
    };

    let unbind_style = if app.conflict_dialog_selection == HelpConflictDialogSelection::Unbind {
        Style::default().bg(Color::White).fg(Color::Black)
    } else {
        Style::default().fg(Color::White)
    };

    let inspect_style = if app.conflict_dialog_selection == HelpConflictDialogSelection::Inspect {
        Style::default().bg(Color::White).fg(Color::Black)
    } else {
        Style::default().fg(Color::White)
    };

    let text = Text::from(vec![
        Line::from("Multiple actions are using the same keys!"),
        Line::from(""),
        Line::from(conflicts_text),
        Line::from(""),
        Line::from("Choose an option:"),
        Line::from(""),
        Line::from(vec![
            Span::styled("[ Unbind Conflicts ]", unbind_style),
            Span::raw("  "),
            Span::styled("[ Inspect ]", inspect_style),
        ]),
        Line::from(""),
        Line::from("Use ← → to select, Enter to confirm, Esc to cancel"),
    ]);

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

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
