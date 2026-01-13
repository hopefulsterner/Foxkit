//! # Foxkit Search
//!
//! Fast text and file search with ripgrep-like performance.

pub mod query;
pub mod result;
pub mod file_search;
pub mod text_search;
pub mod replace;

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use crossbeam_channel::{Sender, Receiver, unbounded};

pub use query::{SearchQuery, SearchOptions};
pub use result::{SearchResult, Match, FileMatch};
pub use file_search::FileSearcher;
pub use text_search::TextSearcher;
pub use replace::Replacer;

/// Search engine
pub struct SearchEngine {
    /// Results channel sender
    tx: Sender<SearchResult>,
    /// Results channel receiver
    rx: Receiver<SearchResult>,
    /// Active search handle
    active: Option<SearchHandle>,
    /// Search options
    options: SearchOptions,
}

impl SearchEngine {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            tx,
            rx,
            active: None,
            options: SearchOptions::default(),
        }
    }

    /// Search text in files
    pub fn search(&mut self, query: SearchQuery, paths: Vec<PathBuf>) -> SearchHandle {
        // Cancel any active search
        if let Some(handle) = self.active.take() {
            handle.cancel();
        }

        let handle = SearchHandle::new();
        let handle_clone = handle.clone();
        let tx = self.tx.clone();
        let options = self.options.clone();

        std::thread::spawn(move || {
            let searcher = TextSearcher::new(query, options);
            searcher.search(paths, tx, handle_clone);
        });

        self.active = Some(handle.clone());
        handle
    }

    /// Search for files by name
    pub fn search_files(&mut self, pattern: &str, root: PathBuf) -> SearchHandle {
        if let Some(handle) = self.active.take() {
            handle.cancel();
        }

        let handle = SearchHandle::new();
        let handle_clone = handle.clone();
        let tx = self.tx.clone();
        let pattern = pattern.to_string();

        std::thread::spawn(move || {
            let searcher = FileSearcher::new(&pattern);
            searcher.search(root, tx, handle_clone);
        });

        self.active = Some(handle.clone());
        handle
    }

    /// Get results receiver
    pub fn results(&self) -> &Receiver<SearchResult> {
        &self.rx
    }

    /// Cancel active search
    pub fn cancel(&mut self) {
        if let Some(handle) = self.active.take() {
            handle.cancel();
        }
    }

    /// Set search options
    pub fn set_options(&mut self, options: SearchOptions) {
        self.options = options;
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to control/cancel search
#[derive(Debug, Clone)]
pub struct SearchHandle {
    cancelled: Arc<RwLock<bool>>,
    completed: Arc<RwLock<bool>>,
}

impl SearchHandle {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(RwLock::new(false)),
            completed: Arc::new(RwLock::new(false)),
        }
    }

    pub fn cancel(&self) {
        *self.cancelled.write() = true;
    }

    pub fn is_cancelled(&self) -> bool {
        *self.cancelled.read()
    }

    pub fn complete(&self) {
        *self.completed.write() = true;
    }

    pub fn is_completed(&self) -> bool {
        *self.completed.read()
    }
}

impl Default for SearchHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick search function
pub fn search(pattern: &str, path: PathBuf) -> Vec<FileMatch> {
    let (tx, rx) = unbounded();
    let query = SearchQuery::new(pattern);
    let handle = SearchHandle::new();
    
    let searcher = TextSearcher::new(query, SearchOptions::default());
    searcher.search(vec![path], tx, handle);
    
    let mut matches = Vec::new();
    while let Ok(result) = rx.recv() {
        if let SearchResult::Match(m) = result {
            matches.push(m);
        }
    }
    
    matches
}
