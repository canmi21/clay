/* src/app.rs */

use crate::project::ProjectConfig;

#[derive(PartialEq)]
pub enum BottomBarMode {
    Tips,
    Command,
    Input,
    Status,
}

pub enum InputContext {
    AddPackage,
    RemovePackage,
    CommitMessage,
}

pub enum ScriptEndStatus {
    Finished,
    Cancelled,
}

pub struct App {
    pub terminal: crate::terminal::VirtualTerminal,
    pub logs: Vec<String>,
    pub bottom_bar_mode: BottomBarMode,
    pub should_quit: bool,
    pub command_input: String,
    pub command_cursor_position: usize,
    pub command_history: Vec<String>,
    pub history_index: usize,
    pub config: Option<ProjectConfig>,
    pub is_script_running: bool,
    pub running_script_name: String,
    pub status_message: String,
    pub input_context: Option<InputContext>,
    pub show_help: bool,
}

impl App {
    pub fn new(cols: u16, rows: u16, config: Option<ProjectConfig>) -> Self {
        App {
            terminal: crate::terminal::VirtualTerminal::new(rows, cols),
            logs: Vec::new(),
            bottom_bar_mode: BottomBarMode::Tips,
            should_quit: false,
            command_input: String::new(),
            command_cursor_position: 0,
            command_history: Vec::new(),
            history_index: 0,
            config,
            is_script_running: false,
            running_script_name: String::new(),
            status_message: String::new(),
            input_context: None,
            show_help: false,
        }
    }

    pub fn scroll_up(&mut self) {
        self.terminal.scroll_up(1);
    }

    pub fn scroll_down(&mut self) {
        self.terminal.scroll_down(1);
    }

    pub fn move_cursor_left(&mut self) {
        self.command_cursor_position = self.command_cursor_position.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        self.command_cursor_position = self
            .command_cursor_position
            .saturating_add(1)
            .min(self.command_input.len());
    }

    pub fn enter_char(&mut self, new_char: char) {
        self.command_input
            .insert(self.command_cursor_position, new_char);
        self.move_cursor_right();
    }

    pub fn delete_char(&mut self) {
        if self.command_cursor_position > 0 {
            let current_idx = self.command_cursor_position;
            let from_left = self.command_input.chars().take(current_idx - 1);
            let from_right = self.command_input.chars().skip(current_idx);
            self.command_input = from_left.chain(from_right).collect();
            self.move_cursor_left();
        }
    }

    pub fn submit_command(&mut self) {
        let cmd = self.command_input.trim();
        if !cmd.is_empty() {
            if self.command_history.last() != Some(&cmd.to_string()) {
                self.command_history.push(cmd.to_string());
            }
        }
        self.history_index = self.command_history.len();
        self.command_input.clear();
        self.command_cursor_position = 0;
        self.bottom_bar_mode = BottomBarMode::Tips;
    }

    pub fn start_script(&mut self, name: &str, status_message: &str) {
        self.is_script_running = true;
        self.running_script_name = name.to_string();
        self.status_message = status_message.to_string();
        self.bottom_bar_mode = BottomBarMode::Status;
        self.logs
            .push(format!("Script '{}' running...", self.running_script_name));
    }

    pub fn finish_script(&mut self, status: ScriptEndStatus) {
        let log_message = match status {
            ScriptEndStatus::Finished => format!("Script '{}' finished.", self.running_script_name),
            ScriptEndStatus::Cancelled => {
                format!("Script '{}' cancelled.", self.running_script_name)
            }
        };
        self.logs.push(log_message);
        self.is_script_running = false;
        self.running_script_name.clear();
        self.status_message.clear();
        self.bottom_bar_mode = BottomBarMode::Tips;
    }
}
