//! # Foxkit Diff
//!
//! Text diffing and merging algorithms.

pub mod myers;
pub mod merge;
pub mod patch;

use serde::{Deserialize, Serialize};

pub use myers::myers_diff;
pub use merge::{merge3, ConflictStyle};
pub use patch::{Patch, PatchHunk};

/// A single edit operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffOp {
    /// Lines are equal
    Equal(String),
    /// Line was inserted
    Insert(String),
    /// Line was deleted
    Delete(String),
}

impl DiffOp {
    pub fn is_equal(&self) -> bool {
        matches!(self, DiffOp::Equal(_))
    }

    pub fn is_insert(&self) -> bool {
        matches!(self, DiffOp::Insert(_))
    }

    pub fn is_delete(&self) -> bool {
        matches!(self, DiffOp::Delete(_))
    }

    pub fn text(&self) -> &str {
        match self {
            DiffOp::Equal(s) | DiffOp::Insert(s) | DiffOp::Delete(s) => s,
        }
    }
}

/// Diff result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    /// Operations
    pub ops: Vec<DiffOp>,
}

impl Diff {
    pub fn new(ops: Vec<DiffOp>) -> Self {
        Self { ops }
    }

    /// No changes
    pub fn empty() -> Self {
        Self { ops: Vec::new() }
    }

    /// Has changes?
    pub fn has_changes(&self) -> bool {
        self.ops.iter().any(|op| !op.is_equal())
    }

    /// Count insertions
    pub fn insertions(&self) -> usize {
        self.ops.iter().filter(|op| op.is_insert()).count()
    }

    /// Count deletions
    pub fn deletions(&self) -> usize {
        self.ops.iter().filter(|op| op.is_delete()).count()
    }

    /// Get unified diff format
    pub fn unified(&self, old_name: &str, new_name: &str, context: usize) -> String {
        let mut result = String::new();
        result.push_str(&format!("--- {}\n", old_name));
        result.push_str(&format!("+++ {}\n", new_name));

        let hunks = self.to_hunks(context);
        for hunk in hunks {
            result.push_str(&hunk.to_unified());
        }

        result
    }

    /// Convert to hunks with context
    fn to_hunks(&self, context: usize) -> Vec<DiffHunk> {
        let mut hunks = Vec::new();
        let mut current_hunk: Option<DiffHunk> = None;
        let mut old_line = 1;
        let mut new_line = 1;
        let mut context_buffer: Vec<DiffOp> = Vec::new();

        for op in &self.ops {
            match op {
                DiffOp::Equal(line) => {
                    if let Some(ref mut hunk) = current_hunk {
                        context_buffer.push(op.clone());
                        if context_buffer.len() > context * 2 {
                            // End current hunk
                            for ctx in context_buffer.drain(..context) {
                                hunk.ops.push(ctx);
                            }
                            hunk.old_count = old_line - hunk.old_start;
                            hunk.new_count = new_line - hunk.new_start;
                            hunks.push(current_hunk.take().unwrap());
                            context_buffer.clear();
                        }
                    } else {
                        context_buffer.push(op.clone());
                        if context_buffer.len() > context {
                            context_buffer.remove(0);
                        }
                    }
                    old_line += 1;
                    new_line += 1;
                }
                DiffOp::Delete(line) | DiffOp::Insert(line) => {
                    if current_hunk.is_none() {
                        let mut hunk = DiffHunk {
                            old_start: old_line.saturating_sub(context_buffer.len() as u32),
                            old_count: 0,
                            new_start: new_line.saturating_sub(context_buffer.len() as u32),
                            new_count: 0,
                            ops: Vec::new(),
                        };
                        hunk.ops.extend(context_buffer.drain(..));
                        current_hunk = Some(hunk);
                    }
                    
                    if let Some(ref mut hunk) = current_hunk {
                        hunk.ops.extend(context_buffer.drain(..));
                        hunk.ops.push(op.clone());
                    }

                    match op {
                        DiffOp::Delete(_) => old_line += 1,
                        DiffOp::Insert(_) => new_line += 1,
                        _ => {}
                    }
                }
            }
        }

        // Finish last hunk
        if let Some(mut hunk) = current_hunk {
            for ctx in context_buffer.into_iter().take(context) {
                hunk.ops.push(ctx);
            }
            hunk.old_count = old_line - hunk.old_start;
            hunk.new_count = new_line - hunk.new_start;
            hunks.push(hunk);
        }

        hunks
    }
}

/// A hunk of changes
#[derive(Debug, Clone)]
struct DiffHunk {
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
    ops: Vec<DiffOp>,
}

impl DiffHunk {
    fn to_unified(&self) -> String {
        let mut result = format!(
            "@@ -{},{} +{},{} @@\n",
            self.old_start, self.old_count, self.new_start, self.new_count
        );

        for op in &self.ops {
            match op {
                DiffOp::Equal(line) => result.push_str(&format!(" {}\n", line)),
                DiffOp::Delete(line) => result.push_str(&format!("-{}\n", line)),
                DiffOp::Insert(line) => result.push_str(&format!("+{}\n", line)),
            }
        }

        result
    }
}

/// Calculate diff between two strings
pub fn diff(old: &str, new: &str) -> Diff {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    
    myers_diff(&old_lines, &new_lines)
}

/// Calculate diff between two line vectors
pub fn diff_lines(old: &[&str], new: &[&str]) -> Diff {
    myers_diff(old, new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff() {
        let old = "line1\nline2\nline3";
        let new = "line1\nmodified\nline3";
        
        let d = diff(old, new);
        assert!(d.has_changes());
        assert_eq!(d.insertions(), 1);
        assert_eq!(d.deletions(), 1);
    }

    #[test]
    fn test_no_changes() {
        let text = "line1\nline2";
        let d = diff(text, text);
        assert!(!d.has_changes());
    }
}
