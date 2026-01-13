//! Terminal screen buffer

/// Terminal screen
pub struct Screen {
    /// Grid of cells
    cells: Vec<Vec<Cell>>,
    /// Number of rows
    rows: usize,
    /// Number of columns
    cols: usize,
    /// Cursor row
    cursor_row: usize,
    /// Cursor column
    cursor_col: usize,
    /// Cursor visible?
    cursor_visible: bool,
    /// Scrollback buffer
    scrollback: Vec<Vec<Cell>>,
    /// Max scrollback lines
    max_scrollback: usize,
}

/// A single cell in the terminal
#[derive(Debug, Clone)]
pub struct Cell {
    /// Character (empty = space)
    pub char: char,
    /// Cell style
    pub style: CellStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: ' ',
            style: CellStyle::default(),
        }
    }
}

/// Cell styling
#[derive(Debug, Clone, Default)]
pub struct CellStyle {
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub inverse: bool,
}

/// Terminal color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

impl Default for Color {
    fn default() -> Self {
        Self::Default
    }
}

impl Screen {
    /// Create a new screen
    pub fn new(rows: usize, cols: usize) -> Self {
        let cells = vec![vec![Cell::default(); cols]; rows];
        
        Self {
            cells,
            rows,
            cols,
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
            scrollback: Vec::new(),
            max_scrollback: 10000,
        }
    }

    /// Resize the screen
    pub fn resize(&mut self, rows: usize, cols: usize) {
        // Resize existing rows
        for row in &mut self.cells {
            row.resize(cols, Cell::default());
        }
        
        // Add or remove rows
        self.cells.resize(rows, vec![Cell::default(); cols]);
        
        self.rows = rows;
        self.cols = cols;
        
        // Clamp cursor
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    /// Clear the screen
    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = Cell::default();
            }
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    /// Get cell at position
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.cells.get(row).and_then(|r| r.get(col))
    }

    /// Get mutable cell at position
    pub fn cell_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        self.cells.get_mut(row).and_then(|r| r.get_mut(col))
    }

    /// Get a row
    pub fn row(&self, idx: usize) -> Option<&[Cell]> {
        self.cells.get(idx).map(|r| r.as_slice())
    }

    /// Get screen dimensions
    pub fn size(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    /// Get cursor position
    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    /// Is cursor visible?
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Update screen from vt100 parser
    pub fn update_from_vt100(&mut self, vt_screen: &vt100::Screen) {
        // Update dimensions if needed
        let (vt_rows, vt_cols) = vt_screen.size();
        if self.rows != vt_rows as usize || self.cols != vt_cols as usize {
            self.resize(vt_rows as usize, vt_cols as usize);
        }

        // Copy cells
        for row in 0..self.rows {
            for col in 0..self.cols {
                let vt_cell = vt_screen.cell(row as u16, col as u16);
                if let Some(vt_cell) = vt_cell {
                    if let Some(cell) = self.cell_mut(row, col) {
                        cell.char = vt_cell.contents().chars().next().unwrap_or(' ');
                        
                        // Copy styles
                        cell.style.bold = vt_cell.bold();
                        cell.style.italic = vt_cell.italic();
                        cell.style.underline = vt_cell.underline();
                        cell.style.inverse = vt_cell.inverse();
                        
                        // Copy colors
                        cell.style.foreground = convert_vt_color(vt_cell.fgcolor());
                        cell.style.background = convert_vt_color(vt_cell.bgcolor());
                    }
                }
            }
        }

        // Update cursor
        let (cursor_row, cursor_col) = vt_screen.cursor_position();
        self.cursor_row = cursor_row as usize;
        self.cursor_col = cursor_col as usize;
        self.cursor_visible = !vt_screen.hide_cursor();
    }

    /// Get row as string
    pub fn row_text(&self, idx: usize) -> String {
        self.cells.get(idx)
            .map(|row| row.iter().map(|c| c.char).collect())
            .unwrap_or_default()
    }

    /// Get all text content
    pub fn text(&self) -> String {
        self.cells
            .iter()
            .map(|row| row.iter().map(|c| c.char).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get scrollback line count
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Get scrollback line
    pub fn scrollback_line(&self, idx: usize) -> Option<&[Cell]> {
        self.scrollback.get(idx).map(|r| r.as_slice())
    }
}

fn convert_vt_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Default,
        vt100::Color::Idx(0) => Color::Black,
        vt100::Color::Idx(1) => Color::Red,
        vt100::Color::Idx(2) => Color::Green,
        vt100::Color::Idx(3) => Color::Yellow,
        vt100::Color::Idx(4) => Color::Blue,
        vt100::Color::Idx(5) => Color::Magenta,
        vt100::Color::Idx(6) => Color::Cyan,
        vt100::Color::Idx(7) => Color::White,
        vt100::Color::Idx(8) => Color::BrightBlack,
        vt100::Color::Idx(9) => Color::BrightRed,
        vt100::Color::Idx(10) => Color::BrightGreen,
        vt100::Color::Idx(11) => Color::BrightYellow,
        vt100::Color::Idx(12) => Color::BrightBlue,
        vt100::Color::Idx(13) => Color::BrightMagenta,
        vt100::Color::Idx(14) => Color::BrightCyan,
        vt100::Color::Idx(15) => Color::BrightWhite,
        vt100::Color::Idx(n) => Color::Indexed(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
