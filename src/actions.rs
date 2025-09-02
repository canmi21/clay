/* src/actions.rs */

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    EnumIter,
    Display,
    EnumString,
    PartialOrd,
    Ord,
)]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Quit,
    ToggleHelp,
    ScrollUp,
    ScrollDown,
    EnterCommandMode,
    ClearShell,
    Lint,
    Run,
    Build,
    Publish,
    Push,
    Install,
    Clean,
    AddPackage,
    RemovePackage,
    Commit,
    LlmPush,
    ShowDiff,
    GenerateMessage,
    VersionUpdate,
}

impl Action {
    pub fn description(&self) -> &'static str {
        match self {
            Action::Quit => "Quit the application",
            Action::ToggleHelp => "Toggle this help/settings popup",
            Action::ScrollUp => "Scroll shell output up",
            Action::ScrollDown => "Scroll shell output down",
            Action::EnterCommandMode => "Enter Command Mode",
            Action::ClearShell => "Clear the shell screen",
            Action::Lint => "Lint & Format project",
            Action::Run => "Run dev build",
            Action::Build => "Build project",
            Action::Publish => "Publish package",
            Action::Push => "Push changes to git remote",
            Action::Install => "Install binary",
            Action::Clean => "Clean build artifacts",
            Action::AddPackage => "Add a new dependency",
            Action::RemovePackage => "Remove a dependency",
            Action::Commit => "Commit all staged changes",
            Action::LlmPush => "Run the full AI commit and push process",
            Action::ShowDiff => "Show the git diff as JSON",
            Action::GenerateMessage => "Generate commit messages with AI",
            Action::VersionUpdate => "Increment patch version",
        }
    }

    pub fn command_str(&self) -> &'static str {
        match self {
            Action::Quit => "/quit",
            Action::ToggleHelp => "/help",
            Action::ScrollUp => "/up",
            Action::ScrollDown => "/down",
            Action::EnterCommandMode => "/",
            Action::ClearShell => "/c",
            Action::Lint => "/lint",
            Action::Run => "/run",
            Action::Build => "/build",
            Action::Publish => "/publish",
            Action::Push => "/push",
            Action::Install => "/install",
            Action::Clean => "/clean",
            Action::AddPackage => "/add",
            Action::RemovePackage => "/remove",
            Action::Commit => "/commit",
            Action::LlmPush => "/llm",
            Action::ShowDiff => "/diff",
            Action::GenerateMessage => "/message",
            Action::VersionUpdate => "/ver",
        }
    }

    pub fn is_editable(&self) -> bool {
        !matches!(
            self,
            Action::Quit
                | Action::ToggleHelp
                | Action::ScrollUp
                | Action::ScrollDown
                | Action::EnterCommandMode
                | Action::ClearShell
        )
    }

    pub fn fixed_keybinding_display(&self) -> Option<&'static str> {
        match self {
            Action::Quit => Some("[Esc]"),
            Action::ToggleHelp => Some("[h]"),
            Action::ScrollUp => Some("[Up]"),
            Action::ScrollDown => Some("[Down]"),
            Action::EnterCommandMode => Some("[/]"),
            Action::ClearShell => Some("[c]"),
            _ => None,
        }
    }
}
