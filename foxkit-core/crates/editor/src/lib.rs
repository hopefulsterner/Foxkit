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
