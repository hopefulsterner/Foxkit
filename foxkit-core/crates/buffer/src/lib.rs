//! # Foxkit Buffer
//!
//! Text buffer with undo/redo, change tracking, and collaboration support.

pub mod edit;
pub mod history;
pub mod selection;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;

use rope::{Rope, Point};
pub use edit::{Edit, EditKind};
pub use history::History;
pub use selection::{Selection, SelectionSet};

/// Buffer ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(pub u64);

impl BufferId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for BufferId {
    fn default() -> Self {
        Self::new()
    }
}

/// Text buffer
pub struct Buffer {
    /// Unique ID
    pub id: BufferId,
    /// File path (if any)
    pub path: Option<PathBuf>,
    /// Text content
    text: Rope,
    /// Edit history
    history: History,
    /// Selections
    selections: SelectionSet,
    /// Is modified?
    modified: bool,
    /// Language ID
    pub language_id: Option<String>,
    /// Version (increments on each edit)
    version: u64,
    /// Saved version
    saved_version: u64,
}

impl Buffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self {
            id: BufferId::new(),
            path: None,
            text: Rope::new(),
            history: History::new(),
            selections: SelectionSet::new(),
            modified: false,
            language_id: None,
            version: 0,
            saved_version: 0,
        }
    }

    /// Create a buffer from text
    pub fn from_text(text: &str) -> Self {
        let mut buffer = Self::new();
        buffer.text = Rope::from_str(text);
        buffer
    }

    /// Create a buffer from file path
    pub fn from_file(path: impl Into<PathBuf>, content: &str) -> Self {
        let path = path.into();
        let mut buffer = Self::from_text(content);
        buffer.language_id = detect_language(&path);
        buffer.path = Some(path);
        buffer
    }

    /// Get text rope
    pub fn rope(&self) -> &Rope {
        &self.text
    }

    /// Get full text
    pub fn text(&self) -> String {
        self.text.to_string()
    }

    /// Get text length
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        self.text.line_count()
    }

    /// Get a line by index
    pub fn line(&self, idx: usize) -> Option<String> {
        self.text.line(idx)
    }

    /// Get text slice
    pub fn slice(&self, range: std::ops::Range<usize>) -> String {
        self.text.slice(range)
    }

    /// Get current version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Check if modified
    pub fn is_modified(&self) -> bool {
        self.version != self.saved_version
    }

    /// Apply an edit
    pub fn apply_edit(&mut self, edit: Edit) {
        // Save to history
        let inverse = edit.inverse(&self.text);
        self.history.push(edit.clone(), inverse);

        // Apply to text
        match &edit.kind {
            EditKind::Insert { offset, text } => {
                self.text.insert(*offset, text);
            }
            EditKind::Delete { range } => {
                self.text.delete(range.clone());
            }
            EditKind::Replace { range, text } => {
                self.text.replace(range.clone(), text);
            }
        }

        self.version += 1;
    }

    /// Insert text at offset
    pub fn insert(&mut self, offset: usize, text: &str) {
        self.apply_edit(Edit::insert(offset, text));
    }

    /// Delete a range
    pub fn delete(&mut self, range: std::ops::Range<usize>) {
        self.apply_edit(Edit::delete(range));
    }

    /// Replace a range with text
    pub fn replace(&mut self, range: std::ops::Range<usize>, text: &str) {
        self.apply_edit(Edit::replace(range, text));
    }

    /// Undo last edit
    pub fn undo(&mut self) -> bool {
        if let Some(edit) = self.history.undo() {
            self.apply_edit_raw(&edit);
            self.version += 1;
            true
        } else {
            false
        }
    }

    /// Redo last undone edit
    pub fn redo(&mut self) -> bool {
        if let Some(edit) = self.history.redo() {
            self.apply_edit_raw(&edit);
            self.version += 1;
            true
        } else {
            false
        }
    }

    /// Apply edit without recording to history
    fn apply_edit_raw(&mut self, edit: &Edit) {
        match &edit.kind {
            EditKind::Insert { offset, text } => {
                self.text.insert(*offset, text);
            }
            EditKind::Delete { range } => {
                self.text.delete(range.clone());
            }
            EditKind::Replace { range, text } => {
                self.text.replace(range.clone(), text);
            }
        }
    }

    /// Mark as saved
    pub fn mark_saved(&mut self) {
        self.saved_version = self.version;
    }

    /// Get selections
    pub fn selections(&self) -> &SelectionSet {
        &self.selections
    }

    /// Set selections
    pub fn set_selections(&mut self, selections: SelectionSet) {
        self.selections = selections;
    }

    /// Get primary selection
    pub fn primary_selection(&self) -> Selection {
        self.selections.primary()
    }

    /// Convert offset to point
    pub fn offset_to_point(&self, offset: usize) -> Point {
        self.text.offset_to_point(offset)
    }

    /// Convert point to offset
    pub fn point_to_offset(&self, point: Point) -> usize {
        self.text.point_to_offset(point)
    }

    /// Can undo?
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Can redo?
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect language from file path
fn detect_language(path: &PathBuf) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    let lang = match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "hpp" | "cc" | "cxx" => "cpp",
        "java" => "java",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "scala" => "scala",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "md" | "markdown" => "markdown",
        "sh" | "bash" | "zsh" => "shellscript",
        "sql" => "sql",
        "dockerfile" => "dockerfile",
        _ => return None,
    };
    Some(lang.to_string())
}

/// Shared buffer
pub type SharedBuffer = Arc<RwLock<Buffer>>;

/// Create a shared buffer
pub fn shared_buffer(buffer: Buffer) -> SharedBuffer {
    Arc::new(RwLock::new(buffer))
}
