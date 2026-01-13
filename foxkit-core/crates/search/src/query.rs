//! Search query types

use regex::Regex;

/// A search query
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Search pattern
    pub pattern: String,
    /// Is regex search
    pub is_regex: bool,
    /// Case sensitive
    pub case_sensitive: bool,
    /// Whole word match
    pub whole_word: bool,
    /// Files to include (glob patterns)
    pub include: Vec<String>,
    /// Files to exclude (glob patterns)
    pub exclude: Vec<String>,
}

impl SearchQuery {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            is_regex: false,
            case_sensitive: false,
            whole_word: false,
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }

    pub fn regex(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            is_regex: true,
            case_sensitive: false,
            whole_word: false,
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }

    pub fn with_case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    pub fn with_whole_word(mut self, whole_word: bool) -> Self {
        self.whole_word = whole_word;
        self
    }

    pub fn with_include(mut self, patterns: Vec<String>) -> Self {
        self.include = patterns;
        self
    }

    pub fn with_exclude(mut self, patterns: Vec<String>) -> Self {
        self.exclude = patterns;
        self
    }

    /// Build regex from query
    pub fn to_regex(&self) -> Result<Regex, regex::Error> {
        let mut pattern = if self.is_regex {
            self.pattern.clone()
        } else {
            regex::escape(&self.pattern)
        };

        if self.whole_word {
            pattern = format!(r"\b{}\b", pattern);
        }

        if self.case_sensitive {
            Regex::new(&pattern)
        } else {
            Regex::new(&format!("(?i){}", pattern))
        }
    }

    /// Check if text matches
    pub fn matches(&self, text: &str) -> bool {
        if let Ok(regex) = self.to_regex() {
            regex.is_match(text)
        } else {
            false
        }
    }

    /// Find all matches in text
    pub fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        if let Ok(regex) = self.to_regex() {
            regex.find_iter(text)
                .map(|m| (m.start(), m.end()))
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Max results per file
    pub max_results_per_file: usize,
    /// Max total results
    pub max_results: usize,
    /// Search hidden files
    pub include_hidden: bool,
    /// Follow symlinks
    pub follow_symlinks: bool,
    /// Context lines before match
    pub context_before: usize,
    /// Context lines after match
    pub context_after: usize,
    /// Max file size to search (bytes)
    pub max_file_size: u64,
    /// Number of parallel threads
    pub threads: usize,
    /// Binary file handling
    pub binary: BinaryHandling,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            max_results_per_file: 1000,
            max_results: 10000,
            include_hidden: false,
            follow_symlinks: false,
            context_before: 0,
            context_after: 0,
            max_file_size: 50 * 1024 * 1024, // 50MB
            threads: num_cpus::get(),
            binary: BinaryHandling::Skip,
        }
    }
}

/// How to handle binary files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryHandling {
    /// Skip binary files
    Skip,
    /// Search as text
    AsText,
    /// Only report if match found
    ReportOnly,
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
