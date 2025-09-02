# Clay - Your Modern Terminal Workspace

Clay is a next-generation command-line utility, written in Rust, designed to be your primary terminal workspace. It enhances the traditional shell experience with a persistent, interactive Terminal User Interface (TUI), while providing a powerful suite of integrated tools for project management, versioning, and AI-assisted development.

At its core, Clay is a high-fidelity terminal emulator that ensures your favorite command-line tools work exactly as you expect.

## Features

Clay is built around two core components: an interactive TUI for day-to-day operations and a set of powerful CLI commands for automation and scripting.

<div align="center">
<a href="https://github.com/canmi21/clay" target="_blank"><img src="https://raw.githubusercontent.com/canmi21/clay/refs/heads/main/img/clay.png" alt="clay" width="99%"/></a>
<a href="https://github.com/canmi21/clay" target="_blank"><img src="https://raw.githubusercontent.com/canmi21/clay/refs/heads/main/img/build.png" alt="clay" width="99%"/></a>
<a href="https://github.com/canmi21/clay" target="_blank"><img src="https://raw.githubusercontent.com/canmi21/clay/refs/heads/main/img/cancel.png" alt="clay" width="99%"/></a>
<a href="https://github.com/canmi21/clay" target="_blank"><img src="https://raw.githubusercontent.com/canmi21/clay/refs/heads/main/img/lint.png" alt="clay" width="99%"/></a>
<a href="https://github.com/canmi21/clay" target="_blank"><img src="https://raw.githubusercontent.com/canmi21/clay/refs/heads/main/img/command.png" alt="clay" width="99%"/></a>
<a href="https://github.com/canmi21/clay" target="_blank"><img src="https://raw.githubusercontent.com/canmi21/clay/refs/heads/main/img/commit.png" alt="clay" width="99%"/></a>
<a href="https://github.com/canmi21/clay" target="_blank"><img src="https://raw.githubusercontent.com/canmi21/clay/refs/heads/main/img/fun.png" alt="clay" width="99%"/></a>
<a href="https://github.com/canmi21/clay" target="_blank"><img src="https://raw.githubusercontent.com/canmi21/clay/refs/heads/main/img/help.png" alt="clay" width="99%"/></a>
<a href="https://github.com/canmi21/clay" target="_blank"><img src="https://raw.githubusercontent.com/canmi21/clay/refs/heads/main/img/clay-config.png" alt="clay" width="99%"/></a>
</div>

### The Interactive TUI: A True Shell Experience

Launch the TUI by simply running `clay` in your project's root directory. The TUI is designed to be a robust replacement for a standard terminal session.

- **High-Fidelity Terminal Emulation**: The integrated shell offers full support for ASCII art, ANSI escape codes, and precise cursor positioning. This means complex TUI applications like `vim`, `htop`, `lazygit`, and others run seamlessly inside Clay, providing a true-to-form experience.
- **Persistent & Scrollable Session**: A central pane gives you a persistent, scrollable pseudoterminal session within your project. Never lose your command history or output again.
- **Dynamic Action Bar**: A context-aware bottom bar that displays available commands and their keybindings. It also shows the status of ongoing tasks.
- **Command Palette**: Press `/` to enter command mode. You can either execute internal Clay commands (e.g., `/lint`, `/quit`) or run any standard shell command directly.
- **Customizable Keybindings**: Press `h` to open the Help & Settings menu, where you can view all available actions and customize their keybindings. Changes are saved globally to `~/.clay/config.json`.
- **Intelligent Conflict Resolution**: If you assign the same key to multiple actions, Clay will detect the conflict and help you resolve it before saving.

### Augmented Tooling (CLI Commands)

Use Clay's CLI for powerful, one-off actions or for integration into scripts. These tools complement the core TUI experience.

#### AI Workflow (Optional)

- `clay llm commit`: Analyzes your staged git changes (`git diff`), sends a compact summary to the Gemini API, and generates conventional commit messages for each modified file.
- `clay llm git`: A fully automated workflow. It runs `clay llm commit`, uses the AI-generated messages to commit each file individually, bumps your project's patch version, and creates a final version commit.
- `clay llm push`: Runs the entire `clay llm git` workflow and then pushes all commits to your remote repository.
- `clay llm token`: Securely sets and stores your Google Gemini API key in `~/.clay/token`.

#### Project Management

- `clay project update`: Increments the patch version of your project (e.g., `1.1.5` -> `1.1.6`). Currently supports `Cargo.toml`.
- `clay project bump`: Increments the minor version and resets the patch version (e.g., `1.1.6` -> `1.2.0`).

#### Utilities

- `clay lint`: A multi-stage linter. It first runs your project-specific lint command (defined in `clay-config.json`), then formats file headers, and finally normalizes dependency versions in `Cargo.toml`.
- `clay diff`: Generates a compact, LLM-friendly JSON summary of `git diff`. It intelligently truncates large files and includes new, untracked files in the output.

## Configuration

Clay uses a combination of global and project-specific configuration files.

- **Global Keybindings** (`~/.clay/config.json`): Your custom keybindings to all TUI actions.
- **Project Commands** (`./clay-config.json`): Define project-specific script implementations (e.g., what the Run or Build action should execute). Clay will automatically generate a default one for supported project types (currently Rust).

## Getting Started

### Installation

To install Clay, you will need Rust and Cargo installed on your system. You can then install it directly from crates.io:

```bash
cargo install clay-cli
```

Or, you can build from source:

```bash
git clone https://github.com/canmi21/clay.git
cd clay
cargo install --path .
```

### Setup

1. **Set your API Key (Optional)**: Before using the AI features, you need to set your Google Gemini API key.

   ```bash
   clay llm token
   ```

   Follow the prompts to paste your API key.

2. **Launch the TUI**: Navigate to your project directory and run:

   ```bash
   clay
   ```

   If a supported project (like a Rust project with a `Cargo.toml`) is detected, a `clay-config.json` file will be created for you.

3. **Explore and Customize**: Press `h` inside the TUI to see all commands and change keybindings to your liking.