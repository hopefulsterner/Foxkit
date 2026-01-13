//! Git history

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::CommitInfo;

/// Commit history
pub struct History {
    repo_path: PathBuf,
}

impl History {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Get commit log
    pub async fn log(
        &self,
        limit: usize,
        skip: usize,
        path: Option<&PathBuf>,
        author: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
    ) -> anyhow::Result<Vec<CommitInfo>> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.args([
            "log",
            "--format=%H|%h|%s|%an|%ae|%at|%P",
            &format!("-{}", limit),
            &format!("--skip={}", skip),
        ]);
        cmd.current_dir(&self.repo_path);

        if let Some(p) = path {
            cmd.arg("--").arg(p);
        }
        if let Some(a) = author {
            cmd.arg(format!("--author={}", a));
        }
        if let Some(s) = since {
            cmd.arg(format!("--since={}", s));
        }
        if let Some(u) = until {
            cmd.arg(format!("--until={}", u));
        }

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        let mut commits = Vec::new();
        for line in stdout.lines() {
            if let Some(commit) = parse_commit_line(line) {
                commits.push(commit);
            }
        }

        Ok(commits)
    }

    /// Get commit details
    pub async fn get_commit(&self, commit_id: &str) -> anyhow::Result<CommitDetails> {
        let output = tokio::process::Command::new("git")
            .args([
                "show",
                "--format=%H|%h|%s|%an|%ae|%at|%P|%B",
                "--stat",
                commit_id,
            ])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<_> = stdout.lines().collect();
        
        if lines.is_empty() {
            anyhow::bail!("Commit not found: {}", commit_id);
        }

        let info = parse_commit_line(lines[0])
            .ok_or_else(|| anyhow::anyhow!("Failed to parse commit"))?;

        // Parse changed files from stat
        let mut changed_files = Vec::new();
        let mut in_stat = false;
        
        for line in &lines[1..] {
            if line.contains("|") && !in_stat {
                in_stat = true;
            }
            
            if in_stat {
                if line.starts_with(' ') && line.contains('|') {
                    // File stat line: " path/to/file | N +++---"
                    let parts: Vec<_> = line.split('|').collect();
                    if !parts.is_empty() {
                        let file = parts[0].trim().to_string();
                        changed_files.push(file);
                    }
                } else if line.trim().is_empty() || line.contains("file") {
                    break;
                }
            }
        }

        Ok(CommitDetails {
            info,
            full_message: lines.get(1..).map(|l| l.join("\n")).unwrap_or_default(),
            changed_files,
        })
    }

    /// Get file history
    pub async fn file_history(
        &self,
        path: &PathBuf,
        limit: usize,
    ) -> anyhow::Result<Vec<CommitInfo>> {
        self.log(limit, 0, Some(path), None, None, None).await
    }

    /// Compare commits
    pub async fn compare(
        &self,
        from: &str,
        to: &str,
    ) -> anyhow::Result<CompareResult> {
        // Get commits between
        let output = tokio::process::Command::new("git")
            .args([
                "log",
                "--format=%H|%h|%s|%an|%ae|%at|%P",
                &format!("{}..{}", from, to),
            ])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits: Vec<_> = stdout.lines()
            .filter_map(parse_commit_line)
            .collect();

        // Get diff stats
        let output = tokio::process::Command::new("git")
            .args(["diff", "--stat", from, to])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        let diff_stat = String::from_utf8_lossy(&output.stdout).to_string();

        Ok(CompareResult {
            from: from.to_string(),
            to: to.to_string(),
            commits,
            diff_stat,
        })
    }

    /// Search commits
    pub async fn search(
        &self,
        query: &str,
        in_message: bool,
        in_diff: bool,
        limit: usize,
    ) -> anyhow::Result<Vec<CommitInfo>> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.args([
            "log",
            "--format=%H|%h|%s|%an|%ae|%at|%P",
            &format!("-{}", limit),
        ]);
        cmd.current_dir(&self.repo_path);

        if in_message {
            cmd.arg(format!("--grep={}", query));
        }
        if in_diff {
            cmd.arg(format!("-S{}", query));
        }

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        Ok(stdout.lines().filter_map(parse_commit_line).collect())
    }

    /// Get graph data for visualization
    pub async fn graph(&self, limit: usize) -> anyhow::Result<Vec<GraphNode>> {
        let output = tokio::process::Command::new("git")
            .args([
                "log",
                "--format=%H|%P",
                "--all",
                &format!("-{}", limit),
            ])
            .current_dir(&self.repo_path)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut nodes = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<_> = line.split('|').collect();
            if parts.len() >= 2 {
                let id = parts[0].to_string();
                let parents: Vec<_> = parts[1].split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                
                nodes.push(GraphNode {
                    id,
                    parents,
                    column: 0, // Would calculate
                });
            }
        }

        Ok(nodes)
    }
}

fn parse_commit_line(line: &str) -> Option<CommitInfo> {
    let parts: Vec<_> = line.split('|').collect();
    if parts.len() < 6 {
        return None;
    }

    Some(CommitInfo {
        id: parts[0].to_string(),
        short_id: parts[1].to_string(),
        message: parts[2].to_string(),
        author: parts[3].to_string(),
        author_email: parts[4].to_string(),
        timestamp: parts[5].parse().unwrap_or(0),
        parent_ids: parts.get(6)
            .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
            .unwrap_or_default(),
    })
}

/// Commit details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetails {
    pub info: CommitInfo,
    pub full_message: String,
    pub changed_files: Vec<String>,
}

/// Compare result
#[derive(Debug, Clone)]
pub struct CompareResult {
    pub from: String,
    pub to: String,
    pub commits: Vec<CommitInfo>,
    pub diff_stat: String,
}

/// Graph node for visualization
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub parents: Vec<String>,
    pub column: usize,
}
