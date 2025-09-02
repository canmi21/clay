/* src/history.rs */

use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY_SIZE: usize = 500;

pub struct CommandHistory {
    commands: VecDeque<String>,
    current_index: Option<usize>,
    temp_input: String,
}

impl CommandHistory {
    pub fn new() -> Result<Self> {
        let mut history = Self {
            commands: VecDeque::new(),
            current_index: None,
            temp_input: String::new(),
        };
        history.load()?;
        Ok(history)
    }

    pub fn add_command(&mut self, command: String) {
        if command.trim().is_empty() {
            return;
        }

        // Don't add duplicate consecutive commands
        if let Some(last) = self.commands.back() {
            if last == &command {
                return;
            }
        }

        self.commands.push_back(command);

        // Keep only MAX_HISTORY_SIZE commands
        while self.commands.len() > MAX_HISTORY_SIZE {
            self.commands.pop_front();
        }

        self.reset_navigation();
    }

    pub fn navigate_up(&mut self, current_input: &str) -> Option<String> {
        if self.commands.is_empty() {
            return None;
        }

        match self.current_index {
            None => {
                // First time navigating, save current input
                self.temp_input = current_input.to_string();
                self.current_index = Some(self.commands.len() - 1);
                self.commands.get(self.commands.len() - 1).cloned()
            }
            Some(index) => {
                if index > 0 {
                    self.current_index = Some(index - 1);
                    self.commands.get(index - 1).cloned()
                } else {
                    None // Already at the oldest command
                }
            }
        }
    }

    pub fn navigate_down(&mut self) -> Option<String> {
        match self.current_index {
            None => None,
            Some(index) => {
                if index < self.commands.len() - 1 {
                    self.current_index = Some(index + 1);
                    self.commands.get(index + 1).cloned()
                } else {
                    // Return to the original input
                    self.current_index = None;
                    Some(self.temp_input.clone())
                }
            }
        }
    }

    pub fn reset_navigation(&mut self) {
        self.current_index = None;
        self.temp_input.clear();
    }

    pub fn save(&self) -> Result<()> {
        let history_path = Self::get_history_path()?;
        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content: Vec<&String> = self.commands.iter().collect();
        let serialized = content
            .iter()
            .map(|cmd| format!("{}\n", cmd))
            .collect::<String>();

        fs::write(history_path, serialized)?;
        Ok(())
    }

    fn load(&mut self) -> Result<()> {
        let history_path = Self::get_history_path()?;
        if !history_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&history_path)?;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.commands.push_back(trimmed.to_string());
            }
        }

        // Ensure we don't exceed max size
        while self.commands.len() > MAX_HISTORY_SIZE {
            self.commands.pop_front();
        }

        Ok(())
    }

    fn get_history_path() -> Result<PathBuf> {
        let base_dirs = directories::BaseDirs::new().context("Could not find home directory")?;
        Ok(base_dirs.home_dir().join(".clay/history.clay"))
    }
}

impl Drop for CommandHistory {
    fn drop(&mut self) {
        // Save history when the struct is dropped
        let _ = self.save();
    }
}
