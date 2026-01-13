//! Patch generation and application

use crate::{Diff, DiffOp};
use serde::{Deserialize, Serialize};

/// A patch that can be applied to transform one text into another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// Original file name
    pub old_name: String,
    /// New file name
    pub new_name: String,
    /// Hunks
    pub hunks: Vec<PatchHunk>,
}

impl Patch {
    pub fn from_diff(diff: &Diff, old_name: &str, new_name: &str, context: usize) -> Self {
        let hunks = extract_hunks(diff, context);
        Self {
            old_name: old_name.to_string(),
            new_name: new_name.to_string(),
            hunks,
        }
    }

    /// Generate unified diff format
    pub fn to_unified(&self) -> String {
        let mut result = String::new();
        result.push_str(&format!("--- {}\n", self.old_name));
        result.push_str(&format!("+++ {}\n", self.new_name));
        
        for hunk in &self.hunks {
            result.push_str(&hunk.to_unified());
        }
        
        result
    }

    /// Apply patch to text
    pub fn apply(&self, text: &str) -> Result<String, PatchError> {
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        let mut offset: i32 = 0;

        for hunk in &self.hunks {
            let adjusted_start = (hunk.old_start as i32 + offset - 1).max(0) as usize;
            
            // Verify context matches
            for (i, line) in hunk.lines.iter().enumerate() {
                match line {
                    PatchLine::Context(expected) | PatchLine::Remove(expected) => {
                        let line_idx = adjusted_start + i;
                        if line_idx >= lines.len() || &lines[line_idx] != expected {
                            return Err(PatchError::ContextMismatch {
                                line: line_idx + 1,
                                expected: expected.clone(),
                                found: lines.get(line_idx).cloned().unwrap_or_default(),
                            });
                        }
                    }
                    _ => {}
                }
            }

            // Apply changes
            let mut new_lines = Vec::new();
            let mut old_idx = 0;
            
            for line in &hunk.lines {
                match line {
                    PatchLine::Context(s) => {
                        new_lines.push(s.clone());
                        old_idx += 1;
                    }
                    PatchLine::Remove(_) => {
                        old_idx += 1;
                    }
                    PatchLine::Add(s) => {
                        new_lines.push(s.clone());
                    }
                }
            }

            // Replace lines
            let end_idx = (adjusted_start + hunk.old_count as usize).min(lines.len());
            lines.splice(adjusted_start..end_idx, new_lines.clone());
            
            // Update offset
            offset += hunk.new_count as i32 - hunk.old_count as i32;
        }

        Ok(lines.join("\n"))
    }

    /// Reverse patch
    pub fn reverse(&self) -> Patch {
        Patch {
            old_name: self.new_name.clone(),
            new_name: self.old_name.clone(),
            hunks: self.hunks.iter().map(|h| h.reverse()).collect(),
        }
    }
}

/// A hunk of changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchHunk {
    /// Start line in old file
    pub old_start: u32,
    /// Number of lines in old file
    pub old_count: u32,
    /// Start line in new file
    pub new_start: u32,
    /// Number of lines in new file
    pub new_count: u32,
    /// Lines in hunk
    pub lines: Vec<PatchLine>,
}

impl PatchHunk {
    pub fn to_unified(&self) -> String {
        let mut result = format!(
            "@@ -{},{} +{},{} @@\n",
            self.old_start, self.old_count, self.new_start, self.new_count
        );

        for line in &self.lines {
            result.push_str(&line.to_string());
            result.push('\n');
        }

        result
    }

    pub fn reverse(&self) -> PatchHunk {
        PatchHunk {
            old_start: self.new_start,
            old_count: self.new_count,
            new_start: self.old_start,
            new_count: self.old_count,
            lines: self.lines.iter().map(|l| l.reverse()).collect(),
        }
    }
}

/// A line in a patch hunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatchLine {
    /// Context line (unchanged)
    Context(String),
    /// Removed line
    Remove(String),
    /// Added line
    Add(String),
}

impl PatchLine {
    pub fn reverse(&self) -> PatchLine {
        match self {
            PatchLine::Context(s) => PatchLine::Context(s.clone()),
            PatchLine::Remove(s) => PatchLine::Add(s.clone()),
            PatchLine::Add(s) => PatchLine::Remove(s.clone()),
        }
    }
}

impl std::fmt::Display for PatchLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchLine::Context(s) => write!(f, " {}", s),
            PatchLine::Remove(s) => write!(f, "-{}", s),
            PatchLine::Add(s) => write!(f, "+{}", s),
        }
    }
}

/// Patch error
#[derive(Debug, Clone, thiserror::Error)]
pub enum PatchError {
    #[error("Context mismatch at line {line}: expected '{expected}', found '{found}'")]
    ContextMismatch {
        line: usize,
        expected: String,
        found: String,
    },
    #[error("Patch cannot be applied: {0}")]
    CannotApply(String),
}

fn extract_hunks(diff: &Diff, context: usize) -> Vec<PatchHunk> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<PatchHunk> = None;
    let mut old_line = 1u32;
    let mut new_line = 1u32;
    let mut context_buffer: Vec<PatchLine> = Vec::new();
    let mut in_change = false;

    for op in &diff.ops {
        match op {
            DiffOp::Equal(line) => {
                if in_change {
                    context_buffer.push(PatchLine::Context(line.clone()));
                    if context_buffer.len() > context * 2 {
                        // End current hunk
                        if let Some(ref mut hunk) = current_hunk {
                            for ctx in context_buffer.drain(..context) {
                                hunk.lines.push(ctx);
                            }
                            update_hunk_counts(hunk);
                            hunks.push(current_hunk.take().unwrap());
                        }
                        context_buffer.clear();
                        in_change = false;
                    }
                } else {
                    context_buffer.push(PatchLine::Context(line.clone()));
                    if context_buffer.len() > context {
                        context_buffer.remove(0);
                    }
                }
                old_line += 1;
                new_line += 1;
            }
            DiffOp::Delete(line) | DiffOp::Insert(line) => {
                if !in_change {
                    let mut hunk = PatchHunk {
                        old_start: old_line.saturating_sub(context_buffer.len() as u32),
                        old_count: 0,
                        new_start: new_line.saturating_sub(context_buffer.len() as u32),
                        new_count: 0,
                        lines: Vec::new(),
                    };
                    hunk.lines.extend(context_buffer.drain(..));
                    current_hunk = Some(hunk);
                    in_change = true;
                }

                if let Some(ref mut hunk) = current_hunk {
                    hunk.lines.extend(context_buffer.drain(..));
                    match op {
                        DiffOp::Delete(line) => {
                            hunk.lines.push(PatchLine::Remove(line.clone()));
                            old_line += 1;
                        }
                        DiffOp::Insert(line) => {
                            hunk.lines.push(PatchLine::Add(line.clone()));
                            new_line += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Finish last hunk
    if let Some(mut hunk) = current_hunk {
        for ctx in context_buffer.into_iter().take(context) {
            hunk.lines.push(ctx);
        }
        update_hunk_counts(&mut hunk);
        hunks.push(hunk);
    }

    hunks
}

fn update_hunk_counts(hunk: &mut PatchHunk) {
    hunk.old_count = hunk.lines.iter().filter(|l| {
        matches!(l, PatchLine::Context(_) | PatchLine::Remove(_))
    }).count() as u32;
    
    hunk.new_count = hunk.lines.iter().filter(|l| {
        matches!(l, PatchLine::Context(_) | PatchLine::Add(_))
    }).count() as u32;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff;

    #[test]
    fn test_patch_apply() {
        let old = "line1\nline2\nline3";
        let new = "line1\nmodified\nline3";
        
        let d = diff(old, new);
        let patch = Patch::from_diff(&d, "old.txt", "new.txt", 3);
        
        let result = patch.apply(old).unwrap();
        assert_eq!(result, new);
    }

    #[test]
    fn test_patch_reverse() {
        let old = "line1\nline2\nline3";
        let new = "line1\nmodified\nline3";
        
        let d = diff(old, new);
        let patch = Patch::from_diff(&d, "old.txt", "new.txt", 3);
        let reversed = patch.reverse();
        
        let result = reversed.apply(new).unwrap();
        assert_eq!(result, old);
    }
}
