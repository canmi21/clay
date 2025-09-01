/* src/app.rs */

#[derive(PartialEq)]
pub enum BottomBarMode {
    Tips,
    Command,
    Input,
    Progress,
}

pub struct App {
    pub shell_output: Vec<String>,
    pub logs: Vec<String>,
    // New state for shell scrolling, removed logs_scroll_state
    pub shell_scroll_state: u16,
    pub bottom_bar_mode: BottomBarMode,
    pub should_quit: bool,
    pub command_input: String,
    pub command_cursor_position: usize,
    pub command_history: Vec<String>,
    pub history_index: usize,
}

impl App {
    pub fn new() -> App {
        App {
            shell_output: vec![],
            logs: vec!["Welcome to Clay! Press '/' to enter command mode.".to_string()],
            shell_scroll_state: 0,
            bottom_bar_mode: BottomBarMode::Tips,
            should_quit: false,
            command_input: String::new(),
            command_cursor_position: 0,
            command_history: Vec::new(),
            history_index: 0,
        }
    }

    // New method to add output to the shell pane and enforce 500 lines limit
    pub fn add_shell_output(&mut self, new_lines_str: String) {
        let new_lines: Vec<String> = new_lines_str.split('\n').map(String::from).collect();
        self.shell_output.extend(new_lines);
        
        let line_count = self.shell_output.len();
        if line_count > 500 {
            let lines_to_remove = line_count - 500;
            self.shell_output.drain(0..lines_to_remove);
            // Adjust scroll state to prevent it from being out of bounds
            self.shell_scroll_state = self.shell_scroll_state.saturating_sub(lines_to_remove as u16);
        }
    }

    // Methods from previous step remain the same
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
}