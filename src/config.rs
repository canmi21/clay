/* src/config.rs */

use crate::actions::Action;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use strum::IntoEnumIterator;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Keybind {
    Char(char),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub keybindings: HashMap<String, Keybind>,
}

impl Config {
    pub fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let mut config: Config =
                serde_json::from_str(&content).context("Failed to parse config file")?;

            // Ensure all actions have keybindings
            config.ensure_all_actions_present();
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn get_action_for_key(&self, c: char) -> Option<Action> {
        self.keybindings
            .iter()
            .find(|(_, keybind)| **keybind == Keybind::Char(c))
            .and_then(|(action_str, _)| action_str.parse().ok())
    }

    fn ensure_all_actions_present(&mut self) {
        for action in Action::iter() {
            let action_str = action.to_string();
            if !self.keybindings.contains_key(&action_str) {
                let keybind = Self::default_keybind_for_action(action);
                self.keybindings.insert(action_str, keybind);
            }
        }
    }

    fn default_keybind_for_action(action: Action) -> Keybind {
        match action {
            Action::Run => Keybind::Char('r'),
            Action::Build => Keybind::Char('b'),
            Action::Lint => Keybind::Char('l'),
            Action::Publish => Keybind::Char('P'),
            Action::Push => Keybind::Char('p'),
            Action::Install => Keybind::Char('i'),
            Action::Clean => Keybind::Char('q'),
            Action::AddPackage => Keybind::Char('a'),
            Action::RemovePackage => Keybind::Char('R'),
            Action::Commit => Keybind::Char('m'),
            _ => Keybind::None,
        }
    }

    pub fn get_keybind(&self, action: Action) -> Option<&Keybind> {
        self.keybindings.get(&action.to_string())
    }

    pub fn set_keybind(&mut self, action: Action, keybind: Keybind) {
        self.keybindings.insert(action.to_string(), keybind);
    }

    fn get_config_path() -> Result<PathBuf> {
        let base_dirs = directories::BaseDirs::new().context("Could not find home directory")?;
        Ok(base_dirs.home_dir().join(".clay/config.json"))
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut keybindings = HashMap::new();
        for action in Action::iter() {
            let keybind = Self::default_keybind_for_action(action);
            keybindings.insert(action.to_string(), keybind);
        }
        Self { keybindings }
    }
}
