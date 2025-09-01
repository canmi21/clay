/* src/terminal.rs */

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use vte::{Parser, Perform};

#[derive(Clone, Debug)]
pub struct Cell {
    pub c: char,
    pub fg: Color,
    pub bg: Color,
    pub flags: CellFlags,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            c: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            flags: CellFlags::empty(),
        }
    }
}

bitflags::bitflags! {
    #[derive(Clone, Debug)]
    pub struct CellFlags: u8 {
        const BOLD = 1;
        const ITALIC = 2;
        const UNDERLINE = 4;
        const INVERSE = 8;
    }
}

pub struct Grid {
    cells: Vec<Vec<Cell>>,
    rows: usize,
    cols: usize,
}

impl Grid {
    pub fn new(rows: usize, cols: usize) -> Self {
        let cells = vec![vec![Cell::default(); cols]; rows];
        Self { cells, rows, cols }
    }

    pub fn cell_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        self.cells.get_mut(row)?.get_mut(col)
    }

    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.cells.get(row)?.get(col)
    }

    pub fn row(&self, row: usize) -> Option<&[Cell]> {
        self.cells.get(row).map(|r| r.as_slice())
    }

    pub fn height(&self) -> usize {
        self.rows
    }

    pub fn width(&self) -> usize {
        self.cols
    }

    pub fn clear_line(&mut self, row: usize) {
        if let Some(line) = self.cells.get_mut(row) {
            for cell in line {
                *cell = Cell::default();
            }
        }
    }

    pub fn clear_all(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = Cell::default();
            }
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        for _ in 0..lines {
            self.cells.remove(0);
            self.cells.push(vec![Cell::default(); self.cols]);
        }
    }
}

pub struct TerminalState {
    grid: Grid,
    cursor_row: usize,
    cursor_col: usize,
    current_style: Style,
    saved_cursor: (usize, usize),
}

impl TerminalState {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            grid: Grid::new(rows, cols),
            cursor_row: 0,
            cursor_col: 0,
            current_style: Style::default(),
            saved_cursor: (0, 0),
        }
    }

    fn write_char(&mut self, c: char) {
        if self.cursor_row >= self.grid.height() {
            self.cursor_row = self.grid.height() - 1;
            self.grid.scroll_up(1);
        }
        
        if self.cursor_col >= self.grid.width() {
            self.cursor_row += 1;
            self.cursor_col = 0;
            if self.cursor_row >= self.grid.height() {
                self.cursor_row = self.grid.height() - 1;
                self.grid.scroll_up(1);
            }
        }

        // Store style values to avoid borrow checker issues
        let fg_color = self.ratatui_style_to_color(self.current_style.fg);
        let bg_color = self.ratatui_style_to_color(self.current_style.bg);
        let mut flags = CellFlags::empty();
        if self.current_style.add_modifier.contains(Modifier::BOLD) {
            flags |= CellFlags::BOLD;
        }
        if self.current_style.add_modifier.contains(Modifier::ITALIC) {
            flags |= CellFlags::ITALIC;
        }
        if self.current_style.add_modifier.contains(Modifier::UNDERLINED) {
            flags |= CellFlags::UNDERLINE;
        }
        if self.current_style.add_modifier.contains(Modifier::REVERSED) {
            flags |= CellFlags::INVERSE;
        }

        if let Some(cell) = self.grid.cell_mut(self.cursor_row, self.cursor_col) {
            cell.c = c;
            cell.fg = fg_color;
            cell.bg = bg_color;
            cell.flags = flags;
        }
        
        self.cursor_col += 1;
    }

    fn ratatui_style_to_color(&self, color: Option<Color>) -> Color {
        color.unwrap_or(Color::Reset)
    }
}

impl Perform for TerminalState {
    fn print(&mut self, c: char) {
        self.write_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                // Line feed
                self.cursor_row += 1;
                if self.cursor_row >= self.grid.height() {
                    self.cursor_row = self.grid.height() - 1;
                    self.grid.scroll_up(1);
                }
            }
            b'\r' => {
                // Carriage return
                self.cursor_col = 0;
            }
            b'\t' => {
                // Tab
                let tab_stop = 8;
                let spaces = tab_stop - (self.cursor_col % tab_stop);
                for _ in 0..spaces {
                    self.write_char(' ');
                }
            }
            b'\x08' => {
                // Backspace
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _c: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, c: char) {
        match c {
            'A' => {
                // Cursor up
                let lines = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&1);
                self.cursor_row = self.cursor_row.saturating_sub(*lines as usize);
            }
            'B' => {
                // Cursor down
                let lines = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&1);
                self.cursor_row = (self.cursor_row + *lines as usize).min(self.grid.height() - 1);
            }
            'C' => {
                // Cursor forward
                let cols = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&1);
                self.cursor_col = (self.cursor_col + *cols as usize).min(self.grid.width() - 1);
            }
            'D' => {
                // Cursor backward
                let cols = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&1);
                self.cursor_col = self.cursor_col.saturating_sub(*cols as usize);
            }
            'H' => {
                // Cursor position
                let mut iter = params.iter();
                let row = iter.next().and_then(|p| p.get(0)).unwrap_or(&1).saturating_sub(1) as usize;
                let col = iter.next().and_then(|p| p.get(0)).unwrap_or(&1).saturating_sub(1) as usize;
                self.cursor_row = row.min(self.grid.height() - 1);
                self.cursor_col = col.min(self.grid.width() - 1);
            }
            'J' => {
                // Erase display
                let mode = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&0);
                match mode {
                    0 => {
                        // Clear from cursor to end of screen
                        for col in self.cursor_col..self.grid.width() {
                            if let Some(cell) = self.grid.cell_mut(self.cursor_row, col) {
                                *cell = Cell::default();
                            }
                        }
                        for row in (self.cursor_row + 1)..self.grid.height() {
                            self.grid.clear_line(row);
                        }
                    }
                    1 => {
                        // Clear from beginning of screen to cursor
                        for row in 0..self.cursor_row {
                            self.grid.clear_line(row);
                        }
                        for col in 0..=self.cursor_col {
                            if let Some(cell) = self.grid.cell_mut(self.cursor_row, col) {
                                *cell = Cell::default();
                            }
                        }
                    }
                    2 => {
                        // Clear entire screen
                        self.grid.clear_all();
                    }
                    _ => {}
                }
            }
            'K' => {
                // Erase line
                let mode = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&0);
                match mode {
                    0 => {
                        // Clear from cursor to end of line
                        for col in self.cursor_col..self.grid.width() {
                            if let Some(cell) = self.grid.cell_mut(self.cursor_row, col) {
                                *cell = Cell::default();
                            }
                        }
                    }
                    1 => {
                        // Clear from beginning of line to cursor
                        for col in 0..=self.cursor_col {
                            if let Some(cell) = self.grid.cell_mut(self.cursor_row, col) {
                                *cell = Cell::default();
                            }
                        }
                    }
                    2 => {
                        // Clear entire line
                        self.grid.clear_line(self.cursor_row);
                    }
                    _ => {}
                }
            }
            'm' => {
                // Set graphics rendition (colors and styles)
                for param in params.iter() {
                    for &value in param {
                        match value {
                            0 => self.current_style = Style::default(),
                            1 => self.current_style = self.current_style.add_modifier(Modifier::BOLD),
                            3 => self.current_style = self.current_style.add_modifier(Modifier::ITALIC),
                            4 => self.current_style = self.current_style.add_modifier(Modifier::UNDERLINED),
                            7 => self.current_style = self.current_style.add_modifier(Modifier::REVERSED),
                            22 => self.current_style = self.current_style.remove_modifier(Modifier::BOLD),
                            23 => self.current_style = self.current_style.remove_modifier(Modifier::ITALIC),
                            24 => self.current_style = self.current_style.remove_modifier(Modifier::UNDERLINED),
                            27 => self.current_style = self.current_style.remove_modifier(Modifier::REVERSED),
                            30..=37 => {
                                let color = match value {
                                    30 => Color::Black,
                                    31 => Color::Red,
                                    32 => Color::Green,
                                    33 => Color::Yellow,
                                    34 => Color::Blue,
                                    35 => Color::Magenta,
                                    36 => Color::Cyan,
                                    37 => Color::White,
                                    _ => Color::Reset,
                                };
                                self.current_style = self.current_style.fg(color);
                            }
                            40..=47 => {
                                let color = match value {
                                    40 => Color::Black,
                                    41 => Color::Red,
                                    42 => Color::Green,
                                    43 => Color::Yellow,
                                    44 => Color::Blue,
                                    45 => Color::Magenta,
                                    46 => Color::Cyan,
                                    47 => Color::White,
                                    _ => Color::Reset,
                                };
                                self.current_style = self.current_style.bg(color);
                            }
                            90..=97 => {
                                let color = match value {
                                    90 => Color::DarkGray,
                                    91 => Color::LightRed,
                                    92 => Color::LightGreen,
                                    93 => Color::LightYellow,
                                    94 => Color::LightBlue,
                                    95 => Color::LightMagenta,
                                    96 => Color::LightCyan,
                                    97 => Color::White,
                                    _ => Color::Reset,
                                };
                                self.current_style = self.current_style.fg(color);
                            }
                            _ => {}
                        }
                    }
                }
            }
            's' => {
                // Save cursor position
                self.saved_cursor = (self.cursor_row, self.cursor_col);
            }
            'u' => {
                // Restore cursor position
                self.cursor_row = self.saved_cursor.0;
                self.cursor_col = self.saved_cursor.1;
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

/// A wrapper around a VTE parser and terminal state.
pub struct VirtualTerminal {
    state: TerminalState,
    parser: Parser,
}

impl VirtualTerminal {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            state: TerminalState::new(rows as usize, cols as usize),
            parser: Parser::new(),
        }
    }

    /// Process a byte stream from the PTY and update the terminal state.
    pub fn process_bytes(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.state, bytes);
    }
    
    /// Get the visible lines from the grid to be rendered by Ratatui.
    pub fn get_visible_lines(&self) -> Vec<Line> {
        let mut lines = Vec::with_capacity(self.state.grid.height());
        for row_idx in 0..self.state.grid.height() {
            if let Some(row) = self.state.grid.row(row_idx) {
                let mut spans: Vec<Span> = Vec::new();
                
                for cell in row {
                    let style = self.cell_to_ratatui_style(cell);
                    let last_span = spans.last_mut();

                    if let Some(last) = last_span {
                        if last.style == style {
                            last.content.to_mut().push(cell.c);
                            continue;
                        }
                    }
                    spans.push(Span::styled(cell.c.to_string(), style));
                }
                lines.push(Line::from(spans));
            }
        }
        lines
    }

    /// Helper to convert cell style to Ratatui style.
    fn cell_to_ratatui_style(&self, cell: &Cell) -> Style {
        let mut style = Style::default();
        style = style.fg(cell.fg);
        style = style.bg(cell.bg);

        if cell.flags.contains(CellFlags::BOLD) {
            style = style.add_modifier(Modifier::BOLD);
        }
        if cell.flags.contains(CellFlags::ITALIC) {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if cell.flags.contains(CellFlags::UNDERLINE) {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        if cell.flags.contains(CellFlags::INVERSE) {
            style = style.add_modifier(Modifier::REVERSED);
        }

        style
    }
}