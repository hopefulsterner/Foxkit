//! Git blame

use std::path::Path;

/// Blame information for a file
#[derive(Debug, Clone)]
pub struct Blame {
    pub path: String,
    pub lines: Vec<BlameLine>,
}

impl Blame {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            lines: Vec::new(),
        }
    }

    /// Get blame for line (1-indexed)
    pub fn line(&self, line_number: usize) -> Option<&BlameLine> {
        self.lines.get(line_number.saturating_sub(1))
    }

    /// Get all commits involved
    pub fn commits(&self) -> Vec<&str> {
        let mut commits: Vec<&str> = self.lines.iter().map(|l| l.commit_id.as_str()).collect();
        commits.sort();
        commits.dedup();
        commits
    }
}

/// Blame information for a single line
#[derive(Debug, Clone)]
pub struct BlameLine {
    pub commit_id: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub line_number: usize,
    pub original_line: usize,
    pub summary: Option<String>,
}

impl BlameLine {
    /// Get short commit ID
    pub fn short_id(&self) -> &str {
        if self.commit_id.len() >= 7 {
            &self.commit_id[..7]
        } else {
            &self.commit_id
        }
    }

    /// Format for display
    pub fn display(&self) -> String {
        format!("{} {} {}", self.short_id(), self.author, self.relative_time())
    }

    /// Get relative time string
    pub fn relative_time(&self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        
        let diff = now - self.timestamp;
        
        if diff < 60 {
            "just now".to_string()
        } else if diff < 3600 {
            format!("{} minutes ago", diff / 60)
        } else if diff < 86400 {
            format!("{} hours ago", diff / 3600)
        } else if diff < 604800 {
            format!("{} days ago", diff / 86400)
        } else if diff < 2592000 {
            format!("{} weeks ago", diff / 604800)
        } else if diff < 31536000 {
            format!("{} months ago", diff / 2592000)
        } else {
            format!("{} years ago", diff / 31536000)
        }
    }
}

/// Parse porcelain blame output
pub fn parse_blame(output: &str, path: &str) -> Blame {
    let mut blame = Blame::new(path);
    let mut lines = output.lines().peekable();
    let mut line_number = 1usize;

    while let Some(header) = lines.next() {
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let commit_id = parts[0].to_string();
        let original_line = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(line_number);
        
        // Parse metadata lines
        let mut author = String::new();
        let mut email = String::new();
        let mut timestamp = 0i64;
        let mut summary = None;

        while let Some(line) = lines.peek() {
            if line.starts_with('\t') {
                // Content line
                lines.next();
                break;
            }
            
            let line = lines.next().unwrap();
            if let Some(value) = line.strip_prefix("author ") {
                author = value.to_string();
            } else if let Some(value) = line.strip_prefix("author-mail ") {
                email = value.trim_matches(|c| c == '<' || c == '>').to_string();
            } else if let Some(value) = line.strip_prefix("author-time ") {
                timestamp = value.parse().unwrap_or(0);
            } else if let Some(value) = line.strip_prefix("summary ") {
                summary = Some(value.to_string());
            }
        }

        blame.lines.push(BlameLine {
            commit_id,
            author,
            email,
            timestamp,
            line_number,
            original_line,
            summary,
        });

        line_number += 1;
    }

    blame
}
