//! Search result types

use std::path::PathBuf;

/// Search result
#[derive(Debug, Clone)]
pub enum SearchResult {
    /// File match
    Match(FileMatch),
    /// Search completed
    Done(SearchStats),
    /// Error occurred
    Error(SearchError),
    /// Progress update
    Progress(SearchProgress),
}

/// A match in a file
#[derive(Debug, Clone)]
pub struct FileMatch {
    /// File path
    pub path: PathBuf,
    /// Matches in this file
    pub matches: Vec<Match>,
}

impl FileMatch {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            matches: Vec::new(),
        }
    }

    pub fn add_match(&mut self, m: Match) {
        self.matches.push(m);
    }

    pub fn count(&self) -> usize {
        self.matches.len()
    }
}

/// A single match
#[derive(Debug, Clone)]
pub struct Match {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Length of match
    pub length: usize,
    /// Byte offset in file
    pub offset: usize,
    /// Line text
    pub text: String,
    /// Context lines before
    pub context_before: Vec<ContextLine>,
    /// Context lines after
    pub context_after: Vec<ContextLine>,
}

impl Match {
    pub fn new(line: usize, column: usize, length: usize, text: String) -> Self {
        Self {
            line,
            column,
            length,
            offset: 0,
            text,
            context_before: Vec::new(),
            context_after: Vec::new(),
        }
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Get the matched text
    pub fn matched_text(&self) -> &str {
        let start = self.column.saturating_sub(1);
        let end = (start + self.length).min(self.text.len());
        &self.text[start..end]
    }

    /// Text before the match on same line
    pub fn prefix(&self) -> &str {
        let end = self.column.saturating_sub(1);
        &self.text[..end]
    }

    /// Text after the match on same line
    pub fn suffix(&self) -> &str {
        let start = (self.column.saturating_sub(1) + self.length).min(self.text.len());
        &self.text[start..]
    }
}

/// A context line
#[derive(Debug, Clone)]
pub struct ContextLine {
    pub line: usize,
    pub text: String,
}

/// Search statistics
#[derive(Debug, Clone, Default)]
pub struct SearchStats {
    /// Files searched
    pub files_searched: usize,
    /// Files matched
    pub files_matched: usize,
    /// Total matches
    pub total_matches: usize,
    /// Files skipped
    pub files_skipped: usize,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl SearchStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn files_per_second(&self) -> f64 {
        if self.duration_ms == 0 {
            0.0
        } else {
            self.files_searched as f64 / (self.duration_ms as f64 / 1000.0)
        }
    }
}

/// Search error
#[derive(Debug, Clone)]
pub struct SearchError {
    pub path: Option<PathBuf>,
    pub message: String,
}

impl SearchError {
    pub fn new(message: &str) -> Self {
        Self {
            path: None,
            message: message.to_string(),
        }
    }

    pub fn for_file(path: PathBuf, message: &str) -> Self {
        Self {
            path: Some(path),
            message: message.to_string(),
        }
    }
}

/// Search progress
#[derive(Debug, Clone)]
pub struct SearchProgress {
    /// Files searched so far
    pub files_searched: usize,
    /// Current file being searched
    pub current_file: Option<PathBuf>,
    /// Matches found so far
    pub matches_found: usize,
}
