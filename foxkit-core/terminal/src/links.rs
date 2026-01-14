//! Terminal hyperlink detection and management.
//!
//! Provides support for:
//! - OSC 8 hyperlinks (explicit terminal hyperlinks)
//! - Automatic URL/path detection
//! - File path with line number parsing
//! - Custom link handlers

use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// A detected or explicit hyperlink.
#[derive(Debug, Clone, PartialEq)]
pub struct TerminalLink {
    /// The text that was linked.
    pub text: String,
    /// The target URL or path.
    pub target: LinkTarget,
    /// Start position in the line (column).
    pub start_col: usize,
    /// End position in the line (column).
    pub end_col: usize,
    /// Row in the terminal buffer.
    pub row: usize,
    /// Optional link ID (for OSC 8 links).
    pub id: Option<String>,
    /// Whether this is an explicit OSC 8 link.
    pub explicit: bool,
}

/// Target of a terminal link.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// Web URL (http/https).
    Url(String),
    /// File path, optionally with line and column.
    File {
        path: PathBuf,
        line: Option<u32>,
        column: Option<u32>,
    },
    /// Custom protocol handler.
    Custom {
        protocol: String,
        data: String,
    },
}

impl LinkTarget {
    /// Create a URL target.
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url(url.into())
    }

    /// Create a file target.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::File {
            path: path.into(),
            line: None,
            column: None,
        }
    }

    /// Create a file target with line number.
    pub fn file_with_line(path: impl Into<PathBuf>, line: u32) -> Self {
        Self::File {
            path: path.into(),
            line: Some(line),
            column: None,
        }
    }

    /// Create a file target with line and column.
    pub fn file_with_position(path: impl Into<PathBuf>, line: u32, column: u32) -> Self {
        Self::File {
            path: path.into(),
            line: Some(line),
            column: Some(column),
        }
    }

    /// Convert to a URI string.
    pub fn to_uri(&self) -> String {
        match self {
            Self::Url(url) => url.clone(),
            Self::File { path, line, column } => {
                let mut uri = format!("file://{}", path.display());
                if let Some(l) = line {
                    uri.push_str(&format!(":{}", l));
                    if let Some(c) = column {
                        uri.push_str(&format!(":{}", c));
                    }
                }
                uri
            }
            Self::Custom { protocol, data } => format!("{}:{}", protocol, data),
        }
    }
}

/// Configuration for link detection.
#[derive(Debug, Clone)]
pub struct LinkDetectorConfig {
    /// Whether to detect URLs automatically.
    pub detect_urls: bool,
    /// Whether to detect file paths automatically.
    pub detect_file_paths: bool,
    /// Working directory for resolving relative paths.
    pub working_dir: Option<PathBuf>,
    /// Custom patterns to detect as links.
    pub custom_patterns: Vec<LinkPattern>,
    /// URL schemes to recognize.
    pub url_schemes: Vec<String>,
    /// Whether to validate file paths exist.
    pub validate_paths: bool,
}

impl Default for LinkDetectorConfig {
    fn default() -> Self {
        Self {
            detect_urls: true,
            detect_file_paths: true,
            working_dir: None,
            custom_patterns: Vec::new(),
            url_schemes: vec![
                "http".into(),
                "https".into(),
                "ftp".into(),
                "ftps".into(),
                "mailto".into(),
                "file".into(),
            ],
            validate_paths: false,
        }
    }
}

/// A custom link pattern.
#[derive(Debug, Clone)]
pub struct LinkPattern {
    /// Name of this pattern.
    pub name: String,
    /// Regex pattern to match.
    pub pattern: String,
    /// Compiled regex.
    regex: Option<Regex>,
    /// How to build the target from the match.
    pub target_template: String,
}

impl LinkPattern {
    /// Create a new link pattern.
    pub fn new(name: impl Into<String>, pattern: impl Into<String>, target: impl Into<String>) -> Self {
        let pattern_str = pattern.into();
        let regex = Regex::new(&pattern_str).ok();
        Self {
            name: name.into(),
            pattern: pattern_str,
            regex,
            target_template: target.into(),
        }
    }

    /// Check if this pattern matches the text.
    pub fn find_matches(&self, text: &str) -> Vec<(usize, usize, String)> {
        let Some(ref regex) = self.regex else {
            return Vec::new();
        };

        regex
            .find_iter(text)
            .map(|m| {
                let target = self.build_target(m.as_str());
                (m.start(), m.end(), target)
            })
            .collect()
    }

    /// Build target URL from matched text.
    fn build_target(&self, matched: &str) -> String {
        self.target_template.replace("$0", matched)
    }
}

// Common regex patterns for link detection.
static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(https?://|ftp://|file://)[^\s<>\[\]{}|\\^`\x00-\x1f\x7f]+").unwrap()
});

static FILE_PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match file paths with optional line:column
    // Examples: /path/to/file.rs:10:5, ./src/main.rs:42, file.ts(10,5)
    Regex::new(r"(?x)
        # Unix-style paths or relative paths
        (?:
            (?:/[^\s:,]+)|                    # Absolute path
            (?:\./[^\s:,]+)|                  # Relative path with ./
            (?:\.\./[^\s:,]+)|                # Relative path with ../
            (?:[a-zA-Z_][a-zA-Z0-9_./\-]+\.[a-zA-Z]{1,10})  # file.ext style
        )
        # Optional line number
        (?:
            (?::(\d+)(?::(\d+))?)|            # :line:col
            (?:\((\d+)(?:,\s*(\d+))?\))       # (line, col)
        )?
    ").unwrap()
});

/// Terminal link detector.
pub struct LinkDetector {
    config: LinkDetectorConfig,
}

impl LinkDetector {
    /// Create a new link detector with default config.
    pub fn new() -> Self {
        Self {
            config: LinkDetectorConfig::default(),
        }
    }

    /// Create a link detector with custom config.
    pub fn with_config(config: LinkDetectorConfig) -> Self {
        Self { config }
    }

    /// Set the working directory.
    pub fn set_working_dir(&mut self, dir: impl Into<PathBuf>) {
        self.config.working_dir = Some(dir.into());
    }

    /// Add a custom pattern.
    pub fn add_pattern(&mut self, pattern: LinkPattern) {
        self.config.custom_patterns.push(pattern);
    }

    /// Detect links in a line of text.
    pub fn detect_links(&self, text: &str, row: usize) -> Vec<TerminalLink> {
        let mut links = Vec::new();

        // Detect URLs
        if self.config.detect_urls {
            for url_match in URL_REGEX.find_iter(text) {
                links.push(TerminalLink {
                    text: url_match.as_str().to_string(),
                    target: LinkTarget::Url(url_match.as_str().to_string()),
                    start_col: url_match.start(),
                    end_col: url_match.end(),
                    row,
                    id: None,
                    explicit: false,
                });
            }
        }

        // Detect file paths
        if self.config.detect_file_paths {
            for caps in FILE_PATH_REGEX.captures_iter(text) {
                if let Some(path_match) = caps.get(0) {
                    let path_str = path_match.as_str();
                    
                    // Extract line and column if present
                    let (path, line, col) = self.parse_path_with_location(path_str);
                    
                    // Resolve relative paths
                    let resolved = self.resolve_path(&path);
                    
                    // Optionally validate path exists
                    if self.config.validate_paths && !resolved.exists() {
                        continue;
                    }

                    links.push(TerminalLink {
                        text: path_str.to_string(),
                        target: LinkTarget::File {
                            path: resolved,
                            line,
                            column: col,
                        },
                        start_col: path_match.start(),
                        end_col: path_match.end(),
                        row,
                        id: None,
                        explicit: false,
                    });
                }
            }
        }

        // Apply custom patterns
        for pattern in &self.config.custom_patterns {
            for (start, end, target) in pattern.find_matches(text) {
                // Skip if overlapping with existing link
                let overlaps = links.iter().any(|l| {
                    (start >= l.start_col && start < l.end_col)
                        || (end > l.start_col && end <= l.end_col)
                });
                
                if !overlaps {
                    links.push(TerminalLink {
                        text: text[start..end].to_string(),
                        target: LinkTarget::Custom {
                            protocol: pattern.name.clone(),
                            data: target,
                        },
                        start_col: start,
                        end_col: end,
                        row,
                        id: None,
                        explicit: false,
                    });
                }
            }
        }

        // Sort by position
        links.sort_by_key(|l| l.start_col);
        links
    }

    /// Parse a path that may contain line:col information.
    fn parse_path_with_location(&self, text: &str) -> (String, Option<u32>, Option<u32>) {
        // Try :line:col format
        if let Some(colon_idx) = text.rfind(':') {
            let before_colon = &text[..colon_idx];
            let after_colon = &text[colon_idx + 1..];
            
            if let Ok(line) = after_colon.parse::<u32>() {
                // Check if there's another colon for column
                if let Some(second_colon) = before_colon.rfind(':') {
                    let path = &before_colon[..second_colon];
                    let line_str = &before_colon[second_colon + 1..];
                    if let Ok(actual_line) = line_str.parse::<u32>() {
                        return (path.to_string(), Some(actual_line), Some(line));
                    }
                }
                return (before_colon.to_string(), Some(line), None);
            }
        }

        // Try (line, col) format
        if let Some(paren_start) = text.rfind('(') {
            if text.ends_with(')') {
                let path = &text[..paren_start];
                let coords = &text[paren_start + 1..text.len() - 1];
                
                let parts: Vec<&str> = coords.split(',').collect();
                if let Ok(line) = parts[0].trim().parse::<u32>() {
                    let col = parts.get(1).and_then(|c| c.trim().parse::<u32>().ok());
                    return (path.to_string(), Some(line), col);
                }
            }
        }

        (text.to_string(), None, None)
    }

    /// Resolve a path relative to working directory.
    fn resolve_path(&self, path: &str) -> PathBuf {
        let path_buf = PathBuf::from(path);
        
        if path_buf.is_absolute() {
            return path_buf;
        }

        if let Some(ref cwd) = self.config.working_dir {
            return cwd.join(&path_buf);
        }

        path_buf
    }

    /// Get link at a specific position.
    pub fn link_at(&self, text: &str, row: usize, col: usize) -> Option<TerminalLink> {
        let links = self.detect_links(text, row);
        links.into_iter().find(|l| col >= l.start_col && col < l.end_col)
    }
}

impl Default for LinkDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// OSC 8 hyperlink state tracker.
/// 
/// Tracks active hyperlinks set via OSC 8 escape sequences.
pub struct HyperlinkState {
    /// Currently active hyperlink (if any).
    current: Option<ActiveHyperlink>,
    /// All hyperlinks by ID.
    links_by_id: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct ActiveHyperlink {
    url: String,
    id: Option<String>,
    params: HashMap<String, String>,
}

impl HyperlinkState {
    /// Create a new hyperlink state tracker.
    pub fn new() -> Self {
        Self {
            current: None,
            links_by_id: HashMap::new(),
        }
    }

    /// Set the current hyperlink from OSC 8 sequence.
    /// 
    /// Format: OSC 8 ; params ; url ST
    pub fn set_hyperlink(&mut self, params_str: &str, url: &str) {
        if url.is_empty() {
            // Empty URL closes the hyperlink
            self.current = None;
            return;
        }

        let mut params = HashMap::new();
        for param in params_str.split(':') {
            if let Some((key, value)) = param.split_once('=') {
                params.insert(key.to_string(), value.to_string());
            }
        }

        let id = params.get("id").cloned();
        if let Some(ref id) = id {
            self.links_by_id.insert(id.clone(), url.to_string());
        }

        self.current = Some(ActiveHyperlink {
            url: url.to_string(),
            id,
            params,
        });
    }

    /// Get the current hyperlink URL.
    pub fn current_url(&self) -> Option<&str> {
        self.current.as_ref().map(|h| h.url.as_str())
    }

    /// Get the current hyperlink ID.
    pub fn current_id(&self) -> Option<&str> {
        self.current.as_ref().and_then(|h| h.id.as_deref())
    }

    /// Check if there's an active hyperlink.
    pub fn is_active(&self) -> bool {
        self.current.is_some()
    }

    /// Clear the current hyperlink.
    pub fn clear(&mut self) {
        self.current = None;
    }

    /// Get URL by link ID.
    pub fn url_by_id(&self, id: &str) -> Option<&str> {
        self.links_by_id.get(id).map(|s| s.as_str())
    }
}

impl Default for HyperlinkState {
    fn default() -> Self {
        Self::new()
    }
}

/// Common link patterns for various tools.
pub mod patterns {
    use super::LinkPattern;

    /// Create a pattern for Rust compiler errors.
    /// 
    /// Matches: --> src/main.rs:10:5
    pub fn rust_compiler() -> LinkPattern {
        LinkPattern::new(
            "rust_compiler",
            r"-->\s+([^\s]+:\d+:\d+)",
            "file://$0",
        )
    }

    /// Create a pattern for Node.js stack traces.
    /// 
    /// Matches: at Function (/path/to/file.js:10:5)
    pub fn node_stack_trace() -> LinkPattern {
        LinkPattern::new(
            "node_stack",
            r"at\s+.+\s+\(([^)]+:\d+:\d+)\)",
            "file://$1",
        )
    }

    /// Create a pattern for Python stack traces.
    /// 
    /// Matches: File "/path/to/file.py", line 10
    pub fn python_stack_trace() -> LinkPattern {
        LinkPattern::new(
            "python_stack",
            r#"File\s+"([^"]+)",\s+line\s+(\d+)"#,
            "file://$1:$2",
        )
    }

    /// Create a pattern for Go compiler errors.
    /// 
    /// Matches: ./main.go:10:5: error message
    pub fn go_compiler() -> LinkPattern {
        LinkPattern::new(
            "go_compiler",
            r"(\.?/[^\s:]+:\d+(?::\d+)?)",
            "file://$0",
        )
    }

    /// Create a pattern for TypeScript compiler errors.
    /// 
    /// Matches: src/index.ts(10,5): error TS1234
    pub fn typescript_compiler() -> LinkPattern {
        LinkPattern::new(
            "typescript_compiler",
            r"([^\s(]+)\((\d+),(\d+)\):",
            "file://$1:$2:$3",
        )
    }

    /// Create a pattern for git diff headers.
    /// 
    /// Matches: +++ b/src/main.rs
    pub fn git_diff() -> LinkPattern {
        LinkPattern::new(
            "git_diff",
            r"\+\+\+\s+[ab]/(.+)",
            "file://$1",
        )
    }

    /// Get all common patterns.
    pub fn all() -> Vec<LinkPattern> {
        vec![
            rust_compiler(),
            node_stack_trace(),
            python_stack_trace(),
            go_compiler(),
            typescript_compiler(),
            git_diff(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_url() {
        let detector = LinkDetector::new();
        let links = detector.detect_links("Visit https://example.com for more info", 0);
        
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "https://example.com");
        assert!(matches!(links[0].target, LinkTarget::Url(_)));
    }

    #[test]
    fn test_detect_file_path() {
        let detector = LinkDetector::new();
        let links = detector.detect_links("Error in /path/to/file.rs:42:10", 0);
        
        assert!(links.iter().any(|l| matches!(&l.target, LinkTarget::File { line: Some(42), .. })));
    }

    #[test]
    fn test_link_target_uri() {
        let target = LinkTarget::file_with_position("/src/main.rs", 10, 5);
        assert_eq!(target.to_uri(), "file:///src/main.rs:10:5");
    }

    #[test]
    fn test_hyperlink_state() {
        let mut state = HyperlinkState::new();
        
        assert!(!state.is_active());
        
        state.set_hyperlink("id=link1", "https://example.com");
        assert!(state.is_active());
        assert_eq!(state.current_url(), Some("https://example.com"));
        assert_eq!(state.current_id(), Some("link1"));
        
        state.set_hyperlink("", "");
        assert!(!state.is_active());
    }

    #[test]
    fn test_path_with_parens() {
        let detector = LinkDetector::new();
        let (path, line, col) = detector.parse_path_with_location("file.ts(10, 5)");
        
        assert_eq!(path, "file.ts");
        assert_eq!(line, Some(10));
        assert_eq!(col, Some(5));
    }
}
