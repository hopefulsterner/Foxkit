//! VT100/xterm terminal escape sequence parser and state machine.
//!
//! This module implements a comprehensive terminal emulator that handles:
//! - CSI (Control Sequence Introducer) sequences
//! - OSC (Operating System Command) sequences
//! - SGR (Select Graphic Rendition) for colors/styles
//! - Cursor movement and positioning
//! - Screen manipulation (scrolling, clearing, etc.)

use crate::screen::{Cell, CellStyle, Color, Screen};
use std::collections::VecDeque;

/// Terminal emulator state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserState {
    /// Normal text processing.
    Ground,
    /// Escape sequence started (ESC received).
    Escape,
    /// ESC [ received, collecting CSI parameters.
    CsiEntry,
    /// Collecting CSI parameters.
    CsiParam,
    /// Collecting CSI intermediate bytes.
    CsiIntermediate,
    /// OSC string started.
    OscString,
    /// DCS (Device Control String) entry.
    DcsEntry,
    /// DCS parameter collection.
    DcsParam,
    /// DCS passthrough mode.
    DcsPassthrough,
    /// Collecting UTF-8 continuation bytes.
    Utf8,
}

/// CSI command parsed from escape sequence.
#[derive(Debug, Clone)]
pub struct CsiCommand {
    /// Parameter bytes (digits and semicolons).
    pub params: Vec<u16>,
    /// Intermediate bytes (0x20-0x2F range).
    pub intermediates: Vec<u8>,
    /// Final byte that identifies the command.
    pub final_byte: u8,
    /// Whether this is a private sequence (starts with ?).
    pub private: bool,
}

impl CsiCommand {
    /// Create a new CSI command.
    pub fn new() -> Self {
        Self {
            params: Vec::new(),
            intermediates: Vec::new(),
            final_byte: 0,
            private: false,
        }
    }

    /// Get parameter at index with default value.
    pub fn param(&self, index: usize, default: u16) -> u16 {
        self.params.get(index).copied().unwrap_or(default)
    }

    /// Get first parameter with default of 1.
    pub fn param1(&self) -> u16 {
        self.param(0, 1)
    }

    /// Get second parameter with default of 1.
    pub fn param2(&self) -> u16 {
        self.param(1, 1)
    }
}

impl Default for CsiCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// OSC (Operating System Command) parsed data.
#[derive(Debug, Clone)]
pub struct OscCommand {
    /// OSC number/type.
    pub number: u16,
    /// OSC string data.
    pub data: String,
}

/// Terminal cursor state.
#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    /// Column position (0-indexed).
    pub col: usize,
    /// Row position (0-indexed).
    pub row: usize,
    /// Whether cursor is visible.
    pub visible: bool,
    /// Whether cursor blinks.
    pub blinking: bool,
    /// Cursor shape.
    pub shape: CursorShape,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            col: 0,
            row: 0,
            visible: true,
            blinking: true,
            shape: CursorShape::Block,
        }
    }
}

/// Cursor shapes supported by the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    /// Block cursor (full cell).
    Block,
    /// Underline cursor.
    Underline,
    /// Vertical bar cursor.
    Bar,
}

/// Saved cursor state for DECSC/DECRC.
#[derive(Debug, Clone)]
pub struct SavedCursor {
    pub cursor: Cursor,
    pub style: CellStyle,
    pub origin_mode: bool,
    pub autowrap: bool,
}

/// Terminal modes that can be set/reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalMode {
    /// Cursor keys mode (DECCKM).
    CursorKeys,
    /// Origin mode (DECOM).
    Origin,
    /// Auto-wrap mode (DECAWM).
    AutoWrap,
    /// Show cursor (DECTCEM).
    ShowCursor,
    /// Mouse tracking modes.
    MouseX10,
    MouseVt200,
    MouseBtnEvent,
    MouseAnyEvent,
    MouseFocus,
    MouseUtf8,
    MouseSgr,
    /// Alternate screen buffer.
    AlternateScreen,
    /// Bracketed paste mode.
    BracketedPaste,
    /// Application keypad mode.
    AppKeypad,
    /// Line feed/new line mode.
    LineFeedNewLine,
}

/// Events emitted by the terminal emulator.
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    /// Title changed (OSC 0 or 2).
    TitleChanged(String),
    /// Icon name changed (OSC 1).
    IconNameChanged(String),
    /// Bell/alert triggered.
    Bell,
    /// Clipboard set request (OSC 52).
    ClipboardSet { clipboard: String, data: String },
    /// Hyperlink detected (OSC 8).
    Hyperlink { url: Option<String>, id: Option<String> },
    /// Color query response needed.
    ColorQuery { index: u8 },
    /// Cursor position report requested.
    CursorPositionReport,
    /// Device attributes query.
    DeviceAttributes,
    /// Terminal resize request.
    ResizeRequest { cols: u16, rows: u16 },
}

/// VT100/xterm terminal emulator.
pub struct TerminalEmulator {
    /// Current parser state.
    state: ParserState,
    /// Screen buffer.
    screen: Screen,
    /// Alternate screen buffer.
    alt_screen: Option<Screen>,
    /// Cursor state.
    cursor: Cursor,
    /// Saved cursor (primary screen).
    saved_cursor: Option<SavedCursor>,
    /// Saved cursor (alternate screen).
    saved_cursor_alt: Option<SavedCursor>,
    /// Current cell style for new characters.
    current_style: CellStyle,
    /// Active terminal modes.
    modes: std::collections::HashSet<TerminalMode>,
    /// Scroll region top.
    scroll_top: usize,
    /// Scroll region bottom.
    scroll_bottom: usize,
    /// Tab stops.
    tab_stops: Vec<usize>,
    /// Current CSI command being parsed.
    csi_command: CsiCommand,
    /// Current OSC string being collected.
    osc_string: String,
    /// OSC number.
    osc_number: u16,
    /// UTF-8 buffer for multi-byte sequences.
    utf8_buffer: Vec<u8>,
    /// UTF-8 bytes remaining.
    utf8_remaining: u8,
    /// Pending events to be processed.
    events: VecDeque<TerminalEvent>,
    /// Terminal width in columns.
    cols: usize,
    /// Terminal height in rows.
    rows: usize,
}

impl TerminalEmulator {
    /// Create a new terminal emulator with specified dimensions.
    pub fn new(cols: usize, rows: usize) -> Self {
        let mut modes = std::collections::HashSet::new();
        modes.insert(TerminalMode::AutoWrap);
        modes.insert(TerminalMode::ShowCursor);

        let mut tab_stops = Vec::new();
        for i in (8..cols).step_by(8) {
            tab_stops.push(i);
        }

        Self {
            state: ParserState::Ground,
            screen: Screen::new(cols, rows),
            alt_screen: None,
            cursor: Cursor::default(),
            saved_cursor: None,
            saved_cursor_alt: None,
            current_style: CellStyle::default(),
            modes,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            tab_stops,
            csi_command: CsiCommand::new(),
            osc_string: String::new(),
            osc_number: 0,
            utf8_buffer: Vec::with_capacity(4),
            utf8_remaining: 0,
            events: VecDeque::new(),
            cols,
            rows,
        }
    }

    /// Process input bytes through the terminal emulator.
    pub fn process(&mut self, data: &[u8]) {
        for &byte in data {
            self.process_byte(byte);
        }
    }

    /// Process a single byte.
    fn process_byte(&mut self, byte: u8) {
        match self.state {
            ParserState::Ground => self.handle_ground(byte),
            ParserState::Escape => self.handle_escape(byte),
            ParserState::CsiEntry => self.handle_csi_entry(byte),
            ParserState::CsiParam => self.handle_csi_param(byte),
            ParserState::CsiIntermediate => self.handle_csi_intermediate(byte),
            ParserState::OscString => self.handle_osc_string(byte),
            ParserState::Utf8 => self.handle_utf8(byte),
            ParserState::DcsEntry => self.handle_dcs_entry(byte),
            ParserState::DcsParam => self.handle_dcs_param(byte),
            ParserState::DcsPassthrough => self.handle_dcs_passthrough(byte),
        }
    }

    /// Handle bytes in ground state.
    fn handle_ground(&mut self, byte: u8) {
        match byte {
            // C0 control characters
            0x00 => {} // NUL - ignore
            0x07 => self.events.push_back(TerminalEvent::Bell),
            0x08 => self.cursor_back(1), // BS - backspace
            0x09 => self.tab(),           // HT - horizontal tab
            0x0A | 0x0B | 0x0C => self.line_feed(), // LF, VT, FF
            0x0D => self.carriage_return(), // CR
            0x0E => {} // SO - shift out (ignore)
            0x0F => {} // SI - shift in (ignore)
            0x1B => self.state = ParserState::Escape, // ESC
            
            // Printable ASCII
            0x20..=0x7E => self.print_char(byte as char),
            
            // Delete
            0x7F => {} // DEL - ignore
            
            // UTF-8 start bytes
            0xC0..=0xDF => {
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = 1;
                self.state = ParserState::Utf8;
            }
            0xE0..=0xEF => {
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = 2;
                self.state = ParserState::Utf8;
            }
            0xF0..=0xF7 => {
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = 3;
                self.state = ParserState::Utf8;
            }
            
            // C1 control characters (8-bit)
            0x84 => self.index(),     // IND
            0x85 => self.next_line(), // NEL
            0x88 => self.set_tab(),   // HTS
            0x8D => self.reverse_index(), // RI
            0x9B => self.state = ParserState::CsiEntry, // CSI
            0x9D => {
                self.osc_string.clear();
                self.osc_number = 0;
                self.state = ParserState::OscString;
            }
            
            _ => {} // Ignore other bytes
        }
    }

    /// Handle bytes after ESC.
    fn handle_escape(&mut self, byte: u8) {
        match byte {
            b'[' => {
                self.csi_command = CsiCommand::new();
                self.state = ParserState::CsiEntry;
            }
            b']' => {
                self.osc_string.clear();
                self.osc_number = 0;
                self.state = ParserState::OscString;
            }
            b'P' => self.state = ParserState::DcsEntry,
            b'7' => self.save_cursor(),  // DECSC
            b'8' => self.restore_cursor(), // DECRC
            b'D' => {
                self.index();
                self.state = ParserState::Ground;
            }
            b'E' => {
                self.next_line();
                self.state = ParserState::Ground;
            }
            b'H' => {
                self.set_tab();
                self.state = ParserState::Ground;
            }
            b'M' => {
                self.reverse_index();
                self.state = ParserState::Ground;
            }
            b'c' => {
                self.reset();
                self.state = ParserState::Ground;
            }
            b'=' => {
                self.modes.insert(TerminalMode::AppKeypad);
                self.state = ParserState::Ground;
            }
            b'>' => {
                self.modes.remove(&TerminalMode::AppKeypad);
                self.state = ParserState::Ground;
            }
            _ => self.state = ParserState::Ground,
        }
    }

    /// Handle CSI entry state.
    fn handle_csi_entry(&mut self, byte: u8) {
        match byte {
            b'?' => {
                self.csi_command.private = true;
                self.state = ParserState::CsiParam;
            }
            b'0'..=b'9' => {
                self.csi_command.params.push((byte - b'0') as u16);
                self.state = ParserState::CsiParam;
            }
            b';' => {
                self.csi_command.params.push(0);
                self.state = ParserState::CsiParam;
            }
            0x20..=0x2F => {
                self.csi_command.intermediates.push(byte);
                self.state = ParserState::CsiIntermediate;
            }
            0x40..=0x7E => {
                self.csi_command.final_byte = byte;
                self.execute_csi();
                self.state = ParserState::Ground;
            }
            _ => self.state = ParserState::Ground,
        }
    }

    /// Handle CSI parameter state.
    fn handle_csi_param(&mut self, byte: u8) {
        match byte {
            b'0'..=b'9' => {
                if let Some(last) = self.csi_command.params.last_mut() {
                    *last = last.saturating_mul(10).saturating_add((byte - b'0') as u16);
                } else {
                    self.csi_command.params.push((byte - b'0') as u16);
                }
            }
            b';' | b':' => {
                self.csi_command.params.push(0);
            }
            0x20..=0x2F => {
                self.csi_command.intermediates.push(byte);
                self.state = ParserState::CsiIntermediate;
            }
            0x40..=0x7E => {
                self.csi_command.final_byte = byte;
                self.execute_csi();
                self.state = ParserState::Ground;
            }
            _ => self.state = ParserState::Ground,
        }
    }

    /// Handle CSI intermediate state.
    fn handle_csi_intermediate(&mut self, byte: u8) {
        match byte {
            0x20..=0x2F => {
                self.csi_command.intermediates.push(byte);
            }
            0x40..=0x7E => {
                self.csi_command.final_byte = byte;
                self.execute_csi();
                self.state = ParserState::Ground;
            }
            _ => self.state = ParserState::Ground,
        }
    }

    /// Handle OSC string collection.
    fn handle_osc_string(&mut self, byte: u8) {
        match byte {
            0x07 | 0x9C => {
                // ST (String Terminator)
                self.execute_osc();
                self.state = ParserState::Ground;
            }
            0x1B => {
                // Could be ESC \ (ST)
                // For simplicity, treat as end of OSC
                self.execute_osc();
                self.state = ParserState::Escape;
            }
            b';' if self.osc_string.is_empty() => {
                // Separator between number and data
                // osc_number is already set
            }
            b'0'..=b'9' if self.osc_string.is_empty() => {
                self.osc_number = self.osc_number * 10 + (byte - b'0') as u16;
            }
            _ => {
                if byte >= 0x20 {
                    self.osc_string.push(byte as char);
                }
            }
        }
    }

    /// Handle UTF-8 continuation bytes.
    fn handle_utf8(&mut self, byte: u8) {
        if byte & 0xC0 == 0x80 {
            self.utf8_buffer.push(byte);
            self.utf8_remaining -= 1;
            if self.utf8_remaining == 0 {
                if let Ok(s) = std::str::from_utf8(&self.utf8_buffer) {
                    for c in s.chars() {
                        self.print_char(c);
                    }
                }
                self.state = ParserState::Ground;
            }
        } else {
            // Invalid UTF-8, reset
            self.state = ParserState::Ground;
        }
    }

    /// Handle DCS entry.
    fn handle_dcs_entry(&mut self, byte: u8) {
        match byte {
            b'0'..=b'9' | b';' => self.state = ParserState::DcsParam,
            0x40..=0x7E => self.state = ParserState::DcsPassthrough,
            _ => self.state = ParserState::Ground,
        }
    }

    /// Handle DCS parameters.
    fn handle_dcs_param(&mut self, byte: u8) {
        match byte {
            0x40..=0x7E => self.state = ParserState::DcsPassthrough,
            0x1B => self.state = ParserState::Escape,
            _ => {}
        }
    }

    /// Handle DCS passthrough.
    fn handle_dcs_passthrough(&mut self, byte: u8) {
        if byte == 0x1B || byte == 0x9C {
            self.state = ParserState::Ground;
        }
    }

    /// Execute a CSI command.
    fn execute_csi(&mut self) {
        let cmd = &self.csi_command;
        
        if cmd.private {
            self.execute_private_csi();
            return;
        }

        match cmd.final_byte {
            b'@' => self.insert_chars(cmd.param1() as usize),
            b'A' => self.cursor_up(cmd.param1() as usize),
            b'B' => self.cursor_down(cmd.param1() as usize),
            b'C' => self.cursor_forward(cmd.param1() as usize),
            b'D' => self.cursor_back(cmd.param1() as usize),
            b'E' => self.cursor_next_line(cmd.param1() as usize),
            b'F' => self.cursor_prev_line(cmd.param1() as usize),
            b'G' => self.cursor_col(cmd.param1() as usize),
            b'H' | b'f' => self.cursor_position(cmd.param1() as usize, cmd.param2() as usize),
            b'J' => self.erase_display(cmd.param(0, 0)),
            b'K' => self.erase_line(cmd.param(0, 0)),
            b'L' => self.insert_lines(cmd.param1() as usize),
            b'M' => self.delete_lines(cmd.param1() as usize),
            b'P' => self.delete_chars(cmd.param1() as usize),
            b'S' => self.scroll_up(cmd.param1() as usize),
            b'T' => self.scroll_down(cmd.param1() as usize),
            b'X' => self.erase_chars(cmd.param1() as usize),
            b'd' => self.cursor_row(cmd.param1() as usize),
            b'g' => self.clear_tabs(cmd.param(0, 0)),
            b'm' => self.execute_sgr(),
            b'n' => self.device_status_report(cmd.param(0, 0)),
            b'r' => self.set_scroll_region(cmd.param(0, 1) as usize, cmd.param(1, self.rows as u16) as usize),
            b's' => self.save_cursor(),
            b'u' => self.restore_cursor(),
            b't' => self.execute_window_manipulation(),
            _ => {} // Ignore unknown sequences
        }
    }

    /// Execute private CSI sequences (those starting with ?).
    fn execute_private_csi(&mut self) {
        let cmd = &self.csi_command;
        let set = cmd.final_byte == b'h';
        
        for &param in &cmd.params {
            match param {
                1 => self.set_mode(TerminalMode::CursorKeys, set),
                6 => self.set_mode(TerminalMode::Origin, set),
                7 => self.set_mode(TerminalMode::AutoWrap, set),
                12 => self.cursor.blinking = set,
                25 => self.set_mode(TerminalMode::ShowCursor, set),
                9 => self.set_mode(TerminalMode::MouseX10, set),
                1000 => self.set_mode(TerminalMode::MouseVt200, set),
                1002 => self.set_mode(TerminalMode::MouseBtnEvent, set),
                1003 => self.set_mode(TerminalMode::MouseAnyEvent, set),
                1004 => self.set_mode(TerminalMode::MouseFocus, set),
                1005 => self.set_mode(TerminalMode::MouseUtf8, set),
                1006 => self.set_mode(TerminalMode::MouseSgr, set),
                1049 => self.set_alternate_screen(set),
                2004 => self.set_mode(TerminalMode::BracketedPaste, set),
                _ => {}
            }
        }
    }

    /// Execute SGR (Select Graphic Rendition) sequence.
    fn execute_sgr(&mut self) {
        let params = &self.csi_command.params;
        if params.is_empty() {
            self.current_style = CellStyle::default();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.current_style = CellStyle::default(),
                1 => self.current_style.bold = true,
                2 => self.current_style.dim = true,
                3 => self.current_style.italic = true,
                4 => self.current_style.underline = true,
                5 | 6 => self.current_style.blink = true,
                7 => self.current_style.reverse = true,
                8 => self.current_style.hidden = true,
                9 => self.current_style.strikethrough = true,
                21 => self.current_style.bold = false,
                22 => {
                    self.current_style.bold = false;
                    self.current_style.dim = false;
                }
                23 => self.current_style.italic = false,
                24 => self.current_style.underline = false,
                25 => self.current_style.blink = false,
                27 => self.current_style.reverse = false,
                28 => self.current_style.hidden = false,
                29 => self.current_style.strikethrough = false,
                
                // Foreground colors
                30..=37 => self.current_style.fg = Color::Indexed((params[i] - 30) as u8),
                38 => {
                    if let Some(color) = self.parse_extended_color(&params[i..]) {
                        self.current_style.fg = color;
                        i += self.extended_color_params(&params[i..]);
                    }
                }
                39 => self.current_style.fg = Color::Default,
                
                // Background colors
                40..=47 => self.current_style.bg = Color::Indexed((params[i] - 40) as u8),
                48 => {
                    if let Some(color) = self.parse_extended_color(&params[i..]) {
                        self.current_style.bg = color;
                        i += self.extended_color_params(&params[i..]);
                    }
                }
                49 => self.current_style.bg = Color::Default,
                
                // Bright foreground colors
                90..=97 => self.current_style.fg = Color::Indexed((params[i] - 90 + 8) as u8),
                
                // Bright background colors
                100..=107 => self.current_style.bg = Color::Indexed((params[i] - 100 + 8) as u8),
                
                _ => {}
            }
            i += 1;
        }
    }

    /// Parse extended color (256 or RGB).
    fn parse_extended_color(&self, params: &[u16]) -> Option<Color> {
        if params.len() < 2 {
            return None;
        }
        match params[1] {
            5 if params.len() >= 3 => Some(Color::Indexed(params[2] as u8)),
            2 if params.len() >= 5 => Some(Color::Rgb(
                params[2] as u8,
                params[3] as u8,
                params[4] as u8,
            )),
            _ => None,
        }
    }

    /// Get number of params consumed by extended color.
    fn extended_color_params(&self, params: &[u16]) -> usize {
        if params.len() < 2 {
            return 0;
        }
        match params[1] {
            5 => 2, // 38;5;N
            2 => 4, // 38;2;R;G;B
            _ => 0,
        }
    }

    /// Execute OSC command.
    fn execute_osc(&mut self) {
        match self.osc_number {
            0 => {
                // Set icon name and window title
                self.events.push_back(TerminalEvent::TitleChanged(self.osc_string.clone()));
                self.events.push_back(TerminalEvent::IconNameChanged(self.osc_string.clone()));
            }
            1 => {
                // Set icon name
                self.events.push_back(TerminalEvent::IconNameChanged(self.osc_string.clone()));
            }
            2 => {
                // Set window title
                self.events.push_back(TerminalEvent::TitleChanged(self.osc_string.clone()));
            }
            8 => {
                // Hyperlink: OSC 8 ; params ; url ST
                let parts: Vec<&str> = self.osc_string.splitn(2, ';').collect();
                let (id, url) = if parts.len() == 2 {
                    let params_str = parts[0];
                    let url = parts[1];
                    let id = params_str
                        .split(':')
                        .find_map(|p| p.strip_prefix("id="))
                        .map(|s| s.to_string());
                    (id, if url.is_empty() { None } else { Some(url.to_string()) })
                } else {
                    (None, None)
                };
                self.events.push_back(TerminalEvent::Hyperlink { url, id });
            }
            52 => {
                // Clipboard: OSC 52 ; clipboard ; base64-data ST
                let parts: Vec<&str> = self.osc_string.splitn(2, ';').collect();
                if parts.len() == 2 {
                    self.events.push_back(TerminalEvent::ClipboardSet {
                        clipboard: parts[0].to_string(),
                        data: parts[1].to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    /// Execute window manipulation sequence.
    fn execute_window_manipulation(&mut self) {
        let cmd = &self.csi_command;
        match cmd.param(0, 0) {
            8 => {
                // Resize terminal
                let rows = cmd.param(1, 0);
                let cols = cmd.param(2, 0);
                if rows > 0 && cols > 0 {
                    self.events.push_back(TerminalEvent::ResizeRequest { cols, rows });
                }
            }
            _ => {}
        }
    }

    // Cursor movement methods
    fn cursor_up(&mut self, n: usize) {
        let top = if self.modes.contains(&TerminalMode::Origin) {
            self.scroll_top
        } else {
            0
        };
        self.cursor.row = self.cursor.row.saturating_sub(n).max(top);
    }

    fn cursor_down(&mut self, n: usize) {
        let bottom = if self.modes.contains(&TerminalMode::Origin) {
            self.scroll_bottom
        } else {
            self.rows - 1
        };
        self.cursor.row = (self.cursor.row + n).min(bottom);
    }

    fn cursor_forward(&mut self, n: usize) {
        self.cursor.col = (self.cursor.col + n).min(self.cols - 1);
    }

    fn cursor_back(&mut self, n: usize) {
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    fn cursor_next_line(&mut self, n: usize) {
        self.cursor_down(n);
        self.cursor.col = 0;
    }

    fn cursor_prev_line(&mut self, n: usize) {
        self.cursor_up(n);
        self.cursor.col = 0;
    }

    fn cursor_col(&mut self, n: usize) {
        self.cursor.col = (n.saturating_sub(1)).min(self.cols - 1);
    }

    fn cursor_row(&mut self, n: usize) {
        let offset = if self.modes.contains(&TerminalMode::Origin) {
            self.scroll_top
        } else {
            0
        };
        self.cursor.row = (offset + n.saturating_sub(1)).min(self.rows - 1);
    }

    fn cursor_position(&mut self, row: usize, col: usize) {
        self.cursor_row(row);
        self.cursor_col(col);
    }

    // Screen manipulation methods
    fn erase_display(&mut self, mode: u16) {
        match mode {
            0 => {
                // Erase from cursor to end of display
                self.erase_line(0);
                for row in (self.cursor.row + 1)..self.rows {
                    self.screen.clear_row(row);
                }
            }
            1 => {
                // Erase from start of display to cursor
                for row in 0..self.cursor.row {
                    self.screen.clear_row(row);
                }
                self.erase_line(1);
            }
            2 | 3 => {
                // Erase entire display (3 also clears scrollback)
                for row in 0..self.rows {
                    self.screen.clear_row(row);
                }
            }
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: u16) {
        let row = self.cursor.row;
        match mode {
            0 => {
                // Erase from cursor to end of line
                for col in self.cursor.col..self.cols {
                    self.screen.set_cell(row, col, Cell::default());
                }
            }
            1 => {
                // Erase from start of line to cursor
                for col in 0..=self.cursor.col {
                    self.screen.set_cell(row, col, Cell::default());
                }
            }
            2 => {
                // Erase entire line
                self.screen.clear_row(row);
            }
            _ => {}
        }
    }

    fn insert_chars(&mut self, n: usize) {
        self.screen.insert_cells(self.cursor.row, self.cursor.col, n);
    }

    fn delete_chars(&mut self, n: usize) {
        self.screen.delete_cells(self.cursor.row, self.cursor.col, n);
    }

    fn erase_chars(&mut self, n: usize) {
        for i in 0..n {
            let col = self.cursor.col + i;
            if col < self.cols {
                self.screen.set_cell(self.cursor.row, col, Cell::default());
            }
        }
    }

    fn insert_lines(&mut self, n: usize) {
        for _ in 0..n {
            self.screen.insert_line(self.cursor.row, self.scroll_bottom);
        }
    }

    fn delete_lines(&mut self, n: usize) {
        for _ in 0..n {
            self.screen.delete_line(self.cursor.row, self.scroll_bottom);
        }
    }

    fn scroll_up(&mut self, n: usize) {
        for _ in 0..n {
            self.screen.scroll_up(self.scroll_top, self.scroll_bottom);
        }
    }

    fn scroll_down(&mut self, n: usize) {
        for _ in 0..n {
            self.screen.scroll_down(self.scroll_top, self.scroll_bottom);
        }
    }

    // Line operations
    fn line_feed(&mut self) {
        if self.cursor.row == self.scroll_bottom {
            self.screen.scroll_up(self.scroll_top, self.scroll_bottom);
        } else if self.cursor.row < self.rows - 1 {
            self.cursor.row += 1;
        }
        
        if self.modes.contains(&TerminalMode::LineFeedNewLine) {
            self.cursor.col = 0;
        }
    }

    fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    fn index(&mut self) {
        if self.cursor.row == self.scroll_bottom {
            self.screen.scroll_up(self.scroll_top, self.scroll_bottom);
        } else if self.cursor.row < self.rows - 1 {
            self.cursor.row += 1;
        }
    }

    fn reverse_index(&mut self) {
        if self.cursor.row == self.scroll_top {
            self.screen.scroll_down(self.scroll_top, self.scroll_bottom);
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
    }

    fn next_line(&mut self) {
        self.line_feed();
        self.cursor.col = 0;
    }

    // Tab operations
    fn tab(&mut self) {
        let next_tab = self.tab_stops.iter()
            .find(|&&t| t > self.cursor.col)
            .copied()
            .unwrap_or(self.cols - 1);
        self.cursor.col = next_tab.min(self.cols - 1);
    }

    fn set_tab(&mut self) {
        if !self.tab_stops.contains(&self.cursor.col) {
            self.tab_stops.push(self.cursor.col);
            self.tab_stops.sort();
        }
    }

    fn clear_tabs(&mut self, mode: u16) {
        match mode {
            0 => {
                self.tab_stops.retain(|&t| t != self.cursor.col);
            }
            3 => {
                self.tab_stops.clear();
            }
            _ => {}
        }
    }

    // Cursor save/restore
    fn save_cursor(&mut self) {
        let saved = SavedCursor {
            cursor: self.cursor,
            style: self.current_style.clone(),
            origin_mode: self.modes.contains(&TerminalMode::Origin),
            autowrap: self.modes.contains(&TerminalMode::AutoWrap),
        };
        if self.alt_screen.is_some() {
            self.saved_cursor_alt = Some(saved);
        } else {
            self.saved_cursor = Some(saved);
        }
    }

    fn restore_cursor(&mut self) {
        let saved = if self.alt_screen.is_some() {
            self.saved_cursor_alt.clone()
        } else {
            self.saved_cursor.clone()
        };
        
        if let Some(saved) = saved {
            self.cursor = saved.cursor;
            self.current_style = saved.style;
            self.set_mode(TerminalMode::Origin, saved.origin_mode);
            self.set_mode(TerminalMode::AutoWrap, saved.autowrap);
        }
    }

    // Mode management
    fn set_mode(&mut self, mode: TerminalMode, enabled: bool) {
        if enabled {
            self.modes.insert(mode);
        } else {
            self.modes.remove(&mode);
        }
        
        if mode == TerminalMode::ShowCursor {
            self.cursor.visible = enabled;
        }
    }

    fn set_alternate_screen(&mut self, enable: bool) {
        if enable && self.alt_screen.is_none() {
            let main = std::mem::replace(&mut self.screen, Screen::new(self.cols, self.rows));
            self.alt_screen = Some(main);
            self.cursor = Cursor::default();
        } else if !enable && self.alt_screen.is_some() {
            if let Some(main) = self.alt_screen.take() {
                self.screen = main;
            }
            if let Some(saved) = self.saved_cursor.clone() {
                self.cursor = saved.cursor;
            }
        }
    }

    fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let top = top.saturating_sub(1);
        let bottom = bottom.saturating_sub(1).min(self.rows - 1);
        if top < bottom {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
            self.cursor_position(1, 1);
        }
    }

    // Device status
    fn device_status_report(&mut self, mode: u16) {
        match mode {
            5 => {
                // Status report - device OK
            }
            6 => {
                // Cursor position report
                self.events.push_back(TerminalEvent::CursorPositionReport);
            }
            _ => {}
        }
    }

    /// Print a character at the current cursor position.
    fn print_char(&mut self, c: char) {
        // Handle autowrap
        if self.cursor.col >= self.cols {
            if self.modes.contains(&TerminalMode::AutoWrap) {
                self.carriage_return();
                self.line_feed();
            } else {
                self.cursor.col = self.cols - 1;
            }
        }

        let cell = Cell {
            c,
            style: self.current_style.clone(),
        };
        self.screen.set_cell(self.cursor.row, self.cursor.col, cell);
        self.cursor.col += 1;
    }

    /// Reset the terminal to initial state.
    pub fn reset(&mut self) {
        *self = Self::new(self.cols, self.rows);
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.screen.resize(cols, rows);
        if let Some(ref mut alt) = self.alt_screen {
            alt.resize(cols, rows);
        }
        self.scroll_bottom = rows.saturating_sub(1);
        self.cursor.col = self.cursor.col.min(cols.saturating_sub(1));
        self.cursor.row = self.cursor.row.min(rows.saturating_sub(1));
        
        // Rebuild tab stops
        self.tab_stops.clear();
        for i in (8..cols).step_by(8) {
            self.tab_stops.push(i);
        }
    }

    /// Get pending events.
    pub fn take_events(&mut self) -> Vec<TerminalEvent> {
        self.events.drain(..).collect()
    }

    /// Get the screen buffer.
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Get the cursor state.
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Check if a mode is enabled.
    pub fn mode(&self, mode: TerminalMode) -> bool {
        self.modes.contains(&mode)
    }

    /// Get cursor position report response.
    pub fn cursor_position_report(&self) -> String {
        format!("\x1b[{};{}R", self.cursor.row + 1, self.cursor.col + 1)
    }

    /// Get device attributes response (primary).
    pub fn device_attributes(&self) -> &'static str {
        "\x1b[?62;c" // VT220
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_text() {
        let mut emu = TerminalEmulator::new(80, 24);
        emu.process(b"Hello, World!");
        assert_eq!(emu.cursor.col, 13);
        assert_eq!(emu.cursor.row, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut emu = TerminalEmulator::new(80, 24);
        emu.process(b"\x1b[10;20H"); // Move to row 10, col 20
        assert_eq!(emu.cursor.row, 9);
        assert_eq!(emu.cursor.col, 19);
    }

    #[test]
    fn test_sgr_colors() {
        let mut emu = TerminalEmulator::new(80, 24);
        emu.process(b"\x1b[31m"); // Red foreground
        assert!(matches!(emu.current_style.fg, Color::Indexed(1)));
        
        emu.process(b"\x1b[38;5;200m"); // 256-color
        assert!(matches!(emu.current_style.fg, Color::Indexed(200)));
        
        emu.process(b"\x1b[38;2;255;128;0m"); // RGB
        assert!(matches!(emu.current_style.fg, Color::Rgb(255, 128, 0)));
    }

    #[test]
    fn test_line_feed() {
        let mut emu = TerminalEmulator::new(80, 24);
        emu.process(b"Line1\nLine2");
        assert_eq!(emu.cursor.row, 1);
    }

    #[test]
    fn test_title_change() {
        let mut emu = TerminalEmulator::new(80, 24);
        emu.process(b"\x1b]0;My Title\x07");
        let events = emu.take_events();
        assert!(events.iter().any(|e| matches!(e, TerminalEvent::TitleChanged(t) if t == "My Title")));
    }
}
