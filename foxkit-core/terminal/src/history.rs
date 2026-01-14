//! Terminal command history management.
//!
//! Provides command history with:
//! - Persistent storage across sessions
//! - Reverse search (Ctrl+R style)
//! - Deduplication options
//! - Per-directory history

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// A single history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// The command that was executed.
    pub command: String,
    /// When the command was executed (Unix timestamp).
    pub timestamp: u64,
    /// Working directory where command was run.
    pub cwd: Option<PathBuf>,
    /// Exit code of the command (if known).
    pub exit_code: Option<i32>,
    /// Duration in milliseconds (if known).
    pub duration_ms: Option<u64>,
    /// Shell used to execute the command.
    pub shell: Option<String>,
}

impl HistoryEntry {
    /// Create a new history entry for a command.
    pub fn new(command: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        Self {
            command: command.into(),
            timestamp,
            cwd: None,
            exit_code: None,
            duration_ms: None,
            shell: None,
        }
    }

    /// Set the working directory.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set the shell.
    pub fn with_shell(mut self, shell: impl Into<String>) -> Self {
        self.shell = Some(shell.into());
        self
    }

    /// Set the exit code.
    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.exit_code = Some(code);
        self
    }

    /// Set the duration.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}

/// History configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    /// Maximum number of entries to keep.
    pub max_entries: usize,
    /// Whether to deduplicate consecutive identical commands.
    pub dedupe_consecutive: bool,
    /// Whether to deduplicate all identical commands (keep most recent).
    pub dedupe_all: bool,
    /// Whether to ignore commands starting with space.
    pub ignore_space_prefix: bool,
    /// Commands to ignore (patterns).
    pub ignore_patterns: Vec<String>,
    /// Whether to save history to disk.
    pub persist: bool,
    /// History file path.
    pub history_file: Option<PathBuf>,
    /// Whether to enable per-directory history.
    pub per_directory: bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            dedupe_consecutive: true,
            dedupe_all: false,
            ignore_space_prefix: true,
            ignore_patterns: vec![
                "^\\s*$".to_string(), // Empty/whitespace only
            ],
            persist: true,
            history_file: None,
            per_directory: false,
        }
    }
}

/// Search result from history.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matching entry.
    pub entry: HistoryEntry,
    /// Index in history (0 = most recent).
    pub index: usize,
    /// Match positions in the command string.
    pub match_positions: Vec<(usize, usize)>,
}

/// Search mode for history lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// Search from most recent to oldest.
    Backward,
    /// Search from oldest to most recent.
    Forward,
    /// Prefix match only.
    Prefix,
    /// Substring match anywhere in command.
    Substring,
    /// Fuzzy matching.
    Fuzzy,
}

/// Terminal command history manager.
pub struct History {
    /// History entries (newest first).
    entries: VecDeque<HistoryEntry>,
    /// Configuration.
    config: HistoryConfig,
    /// Current navigation position (for up/down arrow).
    position: Option<usize>,
    /// Per-directory history cache.
    dir_history: HashMap<PathBuf, VecDeque<HistoryEntry>>,
    /// Compiled ignore patterns.
    #[allow(dead_code)]
    ignore_regex: Vec<regex::Regex>,
}

impl History {
    /// Create a new history manager with default config.
    pub fn new() -> Self {
        Self::with_config(HistoryConfig::default())
    }

    /// Create a new history manager with custom config.
    pub fn with_config(config: HistoryConfig) -> Self {
        let ignore_regex = config
            .ignore_patterns
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect();

        Self {
            entries: VecDeque::with_capacity(config.max_entries),
            config,
            position: None,
            dir_history: HashMap::new(),
            ignore_regex,
        }
    }

    /// Add a command to history.
    pub fn add(&mut self, entry: HistoryEntry) {
        // Check if command should be ignored
        if self.should_ignore(&entry.command) {
            return;
        }

        // Handle deduplication
        if self.config.dedupe_consecutive {
            if let Some(last) = self.entries.front() {
                if last.command == entry.command {
                    return;
                }
            }
        }

        if self.config.dedupe_all {
            self.entries.retain(|e| e.command != entry.command);
        }

        // Add to main history
        self.entries.push_front(entry.clone());

        // Trim if over capacity
        while self.entries.len() > self.config.max_entries {
            self.entries.pop_back();
        }

        // Add to directory history if enabled
        if self.config.per_directory {
            if let Some(ref cwd) = entry.cwd {
                let dir_entries = self.dir_history.entry(cwd.clone()).or_default();
                if self.config.dedupe_consecutive {
                    if let Some(last) = dir_entries.front() {
                        if last.command == entry.command {
                            return;
                        }
                    }
                }
                dir_entries.push_front(entry);
                while dir_entries.len() > self.config.max_entries / 10 {
                    dir_entries.pop_back();
                }
            }
        }

        // Reset navigation position
        self.position = None;
    }

    /// Add a simple command string to history.
    pub fn add_command(&mut self, command: impl Into<String>) {
        self.add(HistoryEntry::new(command));
    }

    /// Check if a command should be ignored.
    fn should_ignore(&self, command: &str) -> bool {
        // Ignore empty commands
        if command.trim().is_empty() {
            return true;
        }

        // Ignore commands starting with space
        if self.config.ignore_space_prefix && command.starts_with(' ') {
            return true;
        }

        // Check against ignore patterns
        for regex in &self.ignore_regex {
            if regex.is_match(command) {
                return true;
            }
        }

        false
    }

    /// Get all history entries.
    pub fn entries(&self) -> impl Iterator<Item = &HistoryEntry> {
        self.entries.iter()
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.dir_history.clear();
        self.position = None;
    }

    /// Navigate to previous command (up arrow).
    pub fn previous(&mut self) -> Option<&HistoryEntry> {
        if self.entries.is_empty() {
            return None;
        }

        let new_pos = match self.position {
            None => 0,
            Some(pos) => (pos + 1).min(self.entries.len() - 1),
        };

        self.position = Some(new_pos);
        self.entries.get(new_pos)
    }

    /// Navigate to next command (down arrow).
    pub fn next(&mut self) -> Option<&HistoryEntry> {
        match self.position {
            None => None,
            Some(0) => {
                self.position = None;
                None
            }
            Some(pos) => {
                self.position = Some(pos - 1);
                self.entries.get(pos - 1)
            }
        }
    }

    /// Reset navigation position.
    pub fn reset_position(&mut self) {
        self.position = None;
    }

    /// Get current navigation position.
    pub fn current_position(&self) -> Option<usize> {
        self.position
    }

    /// Search history for matching commands.
    pub fn search(&self, query: &str, mode: SearchMode, max_results: usize) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for (index, entry) in self.entries.iter().enumerate() {
            let command_lower = entry.command.to_lowercase();
            
            let matches = match mode {
                SearchMode::Prefix => {
                    if command_lower.starts_with(&query_lower) {
                        Some(vec![(0, query.len())])
                    } else {
                        None
                    }
                }
                SearchMode::Substring | SearchMode::Backward | SearchMode::Forward => {
                    if let Some(pos) = command_lower.find(&query_lower) {
                        Some(vec![(pos, pos + query.len())])
                    } else {
                        None
                    }
                }
                SearchMode::Fuzzy => self.fuzzy_match(&command_lower, &query_lower),
            };

            if let Some(match_positions) = matches {
                results.push(SearchResult {
                    entry: entry.clone(),
                    index,
                    match_positions,
                });

                if results.len() >= max_results {
                    break;
                }
            }
        }

        if mode == SearchMode::Forward {
            results.reverse();
        }

        results
    }

    /// Perform fuzzy matching.
    fn fuzzy_match(&self, text: &str, pattern: &str) -> Option<Vec<(usize, usize)>> {
        let mut positions = Vec::new();
        let mut text_idx = 0;
        let text_chars: Vec<char> = text.chars().collect();
        
        for pattern_char in pattern.chars() {
            let mut found = false;
            while text_idx < text_chars.len() {
                if text_chars[text_idx] == pattern_char {
                    positions.push((text_idx, text_idx + 1));
                    text_idx += 1;
                    found = true;
                    break;
                }
                text_idx += 1;
            }
            if !found {
                return None;
            }
        }

        Some(positions)
    }

    /// Reverse incremental search (like Ctrl+R in bash).
    pub fn reverse_search(&self, query: &str) -> Option<SearchResult> {
        self.search(query, SearchMode::Substring, 1).into_iter().next()
    }

    /// Get history for a specific directory.
    pub fn directory_history(&self, dir: &Path) -> impl Iterator<Item = &HistoryEntry> {
        self.dir_history
            .get(dir)
            .map(|entries| entries.iter())
            .into_iter()
            .flatten()
    }

    /// Load history from a file.
    pub fn load(&mut self, path: &Path) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        
        // Try JSON format first
        if let Ok(entries) = serde_json::from_str::<Vec<HistoryEntry>>(&content) {
            self.entries = VecDeque::from(entries);
            return Ok(());
        }

        // Fall back to simple line format (one command per line)
        self.entries.clear();
        for line in content.lines().rev() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                self.entries.push_front(HistoryEntry::new(trimmed));
            }
        }

        Ok(())
    }

    /// Save history to a file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&Vec::from(self.entries.clone()))?;
        std::fs::write(path, json)
    }

    /// Export history to simple text format.
    pub fn export_text(&self) -> String {
        self.entries
            .iter()
            .rev()
            .map(|e| e.command.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Import from shell history file format.
    pub fn import_shell_history(&mut self, content: &str, shell: &str) {
        match shell {
            "zsh" => self.import_zsh_history(content),
            "fish" => self.import_fish_history(content),
            _ => self.import_bash_history(content),
        }
    }

    fn import_bash_history(&mut self, content: &str) {
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                self.add(HistoryEntry::new(trimmed).with_shell("bash"));
            }
        }
    }

    fn import_zsh_history(&mut self, content: &str) {
        for line in content.lines() {
            // zsh format: : timestamp:0;command
            if let Some(command) = line.strip_prefix(": ") {
                if let Some((_meta, cmd)) = command.split_once(';') {
                    self.add(HistoryEntry::new(cmd).with_shell("zsh"));
                }
            } else if !line.is_empty() {
                self.add(HistoryEntry::new(line).with_shell("zsh"));
            }
        }
    }

    fn import_fish_history(&mut self, content: &str) {
        // fish format: - cmd: command
        let mut current_cmd: Option<String> = None;
        
        for line in content.lines() {
            if let Some(cmd) = line.strip_prefix("- cmd: ") {
                if let Some(prev) = current_cmd.take() {
                    self.add(HistoryEntry::new(prev).with_shell("fish"));
                }
                current_cmd = Some(cmd.to_string());
            } else if line.starts_with("  when: ") {
                // Timestamp - we could parse this
            }
        }
        
        if let Some(cmd) = current_cmd {
            self.add(HistoryEntry::new(cmd).with_shell("fish"));
        }
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared history manager for multiple terminals.
pub struct SharedHistory {
    inner: Arc<RwLock<History>>,
}

impl SharedHistory {
    /// Create a new shared history.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(History::new())),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: HistoryConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(History::with_config(config))),
        }
    }

    /// Add a command to history.
    pub fn add(&self, entry: HistoryEntry) {
        self.inner.write().add(entry);
    }

    /// Add a simple command.
    pub fn add_command(&self, command: impl Into<String>) {
        self.inner.write().add_command(command);
    }

    /// Search history.
    pub fn search(&self, query: &str, mode: SearchMode, max_results: usize) -> Vec<SearchResult> {
        self.inner.read().search(query, mode, max_results)
    }

    /// Get number of entries.
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    /// Clone the Arc reference.
    pub fn clone_ref(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Load from file.
    pub fn load(&self, path: &Path) -> std::io::Result<()> {
        self.inner.write().load(path)
    }

    /// Save to file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        self.inner.read().save(path)
    }
}

impl Default for SharedHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SharedHistory {
    fn clone(&self) -> Self {
        self.clone_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_retrieve() {
        let mut history = History::new();
        history.add_command("ls -la");
        history.add_command("cd /tmp");
        history.add_command("echo hello");
        
        assert_eq!(history.len(), 3);
        assert_eq!(history.entries().next().unwrap().command, "echo hello");
    }

    #[test]
    fn test_dedupe_consecutive() {
        let mut config = HistoryConfig::default();
        config.dedupe_consecutive = true;
        
        let mut history = History::with_config(config);
        history.add_command("ls");
        history.add_command("ls");
        history.add_command("ls");
        
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_navigation() {
        let mut history = History::new();
        history.add_command("cmd1");
        history.add_command("cmd2");
        history.add_command("cmd3");
        
        assert_eq!(history.previous().map(|e| e.command.as_str()), Some("cmd3"));
        assert_eq!(history.previous().map(|e| e.command.as_str()), Some("cmd2"));
        assert_eq!(history.previous().map(|e| e.command.as_str()), Some("cmd1"));
        assert_eq!(history.next().map(|e| e.command.as_str()), Some("cmd2"));
    }

    #[test]
    fn test_search() {
        let mut history = History::new();
        history.add_command("git commit -m 'test'");
        history.add_command("git push");
        history.add_command("cargo build");
        history.add_command("git status");
        
        let results = history.search("git", SearchMode::Substring, 10);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_fuzzy_search() {
        let mut history = History::new();
        history.add_command("docker-compose up -d");
        
        let results = history.search("dcu", SearchMode::Fuzzy, 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_ignore_space_prefix() {
        let mut history = History::new();
        history.add_command(" secret-command");
        history.add_command("normal-command");
        
        assert_eq!(history.len(), 1);
        assert_eq!(history.entries().next().unwrap().command, "normal-command");
    }
}
