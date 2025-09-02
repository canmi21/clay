/* src/app.rs */

use crate::actions::Action;
use crate::config::{Config, Keybind};
use crate::history::CommandHistory;
use crate::project::ProjectConfig;
use crate::terminal::VirtualTerminal;
use std::collections::{HashMap, HashSet};
use strum::IntoEnumIterator;

#[derive(PartialEq)]
pub enum BottomBarMode {
    Tips,
    Command,
    Input,
    Status,
}

#[derive(PartialEq, Clone, Copy)]
pub enum InputContext {
    AddPackage,
    RemovePackage,
    CommitMessage,
}

#[derive(PartialEq, Clone, Copy)]
pub enum HelpConflictDialogSelection {
    Unbind,
    Inspect,
}

pub enum ScriptEndStatus {
    Finished,
    Cancelled,
}

pub struct App {
    pub terminal: VirtualTerminal,
    pub logs: Vec<String>,
    pub bottom_bar_mode: BottomBarMode,
    pub should_quit: bool,
    pub command_input: String,
    pub command_cursor_position: usize,
    pub command_history: CommandHistory,
    pub config: Config,
    pub project_config: Option<ProjectConfig>,
    pub is_script_running: bool,
    pub current_script: String,
    pub status_message: String,
    pub input_context: Option<InputContext>,
    // Help screen state
    pub show_help: bool,
    pub help_selected_action_index: usize,
    pub is_editing_keybinding: bool,
    pub show_conflict_dialog: bool,
    pub key_conflicts: HashSet<char>,
    pub conflict_dialog_selection: HelpConflictDialogSelection,
    pub sorted_actions: Vec<Action>,
}

impl App {
    pub fn new(
        cols: u16,
        rows: u16,
        config: Config,
        project_config: Option<ProjectConfig>,
    ) -> Self {
        let mut sorted_actions: Vec<Action> = Action::iter().collect();
        sorted_actions.sort_by(|a, b| {
            let a_editable = a.is_editable();
            let b_editable = b.is_editable();
            a_editable
                .cmp(&b_editable)
                .then_with(|| a.command_str().cmp(b.command_str()))
        });

        let command_history = CommandHistory::new().unwrap_or_else(|_| {
            // If history fails to load, create empty one
            CommandHistory::new().expect("Failed to create command history")
        });

        App {
            terminal: VirtualTerminal::new(rows, cols),
            logs: Vec::new(),
            bottom_bar_mode: BottomBarMode::Tips,
            should_quit: false,
            command_input: String::new(),
            command_cursor_position: 0,
            command_history,
            config,
            project_config,
            is_script_running: false,
            current_script: String::new(),
            status_message: String::new(),
            input_context: None,
            show_help: false,
            help_selected_action_index: 0,
            is_editing_keybinding: false,
            show_conflict_dialog: false,
            key_conflicts: HashSet::new(),
            conflict_dialog_selection: HelpConflictDialogSelection::Inspect,
            sorted_actions,
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
            self.command_history.add_command(cmd.to_string());
        }
        self.command_input.clear();
        self.command_cursor_position = 0;
        self.bottom_bar_mode = BottomBarMode::Tips;
    }

    pub fn navigate_history_up(&mut self) {
        if let Some(command) = self.command_history.navigate_up(&self.command_input) {
            self.command_input = command;
            self.command_cursor_position = self.command_input.len();
        }
    }

    pub fn navigate_history_down(&mut self) {
        if let Some(command) = self.command_history.navigate_down() {
            self.command_input = command;
            self.command_cursor_position = self.command_input.len();
        }
    }

    pub fn reset_history_navigation(&mut self) {
        self.command_history.reset_navigation();
    }

    pub fn start_script(&mut self, name: &str, status_msg: &str) {
        self.is_script_running = true;
        self.current_script = name.to_string();
        self.status_message = status_msg.to_string();
        self.bottom_bar_mode = BottomBarMode::Status;
        self.logs
            .push(format!("Script '{}' running...", self.current_script));
    }

    pub fn finish_script(&mut self, status: ScriptEndStatus) {
        let log_message = match status {
            ScriptEndStatus::Finished => format!("Script '{}' finished.", self.current_script),
            ScriptEndStatus::Cancelled => format!("Script '{}' cancelled.", self.current_script),
        };
        self.logs.push(log_message);
        self.is_script_running = false;
        self.current_script.clear();
        self.status_message.clear();
        self.bottom_bar_mode = BottomBarMode::Tips;
    }

    /// Check for keybinding conflicts and prepare to close help screen
    /// Returns true if help can be closed immediately, false if conflicts need resolution
    pub fn validate_and_prepare_to_close_help(&mut self) -> bool {
        self.key_conflicts.clear();
        let mut char_usage = HashMap::new();

        // Count usage of each character key from both editable and fixed keybindings
        for action in Action::iter() {
            let key_char = if action.is_editable() {
                // For editable actions, get from config
                if let Some(Keybind::Char(c)) = self.config.get_keybind(action) {
                    Some(*c)
                } else {
                    None
                }
            } else {
                // For fixed actions, get their fixed key
                match action {
                    Action::ToggleHelp => Some('h'),
                    Action::ScrollUp => None, // Arrow keys don't conflict with chars
                    Action::ScrollDown => None,
                    Action::EnterCommandMode => Some('/'),
                    Action::ClearShell => Some('c'),
                    Action::Quit => None, // Esc doesn't conflict with chars
                    _ => None,
                }
            };

            if let Some(c) = key_char {
                char_usage.entry(c).or_insert_with(Vec::new).push(action);
            }
        }

        // Find conflicts (characters used by multiple actions)
        for (char_key, actions) in char_usage {
            if actions.len() > 1 {
                self.key_conflicts.insert(char_key);
            }
        }

        if self.key_conflicts.is_empty() {
            self.show_help = false;
            true
        } else {
            self.show_conflict_dialog = true;
            false
        }
    }

    /// Resolve conflicts by unbinding conflicting keys intelligently
    pub fn unbind_conflicting_keys(&mut self) {
        let conflicts = self.key_conflicts.clone();

        for conflict_char in conflicts {
            // Find all actions that use this conflicting character
            let mut conflicting_actions = Vec::new();
            for action in Action::iter() {
                if let Some(Keybind::Char(c)) = self.config.get_keybind(action) {
                    if *c == conflict_char {
                        conflicting_actions.push(action);
                    }
                }
            }

            // Check if any of the conflicting actions have fixed keybindings
            let has_fixed_action = conflicting_actions
                .iter()
                .any(|action| !action.is_editable());

            if has_fixed_action {
                // If there's a fixed action, unbind all editable actions
                for action in conflicting_actions {
                    if action.is_editable() {
                        self.config.set_keybind(action, Keybind::None);
                    }
                    // Fixed actions keep their keybinding unchanged
                }
            } else {
                // If all actions are editable, unbind all of them
                for action in conflicting_actions {
                    self.config.set_keybind(action, Keybind::None);
                }
            }
        }

        self.key_conflicts.clear();
    }
}
