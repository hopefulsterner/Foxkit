//! # Foxkit Editor Core
//! 
//! High-performance text editing engine combining:
//! - Zed's rope-based buffer for O(log n) operations
//! - Multi-cursor editing
//! - Tree-sitter syntax highlighting
//! - LSP integration

pub mod cursor;
pub mod selection;
pub mod view;
pub mod input;
pub mod commands;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::Result;

pub use cursor::{Cursor, CursorShape};
pub use selection::{Selection, SelectionSet};
pub use view::{EditorView, Viewport};

/// Editor instance - manages a single editor pane
pub struct Editor {
    /// Unique editor ID
    id: EditorId,
    /// The buffer being edited
    buffer: Arc<RwLock<Buffer>>,
    /// Current selections (supports multi-cursor)
    selections: SelectionSet,
    /// Viewport state
    viewport: Viewport,
    /// Editor mode (normal, insert, visual for vim-mode)
    mode: EditorMode,
    /// Soft wrap settings
    soft_wrap: SoftWrap,
    /// Is this editor focused?
    focused: bool,
}

/// Unique editor identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EditorId(pub u64);

/// Text buffer (simplified - real impl in buffer crate)
pub struct Buffer {
    /// File path (if saved)
    path: Option<PathBuf>,
    /// Buffer content (rope-based in real impl)
    content: String,
    /// Is buffer modified?
    dirty: bool,
    /// Language ID
    language: Option<String>,
    /// Version for sync
    version: u64,
}

/// Editor mode (for vim-style editing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    #[default]
    Normal,
    Insert,
    Visual,
    VisualLine,
    VisualBlock,
    Replace,
}

/// Soft wrap configuration
#[derive(Debug, Clone)]
pub struct SoftWrap {
    pub enabled: bool,
    pub column: Option<u32>,
}

impl Editor {
    /// Create a new empty editor
    pub fn new(id: EditorId) -> Self {
        Self {
            id,
            buffer: Arc::new(RwLock::new(Buffer::new())),
            selections: SelectionSet::new(),
            viewport: Viewport::default(),
            mode: EditorMode::Normal,
            soft_wrap: SoftWrap { enabled: false, column: None },
            focused: false,
        }
    }

    /// Create an editor for a file
    pub async fn open(id: EditorId, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = tokio::fs::read_to_string(path).await?;
        
        let buffer = Buffer {
            path: Some(path.to_path_buf()),
            content,
            dirty: false,
            language: detect_language(path),
            version: 0,
        };

        Ok(Self {
            id,
            buffer: Arc::new(RwLock::new(buffer)),
            selections: SelectionSet::new(),
            viewport: Viewport::default(),
            mode: EditorMode::Normal,
            soft_wrap: SoftWrap { enabled: false, column: None },
            focused: false,
        })
    }

    /// Get editor ID
    pub fn id(&self) -> EditorId {
        self.id
    }

    /// Get buffer reference
    pub fn buffer(&self) -> &Arc<RwLock<Buffer>> {
        &self.buffer
    }

    /// Get current selections
    pub fn selections(&self) -> &SelectionSet {
        &self.selections
    }

    /// Get mutable selections
    pub fn selections_mut(&mut self) -> &mut SelectionSet {
        &mut self.selections
    }

    /// Get viewport
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Get editor mode
    pub fn mode(&self) -> EditorMode {
        self.mode
    }

    /// Set editor mode
    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
    }

    /// Is this editor focused?
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Insert text at current cursor position(s)
    pub fn insert(&mut self, text: &str) {
        let mut buffer = self.buffer.write();
        
        // For each selection, insert text
        for selection in self.selections.iter_mut() {
            let offset = selection.head.offset;
            buffer.content.insert_str(offset, text);
            
            // Update cursor position
            selection.head.offset += text.len();
            selection.anchor = selection.head;
        }
        
        buffer.dirty = true;
        buffer.version += 1;
    }

    /// Delete selection or character before cursor
    pub fn backspace(&mut self) {
        let mut buffer = self.buffer.write();
        
        for selection in self.selections.iter_mut() {
            if selection.is_empty() {
                // Delete char before cursor
                if selection.head.offset > 0 {
                    let offset = selection.head.offset - 1;
                    buffer.content.remove(offset);
                    selection.head.offset = offset;
                    selection.anchor = selection.head;
                }
            } else {
                // Delete selection
                let (start, end) = selection.range();
                buffer.content.replace_range(start..end, "");
                selection.head.offset = start;
                selection.anchor = selection.head;
            }
        }
        
        buffer.dirty = true;
        buffer.version += 1;
    }

    /// Move cursor(s) in a direction
    pub fn move_cursor(&mut self, direction: Direction, extend_selection: bool) {
        let buffer = self.buffer.read();
        
        for selection in self.selections.iter_mut() {
            let new_offset = match direction {
                Direction::Left => selection.head.offset.saturating_sub(1),
                Direction::Right => (selection.head.offset + 1).min(buffer.content.len()),
                Direction::Up => {
                    // Find previous line
                    self.offset_for_line_delta(&buffer.content, selection.head.offset, -1)
                }
                Direction::Down => {
                    // Find next line
                    self.offset_for_line_delta(&buffer.content, selection.head.offset, 1)
                }
            };
            
            selection.head.offset = new_offset;
            if !extend_selection {
                selection.anchor = selection.head;
            }
        }
    }

    fn offset_for_line_delta(&self, content: &str, offset: usize, delta: i32) -> usize {
        let lines: Vec<&str> = content.lines().collect();
        let mut current_line = 0;
        let mut line_start = 0;
        let mut col = 0;
        
        // Find current line and column
        for (i, line) in lines.iter().enumerate() {
            let line_end = line_start + line.len() + 1; // +1 for newline
            if offset < line_end || i == lines.len() - 1 {
                current_line = i;
                col = offset - line_start;
                break;
            }
            line_start = line_end;
        }
        
        // Calculate new line
        let new_line = (current_line as i32 + delta).max(0) as usize;
        let new_line = new_line.min(lines.len().saturating_sub(1));
        
        // Find offset for new line
        let mut new_offset = 0;
        for (i, line) in lines.iter().enumerate() {
            if i == new_line {
                new_offset += col.min(line.len());
                break;
            }
            new_offset += line.len() + 1;
        }
        
        new_offset.min(content.len())
    }

    /// Save buffer to file
    pub async fn save(&self) -> Result<()> {
        let buffer = self.buffer.read();
        
        if let Some(path) = &buffer.path {
            tokio::fs::write(path, &buffer.content).await?;
        }
        
        Ok(())
    }

    /// Add a new cursor at position
    pub fn add_cursor(&mut self, offset: usize) {
        self.selections.add(Selection::point(offset));
    }

    /// Clear all cursors except primary
    pub fn clear_secondary_cursors(&mut self) {
        self.selections.clear_secondary();
    }

    /// Select all text
    pub fn select_all(&mut self) {
        let len = self.buffer.read().content.len();
        self.selections = SelectionSet::single(Selection::new(0, len));
    }

    /// Get text content
    pub fn text(&self) -> String {
        self.buffer.read().content.clone()
    }

    /// Get file path
    pub fn path(&self) -> Option<PathBuf> {
        self.buffer.read().path.clone()
    }

    /// Is buffer dirty?
    pub fn is_dirty(&self) -> bool {
        self.buffer.read().dirty
    }

    // === Additional editing methods for commands ===

    /// Insert a tab (respecting tab settings)
    pub fn insert_tab(&mut self) {
        // TODO: respect tab vs spaces setting
        self.insert("    ");
    }

    /// Delete character forward
    pub fn delete_forward(&mut self) {
        let mut buffer = self.buffer.write();
        for selection in self.selections.iter_mut() {
            if selection.is_empty() {
                if selection.head.offset < buffer.content.len() {
                    buffer.content.remove(selection.head.offset);
                }
            } else {
                let (start, end) = selection.range();
                buffer.content.replace_range(start..end, "");
                selection.head.offset = start;
                selection.anchor = selection.head;
            }
        }
        buffer.dirty = true;
        buffer.version += 1;
    }

    /// Delete current line
    pub fn delete_line(&mut self) {
        // TODO: implement line deletion
    }

    /// Delete word
    pub fn delete_word(&mut self) {
        // TODO: implement word deletion
    }

    /// Delete to end of line
    pub fn delete_to_line_end(&mut self) {
        // TODO: implement
    }

    /// Delete to start of line
    pub fn delete_to_line_start(&mut self) {
        // TODO: implement
    }

    /// Move by word
    pub fn move_word(&mut self, direction: Direction, extend: bool) {
        // TODO: implement word-wise movement
        self.move_cursor(direction, extend);
    }

    /// Move to line start
    pub fn move_to_line_start(&mut self, extend: bool) {
        let buffer = self.buffer.read();
        for selection in self.selections.iter_mut() {
            let content = &buffer.content;
            let offset = selection.head.offset;
            let line_start = content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
            selection.head.offset = line_start;
            if !extend {
                selection.anchor = selection.head;
            }
        }
    }

    /// Move to line end
    pub fn move_to_line_end(&mut self, extend: bool) {
        let buffer = self.buffer.read();
        for selection in self.selections.iter_mut() {
            let content = &buffer.content;
            let offset = selection.head.offset;
            let line_end = content[offset..].find('\n').map(|i| offset + i).unwrap_or(content.len());
            selection.head.offset = line_end;
            if !extend {
                selection.anchor = selection.head;
            }
        }
    }

    /// Move to document start
    pub fn move_to_document_start(&mut self, extend: bool) {
        for selection in self.selections.iter_mut() {
            selection.head.offset = 0;
            if !extend {
                selection.anchor = selection.head;
            }
        }
    }

    /// Move to document end
    pub fn move_to_document_end(&mut self, extend: bool) {
        let len = self.buffer.read().content.len();
        for selection in self.selections.iter_mut() {
            selection.head.offset = len;
            if !extend {
                selection.anchor = selection.head;
            }
        }
    }

    /// Page up
    pub fn page_up(&mut self, extend: bool) {
        for _ in 0..self.viewport.visible_lines {
            self.move_cursor(Direction::Up, extend);
        }
    }

    /// Page down
    pub fn page_down(&mut self, extend: bool) {
        for _ in 0..self.viewport.visible_lines {
            self.move_cursor(Direction::Down, extend);
        }
    }

    /// Select current line
    pub fn select_line(&mut self) {
        self.move_to_line_start(false);
        self.move_cursor(Direction::Down, true);
    }

    /// Select current word
    pub fn select_word(&mut self) {
        // TODO: implement word selection
    }

    /// Expand selection (tree-sitter aware)
    pub fn expand_selection(&mut self) {
        // TODO: implement syntax-aware selection expansion
    }

    /// Shrink selection
    pub fn shrink_selection(&mut self) {
        // TODO: implement
    }

    /// Add cursor above
    pub fn add_cursor_above(&mut self) {
        if let Some(selection) = self.selections.primary() {
            let offset = self.offset_for_line_delta(
                &self.buffer.read().content,
                selection.head.offset,
                -1,
            );
            self.add_cursor(offset);
        }
    }

    /// Add cursor below
    pub fn add_cursor_below(&mut self) {
        if let Some(selection) = self.selections.primary() {
            let offset = self.offset_for_line_delta(
                &self.buffer.read().content,
                selection.head.offset,
                1,
            );
            self.add_cursor(offset);
        }
    }

    /// Add cursors to all line ends in selection
    pub fn add_cursors_to_line_ends(&mut self) {
        // TODO: implement
    }

    /// Copy selection to clipboard
    pub fn copy(&mut self) {
        // TODO: implement clipboard integration
    }

    /// Cut selection to clipboard
    pub fn cut(&mut self) {
        self.copy();
        self.backspace();
    }

    /// Paste from clipboard
    pub fn paste(&mut self, text: &str) {
        self.insert(text);
    }

    /// Undo last edit
    pub fn undo(&mut self) {
        // TODO: implement with history
    }

    /// Redo last undone edit
    pub fn redo(&mut self) {
        // TODO: implement with history
    }

    /// Duplicate current line
    pub fn duplicate_line(&mut self) {
        let buffer = self.buffer.read();
        let content = &buffer.content;
        
        for selection in self.selections.iter() {
            let offset = selection.head.offset;
            let line_start = content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = content[offset..].find('\n').map(|i| offset + i + 1).unwrap_or(content.len());
            let line = content[line_start..line_end].to_string();
            drop(buffer);
            
            let mut buffer = self.buffer.write();
            buffer.content.insert_str(line_end, &line);
            buffer.dirty = true;
            buffer.version += 1;
            break; // Only process first selection for now
        }
    }

    /// Move line up
    pub fn move_line_up(&mut self) {
        // TODO: implement
    }

    /// Move line down
    pub fn move_line_down(&mut self) {
        // TODO: implement
    }

    /// Join current line with next
    pub fn join_lines(&mut self) {
        // TODO: implement
    }

    /// Toggle line comment
    pub fn toggle_comment(&mut self) {
        // TODO: implement with language awareness
    }

    /// Indent selection
    pub fn indent(&mut self) {
        self.insert_tab();
    }

    /// Outdent selection
    pub fn outdent(&mut self) {
        // TODO: implement
    }

    /// Transform selection case
    pub fn transform_case(&mut self, transform: commands::CaseTransform) {
        let mut buffer = self.buffer.write();
        
        for selection in self.selections.iter() {
            if !selection.is_empty() {
                let (start, end) = selection.range();
                let text = &buffer.content[start..end];
                let transformed = match transform {
                    commands::CaseTransform::Upper => text.to_uppercase(),
                    commands::CaseTransform::Lower => text.to_lowercase(),
                    commands::CaseTransform::Title => to_title_case(text),
                    _ => text.to_string(), // TODO: implement other transforms
                };
                buffer.content.replace_range(start..end, &transformed);
            }
        }
        
        buffer.dirty = true;
        buffer.version += 1;
    }

    /// Start find
    pub fn find(&mut self, _query: &str) {
        // TODO: implement search
    }

    /// Find next match
    pub fn find_next(&mut self) {
        // TODO: implement
    }

    /// Find previous match
    pub fn find_previous(&mut self) {
        // TODO: implement
    }

    /// Replace current match
    pub fn replace(&mut self, _replacement: &str) {
        // TODO: implement
    }

    /// Replace all matches
    pub fn replace_all(&mut self, _replacement: &str) {
        // TODO: implement
    }

    /// Save as new file
    pub async fn save_as(&mut self, path: &str) -> Result<()> {
        {
            let mut buffer = self.buffer.write();
            buffer.path = Some(PathBuf::from(path));
        }
        self.save().await
    }

    /// Center cursor in viewport
    pub fn center_cursor(&mut self) {
        if let Some(selection) = self.selections.primary() {
            let line = self.offset_to_line(selection.head.offset);
            let center_offset = self.viewport.visible_lines / 2;
            self.viewport.first_line = line.saturating_sub(center_offset);
        }
    }

    /// Scroll up
    pub fn scroll_up(&mut self, lines: usize) {
        self.viewport.first_line = self.viewport.first_line.saturating_sub(lines);
    }

    /// Scroll down
    pub fn scroll_down(&mut self, lines: usize) {
        let max_line = self.buffer.read().line_count().saturating_sub(1);
        self.viewport.first_line = (self.viewport.first_line + lines).min(max_line);
    }

    fn offset_to_line(&self, offset: usize) -> usize {
        let buffer = self.buffer.read();
        buffer.content[..offset].matches('\n').count()
    }
}

fn to_title_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    
    for c in s.chars() {
        if c.is_whitespace() {
            capitalize_next = true;
            result.push(c);
        } else if capitalize_next {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.extend(c.to_lowercase());
        }
    }
    
    result
}

/// Movement direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            path: None,
            content: String::new(),
            dirty: false,
            language: None,
            version: 0,
        }
    }

    pub fn line_count(&self) -> usize {
        self.content.lines().count().max(1)
    }

    pub fn line(&self, idx: usize) -> Option<&str> {
        self.content.lines().nth(idx)
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

fn detect_language(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext {
            "rs" => "rust",
            "js" | "mjs" | "cjs" => "javascript",
            "ts" | "mts" | "cts" => "typescript",
            "jsx" => "javascriptreact",
            "tsx" => "typescriptreact",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "kt" | "kts" => "kotlin",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" => "cpp",
            "rb" => "ruby",
            "php" => "php",
            "swift" => "swift",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "md" => "markdown",
            "html" | "htm" => "html",
            "css" => "css",
            "scss" => "scss",
            "sql" => "sql",
            "sh" | "bash" => "shellscript",
            _ => ext,
        })
        .map(String::from)
}

impl Default for SoftWrap {
    fn default() -> Self {
        Self {
            enabled: false,
            column: None,
        }
    }
}
