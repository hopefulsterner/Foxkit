//! Myers diff algorithm

use crate::{Diff, DiffOp};

/// Myers diff algorithm implementation
pub fn myers_diff(old: &[&str], new: &[&str]) -> Diff {
    let n = old.len();
    let m = new.len();
    
    if n == 0 && m == 0 {
        return Diff::empty();
    }
    
    if n == 0 {
        return Diff::new(new.iter().map(|s| DiffOp::Insert(s.to_string())).collect());
    }
    
    if m == 0 {
        return Diff::new(old.iter().map(|s| DiffOp::Delete(s.to_string())).collect());
    }

    // Simple LCS-based diff for now (Myers is more complex)
    let lcs = longest_common_subsequence(old, new);
    let mut ops = Vec::new();
    
    let mut old_idx = 0;
    let mut new_idx = 0;
    
    for (oi, ni) in lcs {
        // Deletions before this match
        while old_idx < oi {
            ops.push(DiffOp::Delete(old[old_idx].to_string()));
            old_idx += 1;
        }
        
        // Insertions before this match
        while new_idx < ni {
            ops.push(DiffOp::Insert(new[new_idx].to_string()));
            new_idx += 1;
        }
        
        // The equal line
        ops.push(DiffOp::Equal(old[old_idx].to_string()));
        old_idx += 1;
        new_idx += 1;
    }
    
    // Remaining deletions
    while old_idx < n {
        ops.push(DiffOp::Delete(old[old_idx].to_string()));
        old_idx += 1;
    }
    
    // Remaining insertions
    while new_idx < m {
        ops.push(DiffOp::Insert(new[new_idx].to_string()));
        new_idx += 1;
    }
    
    Diff::new(ops)
}

/// Find longest common subsequence (returns indices)
fn longest_common_subsequence(old: &[&str], new: &[&str]) -> Vec<(usize, usize)> {
    let n = old.len();
    let m = new.len();
    
    // DP table
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    
    for i in 1..=n {
        for j in 1..=m {
            if old[i - 1] == new[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    
    // Backtrack
    let mut result = Vec::new();
    let mut i = n;
    let mut j = m;
    
    while i > 0 && j > 0 {
        if old[i - 1] == new[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    
    result.reverse();
    result
}

/// Patience diff algorithm (for better results with code)
pub fn patience_diff(old: &[&str], new: &[&str]) -> Diff {
    // Find unique lines in both
    let old_unique = find_unique_lines(old);
    let new_unique = find_unique_lines(new);
    
    // Find common unique lines (anchors)
    let anchors: Vec<(&str, usize, usize)> = old_unique
        .iter()
        .filter_map(|(line, &old_idx)| {
            new_unique.get(line).map(|&new_idx| (*line, old_idx, new_idx))
        })
        .collect();
    
    if anchors.is_empty() {
        // Fall back to Myers
        return myers_diff(old, new);
    }
    
    // Use anchors to split diff into smaller problems
    // For now, just use Myers
    myers_diff(old, new)
}

fn find_unique_lines<'a>(lines: &'a [&str]) -> std::collections::HashMap<&'a str, usize> {
    let mut counts = std::collections::HashMap::new();
    let mut indices = std::collections::HashMap::new();
    
    for (i, line) in lines.iter().enumerate() {
        *counts.entry(*line).or_insert(0) += 1;
        indices.insert(*line, i);
    }
    
    indices
        .into_iter()
        .filter(|(line, _)| counts.get(line) == Some(&1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_myers_diff() {
        let old = vec!["a", "b", "c", "d"];
        let new = vec!["a", "x", "c", "d"];
        
        let diff = myers_diff(&old, &new);
        assert!(diff.has_changes());
    }

    #[test]
    fn test_lcs() {
        let old = vec!["a", "b", "c", "d"];
        let new = vec!["a", "c", "d"];
        
        let lcs = longest_common_subsequence(&old, &new);
        assert_eq!(lcs.len(), 3); // a, c, d
    }
}
