/* src/terminal.rs */

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use vte::{Parser, Perform};

const SCROLLBACK_BUFFER_SIZE: usize = 500;

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
            self.cells.rotate_left(1);
            if let Some(last_row) = self.cells.last_mut() {
                for cell in last_row {
                    *cell = Cell::default();
                }
            }
        }
    }
}

pub struct TerminalState {
    grid: Grid,
    cursor_row: usize,
    cursor_col: usize,
    content_bottom_row: usize,
    current_style: Style,
    saved_cursor: (usize, usize),
}

impl TerminalState {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            grid: Grid::new(rows, cols),
            cursor_row: 0,
            cursor_col: 0,
            content_bottom_row: 0,
            current_style: Style::default(),
            saved_cursor: (0, 0),
        }
    }

    fn update_content_bottom(&mut self) {
        self.content_bottom_row = self.content_bottom_row.max(self.cursor_row);
    }

    fn write_char(&mut self, c: char) {
        if self.cursor_col >= self.grid.width() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }

        if self.cursor_row >= self.grid.height() {
            let scroll_count = self.cursor_row - self.grid.height() + 1;
            self.grid.scroll_up(scroll_count);
            self.cursor_row = self.grid.height() - 1;
        }

        self.update_content_bottom();

        let fg_color = self.ratatui_style_to_color(self.current_style.fg);
        let bg_color = self.ratatui_style_to_color(self.current_style.bg);
        let mut flags = CellFlags::empty();
        if self.current_style.add_modifier.contains(Modifier::BOLD) {
            flags |= CellFlags::BOLD;
        }
        if self.current_style.add_modifier.contains(Modifier::ITALIC) {
            flags |= CellFlags::ITALIC;
        }
        if self
            .current_style
            .add_modifier
            .contains(Modifier::UNDERLINED)
        {
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
                self.cursor_row += 1;
                if self.cursor_row >= self.grid.height() {
                    self.grid.scroll_up(1);
                    self.cursor_row = self.grid.height() - 1;
                }
                self.update_content_bottom();
            }
            b'\r' => self.cursor_col = 0,
            b'\t' => {
                let tab_stop = 8;
                let spaces = tab_stop - (self.cursor_col % tab_stop);
                for _ in 0..spaces {
                    self.write_char(' ');
                }
            }
            b'\x08' => {
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
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        c: char,
    ) {
        match c {
            'A' => {
                let lines = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&1);
                self.cursor_row = self.cursor_row.saturating_sub(*lines as usize);
            }
            'B' => {
                let lines = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&1);
                let max_row = self.grid.height() - 1;
                self.cursor_row = (self.cursor_row + *lines as usize).min(max_row);
                self.update_content_bottom();
            }
            'C' => {
                let cols = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&1);
                let max_col = self.grid.width() - 1;
                self.cursor_col = (self.cursor_col + *cols as usize).min(max_col);
            }
            'D' => {
                let cols = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&1);
                self.cursor_col = self.cursor_col.saturating_sub(*cols as usize);
            }
            'H' => {
                let mut iter = params.iter();
                let row = iter
                    .next()
                    .and_then(|p| p.get(0))
                    .unwrap_or(&1)
                    .saturating_sub(1) as usize;
                let col = iter
                    .next()
                    .and_then(|p| p.get(0))
                    .unwrap_or(&1)
                    .saturating_sub(1) as usize;
                self.cursor_row = row.min(self.grid.height() - 1);
                self.cursor_col = col.min(self.grid.width() - 1);
                self.update_content_bottom();
            }
            'J' => {
                let mode = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&0);
                match mode {
                    0 => {
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
                        for row in 0..self.cursor_row {
                            self.grid.clear_line(row);
                        }
                        for col in 0..=self.cursor_col {
                            if let Some(cell) = self.grid.cell_mut(self.cursor_row, col) {
                                *cell = Cell::default();
                            }
                        }
                    }
                    2 => self.grid.clear_all(),
                    _ => {}
                }
            }
            'K' => {
                let mode = params.iter().next().and_then(|p| p.get(0)).unwrap_or(&0);
                match mode {
                    0 => {
                        for col in self.cursor_col..self.grid.width() {
                            if let Some(cell) = self.grid.cell_mut(self.cursor_row, col) {
                                *cell = Cell::default();
                            }
                        }
                    }
                    1 => {
                        for col in 0..=self.cursor_col {
                            if let Some(cell) = self.grid.cell_mut(self.cursor_row, col) {
                                *cell = Cell::default();
                            }
                        }
                    }
                    2 => self.grid.clear_line(self.cursor_row),
                    _ => {}
                }
            }
            'm' => {
                for param in params.iter() {
                    for &value in param {
                        match value {
                            0 => self.current_style = Style::default(),
                            1 => {
                                self.current_style = self.current_style.add_modifier(Modifier::BOLD)
                            }
                            3 => {
                                self.current_style =
                                    self.current_style.add_modifier(Modifier::ITALIC)
                            }
                            4 => {
                                self.current_style =
                                    self.current_style.add_modifier(Modifier::UNDERLINED)
                            }
                            7 => {
                                self.current_style =
                                    self.current_style.add_modifier(Modifier::REVERSED)
                            }
                            22 => {
                                self.current_style =
                                    self.current_style.remove_modifier(Modifier::BOLD)
                            }
                            23 => {
                                self.current_style =
                                    self.current_style.remove_modifier(Modifier::ITALIC)
                            }
                            24 => {
                                self.current_style =
                                    self.current_style.remove_modifier(Modifier::UNDERLINED)
                            }
                            27 => {
                                self.current_style =
                                    self.current_style.remove_modifier(Modifier::REVERSED)
                            }
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
            's' => self.saved_cursor = (self.cursor_row, self.cursor_col),
            'u' => {
                self.cursor_row = self.saved_cursor.0;
                self.cursor_col = self.saved_cursor.1;
                self.update_content_bottom();
            }
            _ => {}
        }
    }
}

pub struct VirtualTerminal {
    state: TerminalState,
    parser: Parser,
    visible_rows: u16,
    scroll_offset: usize,
}

impl VirtualTerminal {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            state: TerminalState::new(SCROLLBACK_BUFFER_SIZE, cols as usize),
            parser: Parser::new(),
            visible_rows: rows,
            scroll_offset: 0,
        }
    }

    pub fn clear(&mut self) {
        self.state.grid.clear_all();
        self.state.cursor_row = 0;
        self.state.cursor_col = 0;
        self.state.content_bottom_row = 0;
        self.scroll_offset = 0;
    }

    pub fn process_bytes(&mut self, bytes: &[u8]) {
        self.scroll_offset = 0;
        self.parser.advance(&mut self.state, bytes);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let max_scroll = self
            .state
            .content_bottom_row
            .saturating_sub(self.visible_rows.saturating_sub(1) as usize);
        self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn get_visible_lines(&self) -> Vec<Line<'_>> {
        let mut lines = Vec::with_capacity(self.visible_rows as usize);

        let viewport_bottom = self
            .state
            .content_bottom_row
            .saturating_sub(self.scroll_offset);
        let viewport_top =
            viewport_bottom.saturating_sub(self.visible_rows.saturating_sub(1) as usize);

        for row_idx in viewport_top..=viewport_bottom {
            if let Some(row) = self.state.grid.row(row_idx) {
                let mut spans: Vec<Span> = Vec::new();
                for cell in row {
                    let style = self.cell_to_ratatui_style(cell);
                    if let Some(last) = spans.last_mut() {
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

    pub fn get_cursor_position(&self) -> Option<(u16, u16)> {
        let viewport_bottom = self
            .state
            .content_bottom_row
            .saturating_sub(self.scroll_offset);
        let viewport_top =
            viewport_bottom.saturating_sub(self.visible_rows.saturating_sub(1) as usize);

        if self.state.cursor_row >= viewport_top && self.state.cursor_row <= viewport_bottom {
            let relative_y = self.state.cursor_row - viewport_top;
            Some((self.state.cursor_col as u16, relative_y as u16))
        } else {
            None
        }
    }

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
