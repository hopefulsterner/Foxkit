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
pub mod word;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::VecDeque;
use parking_lot::RwLock;
use anyhow::Result;

pub use cursor::{Cursor, CursorShape};
pub use selection::{Selection, SelectionSet};
pub use view::{EditorView, Viewport};
pub use word::{word_start, word_end, word_at, CharClass};

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
    /// Undo history
    undo_stack: VecDeque<EditTransaction>,
    /// Redo history
    redo_stack: VecDeque<EditTransaction>,
    /// Current edit group (for grouping edits)
    edit_group: Option<u64>,
    /// Clipboard content (local)
    clipboard: Option<String>,
    /// Find query
    find_query: Option<FindState>,
}

/// An edit transaction for undo/redo
#[derive(Debug, Clone)]
pub struct EditTransaction {
    /// The edits in this transaction
    pub edits: Vec<SingleEdit>,
    /// Cursor positions before the edit
    pub selections_before: Vec<(usize, usize)>,
    /// Cursor positions after the edit
    pub selections_after: Vec<(usize, usize)>,
    /// Timestamp
    pub timestamp: u64,
    /// Group ID (for merging related edits)
    pub group: Option<u64>,
}

/// A single edit operation
#[derive(Debug, Clone)]
pub struct SingleEdit {
    /// Byte range that was replaced
    pub range: std::ops::Range<usize>,
    /// The old text that was there
    pub old_text: String,
    /// The new text that replaced it
    pub new_text: String,
}

impl SingleEdit {
    /// Create an insert edit
    pub fn insert(offset: usize, text: String) -> Self {
        Self {
            range: offset..offset,
            old_text: String::new(),
            new_text: text,
        }
    }

    /// Create a delete edit
    pub fn delete(range: std::ops::Range<usize>, old_text: String) -> Self {
        Self {
            range,
            old_text,
            new_text: String::new(),
        }
    }

    /// Create a replace edit
    pub fn replace(range: std::ops::Range<usize>, old_text: String, new_text: String) -> Self {
        Self {
            range,
            old_text,
            new_text,
        }
    }

    /// Get the inverse of this edit (for undo)
    pub fn inverse(&self) -> Self {
        Self {
            range: self.range.start..self.range.start + self.new_text.len(),
            old_text: self.new_text.clone(),
            new_text: self.old_text.clone(),
        }
    }
}

/// Find/search state
#[derive(Debug, Clone)]
pub struct FindState {
    pub query: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
    pub matches: Vec<std::ops::Range<usize>>,
    pub current_match: usize,
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
            undo_stack: VecDeque::with_capacity(1000),
            redo_stack: VecDeque::with_capacity(100),
            edit_group: None,
            clipboard: None,
            find_query: None,
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
            undo_stack: VecDeque::with_capacity(1000),
            redo_stack: VecDeque::with_capacity(100),
            edit_group: None,
            clipboard: None,
            find_query: None,
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
        // Collect edit info before modifying
        let selections_snapshot: Vec<_> = self.selections.iter().map(|s| s.head.offset).collect();
        
        let mut buffer = self.buffer.write();
        let mut offset_adjustment = 0isize;
        
        // For each selection, insert text
        for (i, selection) in self.selections.iter_mut().enumerate() {
            let original_offset = selections_snapshot[i];
            let adjusted_offset = (original_offset as isize + offset_adjustment) as usize;
            
            // Record undo for this edit
            drop(buffer);
            self.push_edit(adjusted_offset..adjusted_offset, String::new(), text.to_string());
            buffer = self.buffer.write();
            
            buffer.content.insert_str(adjusted_offset, text);
            
            // Update cursor position
            selection.head.offset = adjusted_offset + text.len();
            selection.anchor = selection.head;
            
            offset_adjustment += text.len() as isize;
        }
        
        buffer.dirty = true;
        buffer.version += 1;
    }

    /// Delete selection or character before cursor
    pub fn backspace(&mut self) {
        let buffer_content = self.buffer.read().content.clone();
        drop(self.buffer.read());
        
        for selection in self.selections.iter_mut() {
            if selection.is_empty() {
                // Delete char before cursor
                if selection.head.offset > 0 {
                    let offset = selection.head.offset - 1;
                    let deleted = buffer_content.chars().nth(offset).map(|c| c.to_string()).unwrap_or_default();
                    
                    // Record undo
                    self.push_edit(offset..selection.head.offset, deleted, String::new());
                    
                    let mut buffer = self.buffer.write();
                    buffer.content.remove(offset);
                    selection.head.offset = offset;
                    selection.anchor = selection.head;
                    buffer.dirty = true;
                    buffer.version += 1;
                }
            } else {
                // Delete selection
                let (start, end) = selection.range();
                let deleted = buffer_content[start..end].to_string();
                
                // Record undo
                self.push_edit(start..end, deleted, String::new());
                
                let mut buffer = self.buffer.write();
                buffer.content.replace_range(start..end, "");
                selection.head.offset = start;
                selection.anchor = selection.head;
                buffer.dirty = true;
                buffer.version += 1;
            }
        }
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
        let buffer_content = self.buffer.read().content.clone();
        
        for selection in self.selections.iter_mut() {
            if selection.is_empty() {
                if selection.head.offset < buffer_content.len() {
                    let offset = selection.head.offset;
                    // Get the character to delete (handles multi-byte)
                    let char_end = buffer_content[offset..].char_indices()
                        .nth(1)
                        .map(|(i, _)| offset + i)
                        .unwrap_or(buffer_content.len());
                    
                    let deleted = buffer_content[offset..char_end].to_string();
                    
                    // Record undo
                    self.push_edit(offset..char_end, deleted, String::new());
                    
                    let mut buffer = self.buffer.write();
                    buffer.content.replace_range(offset..char_end, "");
                    buffer.dirty = true;
                    buffer.version += 1;
                }
            } else {
                let (start, end) = selection.range();
                let deleted = buffer_content[start..end].to_string();
                
                self.push_edit(start..end, deleted, String::new());
                
                let mut buffer = self.buffer.write();
                buffer.content.replace_range(start..end, "");
                selection.head.offset = start;
                selection.anchor = selection.head;
                buffer.dirty = true;
                buffer.version += 1;
            }
        }
    }

    /// Delete current line
    pub fn delete_line(&mut self) {
        let buffer_content = self.buffer.read().content.clone();
        
        for selection in self.selections.iter_mut() {
            let offset = selection.head.offset;
            
            // Find line boundaries
            let line_start = buffer_content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = buffer_content[offset..].find('\n')
                .map(|i| offset + i + 1) // Include the newline
                .unwrap_or(buffer_content.len());
            
            let deleted = buffer_content[line_start..line_end].to_string();
            
            // Record undo
            self.push_edit(line_start..line_end, deleted, String::new());
            
            let mut buffer = self.buffer.write();
            buffer.content.replace_range(line_start..line_end, "");
            selection.head.offset = line_start.min(buffer.content.len());
            selection.anchor = selection.head;
            buffer.dirty = true;
            buffer.version += 1;
            break; // Process one line at a time for multi-cursor
        }
    }

    /// Delete word
    pub fn delete_word(&mut self, forward: bool) {
        let buffer_content = self.buffer.read().content.clone();
        
        for selection in self.selections.iter_mut() {
            let offset = selection.head.offset;
            let (start, end) = if forward {
                let word_end = word::next_word_boundary(&buffer_content, offset);
                (offset, word_end)
            } else {
                let word_start = word::prev_word_boundary(&buffer_content, offset);
                (word_start, offset)
            };
            
            if start != end {
                // Record undo
                let old_text = buffer_content[start..end].to_string();
                self.push_edit(start..end, old_text, String::new());
                
                let mut buffer = self.buffer.write();
                buffer.content.replace_range(start..end, "");
                selection.head.offset = start;
                selection.anchor = selection.head;
                buffer.dirty = true;
                buffer.version += 1;
            }
        }
    }

    /// Delete to end of line
    pub fn delete_to_line_end(&mut self) {
        let buffer_content = self.buffer.read().content.clone();
        
        for selection in self.selections.iter_mut() {
            let offset = selection.head.offset;
            let line_end = buffer_content[offset..].find('\n')
                .map(|i| offset + i)
                .unwrap_or(buffer_content.len());
            
            if offset < line_end {
                let deleted = buffer_content[offset..line_end].to_string();
                
                self.push_edit(offset..line_end, deleted, String::new());
                
                let mut buffer = self.buffer.write();
                buffer.content.replace_range(offset..line_end, "");
                buffer.dirty = true;
                buffer.version += 1;
            }
        }
    }

    /// Delete to start of line
    pub fn delete_to_line_start(&mut self) {
        let buffer_content = self.buffer.read().content.clone();
        
        for selection in self.selections.iter_mut() {
            let offset = selection.head.offset;
            let line_start = buffer_content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
            
            if line_start < offset {
                let deleted = buffer_content[line_start..offset].to_string();
                
                self.push_edit(line_start..offset, deleted, String::new());
                
                let mut buffer = self.buffer.write();
                buffer.content.replace_range(line_start..offset, "");
                selection.head.offset = line_start;
                selection.anchor = selection.head;
                buffer.dirty = true;
                buffer.version += 1;
            }
        }
    }

    /// Move by word
    pub fn move_word(&mut self, direction: Direction, extend: bool) {
        let buffer = self.buffer.read();
        let content = &buffer.content;
        
        for selection in self.selections.iter_mut() {
            let offset = selection.head.offset;
            let new_offset = match direction {
                Direction::Left => word::prev_word_boundary(content, offset),
                Direction::Right => word::next_word_boundary(content, offset),
                _ => offset, // Up/Down not applicable for word movement
            };
            
            selection.head.offset = new_offset;
            if !extend {
                selection.anchor = selection.head;
            }
        }
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
        let buffer = self.buffer.read();
        let content = &buffer.content;
        
        for selection in self.selections.iter_mut() {
            let offset = selection.head.offset;
            if let Some((start, end)) = word::word_at(content, offset) {
                selection.anchor.offset = start;
                selection.head.offset = end;
            }
        }
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
        let buffer = self.buffer.read();
        let mut copied_text = String::new();
        
        for selection in self.selections.iter() {
            if !selection.is_empty() {
                let (start, end) = selection.range();
                if !copied_text.is_empty() {
                    copied_text.push('\n');
                }
                copied_text.push_str(&buffer.content[start..end]);
            }
        }
        
        if !copied_text.is_empty() {
            self.clipboard = Some(copied_text);
        }
    }

    /// Cut selection to clipboard
    pub fn cut(&mut self) {
        self.copy();
        if self.clipboard.is_some() {
            self.backspace();
        }
    }

    /// Paste from clipboard
    pub fn paste(&mut self, text: &str) {
        self.insert(text);
    }

    /// Paste from internal clipboard
    pub fn paste_from_clipboard(&mut self) {
        if let Some(text) = self.clipboard.clone() {
            self.insert(&text);
        }
    }

    /// Push an edit to the undo stack
    fn push_edit(&mut self, range: std::ops::Range<usize>, old_text: String, new_text: String) {
        let selections_before: Vec<(usize, usize)> = self.selections
            .iter()
            .map(|s| (s.anchor.offset, s.head.offset))
            .collect();
        
        let edit = SingleEdit {
            range,
            old_text,
            new_text,
        };
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        
        // Check if we should merge with previous edit
        let should_merge = if let Some(last) = self.undo_stack.back_mut() {
            // Merge if edits are close in time (< 500ms) and same group
            timestamp - last.timestamp < 500 && self.edit_group == last.group
        } else {
            false
        };
        
        if should_merge {
            if let Some(last) = self.undo_stack.back_mut() {
                last.edits.push(edit);
                last.timestamp = timestamp;
            }
        } else {
            let transaction = EditTransaction {
                edits: vec![edit],
                selections_before,
                selections_after: Vec::new(), // Will be set when transaction completes
                timestamp,
                group: self.edit_group,
            };
            
            self.undo_stack.push_back(transaction);
            
            // Limit undo stack size
            if self.undo_stack.len() > 1000 {
                self.undo_stack.pop_front();
            }
        }
        
        // Clear redo stack on new edit
        self.redo_stack.clear();
    }

    /// Undo last edit
    pub fn undo(&mut self) {
        if let Some(mut transaction) = self.undo_stack.pop_back() {
            // Store current selections for redo
            transaction.selections_after = self.selections
                .iter()
                .map(|s| (s.anchor.offset, s.head.offset))
                .collect();
            
            let mut buffer = self.buffer.write();
            
            // Apply edits in reverse order
            for edit in transaction.edits.iter().rev() {
                // Replace new_text with old_text (reverse the edit)
                let start = edit.range.start;
                let end = start + edit.new_text.len();
                buffer.content.replace_range(start..end.min(buffer.content.len()), &edit.old_text);
            }
            
            buffer.dirty = true;
            buffer.version += 1;
            drop(buffer);
            
            // Restore selections
            self.selections.clear();
            for (anchor, head) in &transaction.selections_before {
                self.selections.add(Selection::new(*anchor, *head));
            }
            
            self.redo_stack.push_back(transaction);
        }
    }

    /// Redo last undone edit
    pub fn redo(&mut self) {
        if let Some(transaction) = self.redo_stack.pop_back() {
            let mut buffer = self.buffer.write();
            
            // Apply edits in forward order
            for edit in &transaction.edits {
                let start = edit.range.start;
                let end = start + edit.old_text.len();
                buffer.content.replace_range(start..end.min(buffer.content.len()), &edit.new_text);
            }
            
            buffer.dirty = true;
            buffer.version += 1;
            drop(buffer);
            
            // Restore selections to after state
            self.selections.clear();
            for (anchor, head) in &transaction.selections_after {
                self.selections.add(Selection::new(*anchor, *head));
            }
            
            self.undo_stack.push_back(transaction);
        }
    }

    /// Start an edit group (groups consecutive edits for undo)
    pub fn start_edit_group(&mut self) {
        self.edit_group = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        );
    }

    /// End current edit group
    pub fn end_edit_group(&mut self) {
        self.edit_group = None;
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
        let buffer_content = self.buffer.read().content.clone();
        
        if let Some(selection) = self.selections.primary_mut() {
            let offset = selection.head.offset;
            
            // Find current line boundaries
            let line_start = buffer_content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
            
            // Can't move up if we're on the first line
            if line_start == 0 {
                return;
            }
            
            let line_end = buffer_content[offset..].find('\n')
                .map(|i| offset + i)
                .unwrap_or(buffer_content.len());
            
            // Find previous line start
            let prev_line_start = buffer_content[..line_start - 1].rfind('\n').map(|i| i + 1).unwrap_or(0);
            
            // Get current line content (without trailing newline)
            let current_line = buffer_content[line_start..line_end].to_string();
            let prev_line = buffer_content[prev_line_start..line_start - 1].to_string();
            
            // Calculate cursor offset within line
            let cursor_col = offset - line_start;
            
            // Record undo for the swap
            self.push_edit(prev_line_start..line_end, 
                format!("{}\n{}", prev_line, current_line),
                format!("{}\n{}", current_line, prev_line));
            
            let mut buffer = self.buffer.write();
            buffer.content.replace_range(prev_line_start..line_end, &format!("{}\n{}", current_line, prev_line));
            
            // Update cursor position
            selection.head.offset = prev_line_start + cursor_col.min(current_line.len());
            selection.anchor = selection.head;
            
            buffer.dirty = true;
            buffer.version += 1;
        }
    }

    /// Move line down
    pub fn move_line_down(&mut self) {
        let buffer_content = self.buffer.read().content.clone();
        
        if let Some(selection) = self.selections.primary_mut() {
            let offset = selection.head.offset;
            
            // Find current line boundaries
            let line_start = buffer_content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = buffer_content[offset..].find('\n')
                .map(|i| offset + i)
                .unwrap_or(buffer_content.len());
            
            // Can't move down if we're on the last line
            if line_end >= buffer_content.len() {
                return;
            }
            
            // Find next line end
            let next_line_end = buffer_content[line_end + 1..].find('\n')
                .map(|i| line_end + 1 + i)
                .unwrap_or(buffer_content.len());
            
            // Get line contents
            let current_line = buffer_content[line_start..line_end].to_string();
            let next_line = buffer_content[line_end + 1..next_line_end].to_string();
            
            // Calculate cursor offset within line
            let cursor_col = offset - line_start;
            
            // Record undo
            self.push_edit(line_start..next_line_end,
                format!("{}\n{}", current_line, next_line),
                format!("{}\n{}", next_line, current_line));
            
            let mut buffer = self.buffer.write();
            buffer.content.replace_range(line_start..next_line_end, &format!("{}\n{}", next_line, current_line));
            
            // Update cursor position (now on line below)
            let new_line_start = line_start + next_line.len() + 1;
            selection.head.offset = new_line_start + cursor_col.min(current_line.len());
            selection.anchor = selection.head;
            
            buffer.dirty = true;
            buffer.version += 1;
        }
    }

    /// Join current line with next
    pub fn join_lines(&mut self) {
        let buffer_content = self.buffer.read().content.clone();
        
        if let Some(selection) = self.selections.primary_mut() {
            let offset = selection.head.offset;
            
            // Find end of current line
            if let Some(newline_pos) = buffer_content[offset..].find('\n').map(|i| offset + i) {
                // Find start of next line content (skip leading whitespace)
                let next_line_start = newline_pos + 1;
                if next_line_start < buffer_content.len() {
                    let next_line_content_start = buffer_content[next_line_start..]
                        .find(|c: char| !c.is_whitespace() || c == '\n')
                        .map(|i| next_line_start + i)
                        .unwrap_or(next_line_start);
                    
                    let deleted = buffer_content[newline_pos..next_line_content_start].to_string();
                    
                    // Record undo - replace newline and leading whitespace with single space
                    self.push_edit(newline_pos..next_line_content_start, deleted, " ".to_string());
                    
                    let mut buffer = self.buffer.write();
                    buffer.content.replace_range(newline_pos..next_line_content_start, " ");
                    
                    // Move cursor to join point
                    selection.head.offset = newline_pos;
                    selection.anchor = selection.head;
                    
                    buffer.dirty = true;
                    buffer.version += 1;
                }
            }
        }
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
    pub fn find(&mut self, query: &str) {
        let buffer = self.buffer.read();
        let content = &buffer.content;
        
        let mut matches = Vec::new();
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        // Case-insensitive search by default
        let mut search_from = 0;
        while let Some(pos) = content_lower[search_from..].find(&query_lower) {
            let abs_pos = search_from + pos;
            matches.push(abs_pos..abs_pos + query.len());
            search_from = abs_pos + 1;
        }
        
        let current_match = if !matches.is_empty() {
            // Find match closest to cursor
            let cursor_pos = self.selections.primary().map(|s| s.head.offset).unwrap_or(0);
            matches.iter()
                .position(|m| m.start >= cursor_pos)
                .unwrap_or(0)
        } else {
            0
        };
        
        self.find_query = Some(FindState {
            query: query.to_string(),
            case_sensitive: false,
            matches,
            current_match,
        });
        
        // Jump to first match
        self.find_next();
    }

    /// Find next match
    pub fn find_next(&mut self) {
        if let Some(ref mut state) = self.find_query {
            if !state.matches.is_empty() {
                state.current_match = (state.current_match + 1) % state.matches.len();
                let match_range = &state.matches[state.current_match];
                
                // Select the match
                if let Some(selection) = self.selections.primary_mut() {
                    selection.anchor.offset = match_range.start;
                    selection.head.offset = match_range.end;
                }
            }
        }
    }

    /// Find previous match
    pub fn find_previous(&mut self) {
        if let Some(ref mut state) = self.find_query {
            if !state.matches.is_empty() {
                state.current_match = if state.current_match == 0 {
                    state.matches.len() - 1
                } else {
                    state.current_match - 1
                };
                let match_range = &state.matches[state.current_match];
                
                // Select the match
                if let Some(selection) = self.selections.primary_mut() {
                    selection.anchor.offset = match_range.start;
                    selection.head.offset = match_range.end;
                }
            }
        }
    }

    /// Replace current match
    pub fn replace(&mut self, replacement: &str) {
        if let Some(ref state) = self.find_query.clone() {
            if !state.matches.is_empty() && state.current_match < state.matches.len() {
                let match_range = state.matches[state.current_match].clone();
                let old_text = self.buffer.read().content[match_range.clone()].to_string();
                
                self.push_edit(match_range.clone(), old_text, replacement.to_string());
                
                let mut buffer = self.buffer.write();
                buffer.content.replace_range(match_range, replacement);
                buffer.dirty = true;
                buffer.version += 1;
                drop(buffer);
                
                // Re-run search to update matches
                let query = state.query.clone();
                self.find(&query);
            }
        }
    }

    /// Replace all matches
    pub fn replace_all(&mut self, replacement: &str) {
        if let Some(ref state) = self.find_query.clone() {
            if state.matches.is_empty() {
                return;
            }
            
            self.start_edit_group();
            
            // Replace from end to start to preserve offsets
            for match_range in state.matches.iter().rev() {
                let old_text = self.buffer.read().content[match_range.clone()].to_string();
                
                self.push_edit(match_range.clone(), old_text, replacement.to_string());
                
                let mut buffer = self.buffer.write();
                buffer.content.replace_range(match_range.clone(), replacement);
                buffer.dirty = true;
                buffer.version += 1;
            }
            
            self.end_edit_group();
            
            // Clear find state
            self.find_query = None;
        }
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
