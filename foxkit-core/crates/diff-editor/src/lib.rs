//! # Foxkit Diff Editor
//!
//! Side-by-side diff viewing and editing.

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Diff editor service
pub struct DiffEditorService {
    /// Active diff sessions
    sessions: RwLock<Vec<DiffSession>>,
    /// Events
    events: broadcast::Sender<DiffEditorEvent>,
    /// Configuration
    config: RwLock<DiffEditorConfig>,
}

impl DiffEditorService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            sessions: RwLock::new(Vec::new()),
            events,
            config: RwLock::new(DiffEditorConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DiffEditorEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: DiffEditorConfig) {
        *self.config.write() = config;
    }

    /// Open diff between two files
    pub fn open_diff(&self, original: PathBuf, modified: PathBuf) -> DiffSessionId {
        let id = DiffSessionId::new();
        
        let session = DiffSession {
            id: id.clone(),
            original: DiffSide::File(original.clone()),
            modified: DiffSide::File(modified.clone()),
            hunks: Vec::new(),
            sync_scroll: true,
        };

        self.sessions.write().push(session);

        let _ = self.events.send(DiffEditorEvent::Opened {
            id: id.clone(),
            original,
            modified,
        });

        id
    }

    /// Open diff between file and content
    pub fn open_diff_with_content(
        &self,
        original: PathBuf,
        modified_content: String,
        modified_label: String,
    ) -> DiffSessionId {
        let id = DiffSessionId::new();
        
        let session = DiffSession {
            id: id.clone(),
            original: DiffSide::File(original.clone()),
            modified: DiffSide::Content {
                content: modified_content,
                label: modified_label.clone(),
            },
            hunks: Vec::new(),
            sync_scroll: true,
        };

        self.sessions.write().push(session);

        let _ = self.events.send(DiffEditorEvent::OpenedWithContent {
            id: id.clone(),
            original,
            modified_label,
        });

        id
    }

    /// Close diff session
    pub fn close(&self, id: &DiffSessionId) {
        self.sessions.write().retain(|s| &s.id != id);
        let _ = self.events.send(DiffEditorEvent::Closed { id: id.clone() });
    }

    /// Get diff session
    pub fn get_session(&self, id: &DiffSessionId) -> Option<DiffSession> {
        self.sessions.read().iter().find(|s| &s.id == id).cloned()
    }

    /// Update hunks for session
    pub fn set_hunks(&self, id: &DiffSessionId, hunks: Vec<DiffHunk>) {
        if let Some(session) = self.sessions.write().iter_mut().find(|s| &s.id == id) {
            session.hunks = hunks;
        }
    }

    /// Navigate to next change
    pub fn goto_next_change(&self, id: &DiffSessionId, current_line: u32) -> Option<u32> {
        self.sessions
            .read()
            .iter()
            .find(|s| &s.id == id)
            .and_then(|s| {
                s.hunks
                    .iter()
                    .map(|h| h.modified_start)
                    .find(|&line| line > current_line)
            })
    }

    /// Navigate to previous change
    pub fn goto_previous_change(&self, id: &DiffSessionId, current_line: u32) -> Option<u32> {
        self.sessions
            .read()
            .iter()
            .find(|s| &s.id == id)
            .and_then(|s| {
                s.hunks
                    .iter()
                    .map(|h| h.modified_start)
                    .rev()
                    .find(|&line| line < current_line)
            })
    }

    /// Accept change from original
    pub fn accept_original(&self, id: &DiffSessionId, hunk_index: usize) {
        let _ = self.events.send(DiffEditorEvent::AcceptedOriginal {
            id: id.clone(),
            hunk_index,
        });
    }

    /// Accept change from modified
    pub fn accept_modified(&self, id: &DiffSessionId, hunk_index: usize) {
        let _ = self.events.send(DiffEditorEvent::AcceptedModified {
            id: id.clone(),
            hunk_index,
        });
    }

    /// Toggle sync scroll
    pub fn toggle_sync_scroll(&self, id: &DiffSessionId) {
        if let Some(session) = self.sessions.write().iter_mut().find(|s| &s.id == id) {
            session.sync_scroll = !session.sync_scroll;
        }
    }
}

impl Default for DiffEditorService {
    fn default() -> Self {
        Self::new()
    }
}

/// Diff session ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiffSessionId(String);

impl DiffSessionId {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(format!("diff-{}", id))
    }
}

impl Default for DiffSessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Diff session
#[derive(Debug, Clone)]
pub struct DiffSession {
    /// Session ID
    pub id: DiffSessionId,
    /// Original side
    pub original: DiffSide,
    /// Modified side
    pub modified: DiffSide,
    /// Diff hunks
    pub hunks: Vec<DiffHunk>,
    /// Sync scroll enabled
    pub sync_scroll: bool,
}

impl DiffSession {
    pub fn change_count(&self) -> usize {
        self.hunks.len()
    }

    pub fn additions(&self) -> usize {
        self.hunks.iter().map(|h| h.added_count as usize).sum()
    }

    pub fn deletions(&self) -> usize {
        self.hunks.iter().map(|h| h.removed_count as usize).sum()
    }
}

/// Diff side
#[derive(Debug, Clone)]
pub enum DiffSide {
    /// File on disk
    File(PathBuf),
    /// In-memory content
    Content {
        content: String,
        label: String,
    },
}

impl DiffSide {
    pub fn label(&self) -> String {
        match self {
            Self::File(path) => path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Original".to_string()),
            Self::Content { label, .. } => label.clone(),
        }
    }
}

/// Diff hunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// Original start line
    pub original_start: u32,
    /// Original line count
    pub original_count: u32,
    /// Modified start line
    pub modified_start: u32,
    /// Modified line count
    pub modified_count: u32,
    /// Added lines count
    pub added_count: u32,
    /// Removed lines count
    pub removed_count: u32,
    /// Hunk lines
    pub lines: Vec<DiffLine>,
}

impl DiffHunk {
    pub fn is_addition(&self) -> bool {
        self.removed_count == 0 && self.added_count > 0
    }

    pub fn is_deletion(&self) -> bool {
        self.added_count == 0 && self.removed_count > 0
    }

    pub fn is_modification(&self) -> bool {
        self.added_count > 0 && self.removed_count > 0
    }
}

/// Diff line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    /// Line content
    pub content: String,
    /// Line type
    pub line_type: DiffLineType,
    /// Original line number (if applicable)
    pub original_line: Option<u32>,
    /// Modified line number (if applicable)
    pub modified_line: Option<u32>,
}

/// Diff line type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffLineType {
    /// Context line (unchanged)
    Context,
    /// Added line
    Added,
    /// Removed line
    Removed,
}

impl DiffLineType {
    pub fn color(&self) -> &'static str {
        match self {
            Self::Context => "editor.foreground",
            Self::Added => "diffEditor.insertedTextBackground",
            Self::Removed => "diffEditor.removedTextBackground",
        }
    }

    pub fn gutter_color(&self) -> &'static str {
        match self {
            Self::Context => "editorGutter.background",
            Self::Added => "editorGutter.addedBackground",
            Self::Removed => "editorGutter.deletedBackground",
        }
    }
}

/// Diff editor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEditorConfig {
    /// Render side by side
    pub side_by_side: bool,
    /// Render indicator
    pub render_indicators: bool,
    /// Ignore whitespace
    pub ignore_whitespace: bool,
    /// Word wrap
    pub word_wrap: bool,
    /// Original editable
    pub original_editable: bool,
    /// Modified editable  
    pub modified_editable: bool,
    /// Diff algorithm
    pub algorithm: DiffAlgorithm,
}

impl Default for DiffEditorConfig {
    fn default() -> Self {
        Self {
            side_by_side: true,
            render_indicators: true,
            ignore_whitespace: false,
            word_wrap: false,
            original_editable: false,
            modified_editable: true,
            algorithm: DiffAlgorithm::Advanced,
        }
    }
}

/// Diff algorithm
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DiffAlgorithm {
    /// Legacy algorithm
    Legacy,
    /// Advanced algorithm
    Advanced,
    /// Experimental
    Experimental,
}

/// Diff editor event
#[derive(Debug, Clone)]
pub enum DiffEditorEvent {
    Opened {
        id: DiffSessionId,
        original: PathBuf,
        modified: PathBuf,
    },
    OpenedWithContent {
        id: DiffSessionId,
        original: PathBuf,
        modified_label: String,
    },
    Closed {
        id: DiffSessionId,
    },
    AcceptedOriginal {
        id: DiffSessionId,
        hunk_index: usize,
    },
    AcceptedModified {
        id: DiffSessionId,
        hunk_index: usize,
    },
}

/// Inline diff decorations
pub struct InlineDiffDecorations;

impl InlineDiffDecorations {
    /// Compute word-level diff
    pub fn compute_word_diff(original: &str, modified: &str) -> Vec<WordDiff> {
        let original_words: Vec<&str> = original.split_whitespace().collect();
        let modified_words: Vec<&str> = modified.split_whitespace().collect();

        // Simple word-level diff (would use proper algorithm)
        let mut diffs = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < original_words.len() || j < modified_words.len() {
            if i < original_words.len() && j < modified_words.len() {
                if original_words[i] == modified_words[j] {
                    diffs.push(WordDiff::Same(original_words[i].to_string()));
                    i += 1;
                    j += 1;
                } else {
                    diffs.push(WordDiff::Removed(original_words[i].to_string()));
                    diffs.push(WordDiff::Added(modified_words[j].to_string()));
                    i += 1;
                    j += 1;
                }
            } else if i < original_words.len() {
                diffs.push(WordDiff::Removed(original_words[i].to_string()));
                i += 1;
            } else {
                diffs.push(WordDiff::Added(modified_words[j].to_string()));
                j += 1;
            }
        }

        diffs
    }
}

/// Word diff
#[derive(Debug, Clone)]
pub enum WordDiff {
    Same(String),
    Added(String),
    Removed(String),
}
