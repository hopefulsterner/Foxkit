//! Three-way merge

use crate::{Diff, DiffOp, diff_lines};
use serde::{Deserialize, Serialize};

/// Conflict style for merge output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictStyle {
    /// Standard diff3 style
    Diff3,
    /// Git merge style
    Merge,
}

impl Default for ConflictStyle {
    fn default() -> Self {
        ConflictStyle::Merge
    }
}

/// Merge result
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Merged content
    pub content: String,
    /// Has conflicts?
    pub has_conflicts: bool,
    /// Conflict regions
    pub conflicts: Vec<Conflict>,
}

/// A conflict region
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Start line in output
    pub start_line: usize,
    /// End line in output
    pub end_line: usize,
    /// Ours content
    pub ours: Vec<String>,
    /// Theirs content
    pub theirs: Vec<String>,
    /// Base content (for diff3)
    pub base: Vec<String>,
}

/// Three-way merge
pub fn merge3(base: &str, ours: &str, theirs: &str) -> MergeResult {
    merge3_with_style(base, ours, theirs, ConflictStyle::Merge)
}

/// Three-way merge with conflict style
pub fn merge3_with_style(
    base: &str,
    ours: &str,
    theirs: &str,
    style: ConflictStyle,
) -> MergeResult {
    let base_lines: Vec<&str> = base.lines().collect();
    let ours_lines: Vec<&str> = ours.lines().collect();
    let theirs_lines: Vec<&str> = theirs.lines().collect();

    let diff_ours = diff_lines(&base_lines, &ours_lines);
    let diff_theirs = diff_lines(&base_lines, &theirs_lines);

    // Simple merge: if changes don't overlap, merge them
    // If they overlap, create conflict
    
    let mut result_lines = Vec::new();
    let mut conflicts = Vec::new();
    let mut has_conflicts = false;

    let mut base_idx = 0;
    let mut ours_idx = 0;
    let mut theirs_idx = 0;

    // Get changes from both diffs
    let ours_changes = extract_changes(&diff_ours);
    let theirs_changes = extract_changes(&diff_theirs);

    // Sort changes by base position
    let mut all_changes: Vec<ChangeRegion> = Vec::new();
    for (start, end, lines) in ours_changes {
        all_changes.push(ChangeRegion {
            base_start: start,
            base_end: end,
            new_lines: lines,
            source: ChangeSource::Ours,
        });
    }
    for (start, end, lines) in theirs_changes {
        all_changes.push(ChangeRegion {
            base_start: start,
            base_end: end,
            new_lines: lines,
            source: ChangeSource::Theirs,
        });
    }
    
    all_changes.sort_by_key(|c| c.base_start);

    // Process changes
    let mut last_end = 0;
    let mut i = 0;
    
    while i < all_changes.len() {
        let change = &all_changes[i];
        
        // Add unchanged lines before this change
        for j in last_end..change.base_start {
            if j < base_lines.len() {
                result_lines.push(base_lines[j].to_string());
            }
        }

        // Check for overlapping changes
        let mut overlapping: Vec<&ChangeRegion> = vec![change];
        let mut max_end = change.base_end;
        
        let mut j = i + 1;
        while j < all_changes.len() && all_changes[j].base_start < max_end {
            overlapping.push(&all_changes[j]);
            max_end = max_end.max(all_changes[j].base_end);
            j += 1;
        }

        if overlapping.len() == 1 {
            // No conflict, apply change
            result_lines.extend(change.new_lines.clone());
            last_end = change.base_end;
            i += 1;
        } else {
            // Conflict!
            has_conflicts = true;
            
            let ours_content: Vec<String> = overlapping
                .iter()
                .filter(|c| c.source == ChangeSource::Ours)
                .flat_map(|c| c.new_lines.clone())
                .collect();
            
            let theirs_content: Vec<String> = overlapping
                .iter()
                .filter(|c| c.source == ChangeSource::Theirs)
                .flat_map(|c| c.new_lines.clone())
                .collect();
            
            let base_content: Vec<String> = (change.base_start..max_end)
                .filter_map(|idx| base_lines.get(idx).map(|s| s.to_string()))
                .collect();

            let conflict_start = result_lines.len();
            
            match style {
                ConflictStyle::Merge => {
                    result_lines.push("<<<<<<< ours".to_string());
                    result_lines.extend(ours_content.clone());
                    result_lines.push("=======".to_string());
                    result_lines.extend(theirs_content.clone());
                    result_lines.push(">>>>>>> theirs".to_string());
                }
                ConflictStyle::Diff3 => {
                    result_lines.push("<<<<<<< ours".to_string());
                    result_lines.extend(ours_content.clone());
                    result_lines.push("||||||| base".to_string());
                    result_lines.extend(base_content.clone());
                    result_lines.push("=======".to_string());
                    result_lines.extend(theirs_content.clone());
                    result_lines.push(">>>>>>> theirs".to_string());
                }
            }

            conflicts.push(Conflict {
                start_line: conflict_start,
                end_line: result_lines.len(),
                ours: ours_content,
                theirs: theirs_content,
                base: base_content,
            });

            last_end = max_end;
            i = j;
        }
    }

    // Add remaining unchanged lines
    for j in last_end..base_lines.len() {
        result_lines.push(base_lines[j].to_string());
    }

    MergeResult {
        content: result_lines.join("\n"),
        has_conflicts,
        conflicts,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChangeSource {
    Ours,
    Theirs,
}

#[derive(Debug, Clone)]
struct ChangeRegion {
    base_start: usize,
    base_end: usize,
    new_lines: Vec<String>,
    source: ChangeSource,
}

fn extract_changes(diff: &Diff) -> Vec<(usize, usize, Vec<String>)> {
    let mut changes = Vec::new();
    let mut base_idx = 0;
    let mut current_change: Option<(usize, Vec<String>)> = None;

    for op in &diff.ops {
        match op {
            DiffOp::Equal(_) => {
                if let Some((start, lines)) = current_change.take() {
                    changes.push((start, base_idx, lines));
                }
                base_idx += 1;
            }
            DiffOp::Delete(_) => {
                if current_change.is_none() {
                    current_change = Some((base_idx, Vec::new()));
                }
                base_idx += 1;
            }
            DiffOp::Insert(line) => {
                if current_change.is_none() {
                    current_change = Some((base_idx, Vec::new()));
                }
                if let Some((_, ref mut lines)) = current_change {
                    lines.push(line.clone());
                }
            }
        }
    }

    if let Some((start, lines)) = current_change {
        changes.push((start, base_idx, lines));
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_conflict_merge() {
        let base = "line1\nline2\nline3";
        let ours = "line1\nmodified by us\nline3";
        let theirs = "line1\nline2\nmodified by them";
        
        let result = merge3(base, ours, theirs);
        assert!(!result.has_conflicts);
    }

    #[test]
    fn test_conflict_merge() {
        let base = "line1\nline2\nline3";
        let ours = "line1\nours\nline3";
        let theirs = "line1\ntheirs\nline3";
        
        let result = merge3(base, ours, theirs);
        assert!(result.has_conflicts);
        assert_eq!(result.conflicts.len(), 1);
    }
}
