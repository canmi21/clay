/* src/app.rs */

// 导入我们的新模块
use crate::terminal::VirtualTerminal;

#[derive(PartialEq)]
pub enum BottomBarMode {
    Tips,
    Command,
    Input,
    Progress,
}

pub struct App {
    // 关键改动：用 VirtualTerminal 替换旧的 shell_output 和滚动状态
    pub terminal: VirtualTerminal,
    pub logs: Vec<String>,
    pub bottom_bar_mode: BottomBarMode,
    pub should_quit: bool,
    pub command_input: String,
    pub command_cursor_position: usize,
    pub command_history: Vec<String>,
    pub history_index: usize,
}

impl App {
    // App::new 现在需要知道终端尺寸
    pub fn new(cols: u16, rows: u16) -> Self {
        App {
            terminal: VirtualTerminal::new(rows, cols),
            logs: vec!["Welcome to Clay! Press '/' to enter command mode.".to_string()],
            bottom_bar_mode: BottomBarMode::Tips,
            should_quit: false,
            command_input: String::new(),
            command_cursor_position: 0,
            command_history: Vec::new(),
            history_index: 0,
        }
    }

    // add_shell_output 不再需要，逻辑移至 main 循环
    
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