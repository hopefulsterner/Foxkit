//! Git blame

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Blame service
pub struct BlameService {
    repo_path: PathBuf,
    cache: parking_lot::RwLock<HashMap<PathBuf, BlameResult>>,
}

impl BlameService {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            cache: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Get blame for file
    pub async fn blame(&self, file: &PathBuf) -> anyhow::Result<BlameResult> {
        // Check cache
        if let Some(cached) = self.cache.read().get(file) {
            return Ok(cached.clone());
        }

        let output = tokio::process::Command::new("git")
            .args([
                "blame",
                "--porcelain",
                file.to_str().unwrap_or(""),
            ])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result = parse_blame_output(&stdout)?;

        // Cache result
        self.cache.write().insert(file.clone(), result.clone());

        Ok(result)
    }

    /// Get blame for line range
    pub async fn blame_range(
        &self,
        file: &PathBuf,
        start_line: u32,
        end_line: u32,
    ) -> anyhow::Result<BlameResult> {
        let output = tokio::process::Command::new("git")
            .args([
                "blame",
                "--porcelain",
                &format!("-L{},{}", start_line, end_line),
                file.to_str().unwrap_or(""),
            ])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_blame_output(&stdout)
    }

    /// Get inline blame annotation for line
    pub async fn get_line_annotation(
        &self,
        file: &PathBuf,
        line: u32,
    ) -> anyhow::Result<Option<BlameAnnotation>> {
        let blame = self.blame(file).await?;
        
        Ok(blame.lines.get(line as usize).map(|line_blame| {
            BlameAnnotation {
                commit_id: line_blame.commit_id.clone(),
                short_id: line_blame.commit_id[..7.min(line_blame.commit_id.len())].to_string(),
                author: line_blame.author.clone(),
                date: format_relative_date(line_blame.timestamp),
                message: line_blame.summary.clone(),
            }
        }))
    }

    /// Invalidate cache for file
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
    }

    /// Clear all cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

fn parse_blame_output(output: &str) -> anyhow::Result<BlameResult> {
    let mut lines = Vec::new();
    let mut commits: HashMap<String, BlameCommit> = HashMap::new();
    
    let mut current_commit_id = String::new();
    let mut current_author = String::new();
    let mut current_author_mail = String::new();
    let mut current_author_time = 0i64;
    let mut current_summary = String::new();
    let mut current_line = 0u32;

    for line in output.lines() {
        if line.starts_with(|c: char| c.is_ascii_hexdigit()) && line.len() >= 40 {
            // New commit line: <sha> <orig_line> <final_line> [<num_lines>]
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                current_commit_id = parts[0].to_string();
                current_line = parts[2].parse().unwrap_or(0);
            }
        } else if let Some(author) = line.strip_prefix("author ") {
            current_author = author.to_string();
        } else if let Some(mail) = line.strip_prefix("author-mail ") {
            current_author_mail = mail.trim_matches(|c| c == '<' || c == '>').to_string();
        } else if let Some(time) = line.strip_prefix("author-time ") {
            current_author_time = time.parse().unwrap_or(0);
        } else if let Some(summary) = line.strip_prefix("summary ") {
            current_summary = summary.to_string();
        } else if line.starts_with('\t') {
            // Content line - commit info is complete
            if !current_commit_id.is_empty() {
                let commit = commits.entry(current_commit_id.clone())
                    .or_insert_with(|| BlameCommit {
                        id: current_commit_id.clone(),
                        author: current_author.clone(),
                        author_email: current_author_mail.clone(),
                        timestamp: current_author_time,
                        summary: current_summary.clone(),
                    });

                lines.push(LineBlame {
                    line: current_line,
                    commit_id: current_commit_id.clone(),
                    author: commit.author.clone(),
                    timestamp: commit.timestamp,
                    summary: commit.summary.clone(),
                });
            }
        }
    }

    Ok(BlameResult {
        lines,
        commits: commits.into_values().collect(),
    })
}

fn format_relative_date(timestamp: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    
    let diff = now - timestamp;
    
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

/// Blame result
#[derive(Debug, Clone)]
pub struct BlameResult {
    /// Per-line blame info
    pub lines: Vec<LineBlame>,
    /// Unique commits
    pub commits: Vec<BlameCommit>,
}

/// Line blame info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineBlame {
    pub line: u32,
    pub commit_id: String,
    pub author: String,
    pub timestamp: i64,
    pub summary: String,
}

/// Blame commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameCommit {
    pub id: String,
    pub author: String,
    pub author_email: String,
    pub timestamp: i64,
    pub summary: String,
}

/// Inline blame annotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameAnnotation {
    pub commit_id: String,
    pub short_id: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

impl BlameAnnotation {
    /// Format for display
    pub fn format(&self, max_author_len: usize) -> String {
        let author = if self.author.len() > max_author_len {
            format!("{}…", &self.author[..max_author_len - 1])
        } else {
            format!("{:width$}", self.author, width = max_author_len)
        };
        
        format!("{} • {} • {}", author, self.date, self.message)
    }

    /// Format short version
    pub fn format_short(&self) -> String {
        format!("{}, {}", self.author, self.date)
    }
}
