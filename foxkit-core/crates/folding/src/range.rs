//! Folding range types

use serde::{Deserialize, Serialize};

/// A folding range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldingRange {
    /// Start line (0-indexed)
    pub start_line: u32,
    /// Start character
    pub start_character: Option<u32>,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End character
    pub end_character: Option<u32>,
    /// Folding kind
    pub kind: FoldingKind,
    /// Collapsed text preview
    pub collapsed_text: Option<String>,
}

impl FoldingRange {
    pub fn new(start_line: u32, end_line: u32, kind: FoldingKind) -> Self {
        Self {
            start_line,
            start_character: None,
            end_line,
            end_character: None,
            kind,
            collapsed_text: None,
        }
    }

    pub fn with_characters(
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
        kind: FoldingKind,
    ) -> Self {
        Self {
            start_line,
            start_character: Some(start_char),
            end_line,
            end_character: Some(end_char),
            kind,
            collapsed_text: None,
        }
    }

    pub fn with_preview(mut self, text: &str) -> Self {
        self.collapsed_text = Some(text.to_string());
        self
    }

    /// Number of lines in range
    pub fn line_count(&self) -> u32 {
        self.end_line.saturating_sub(self.start_line) + 1
    }

    /// Contains line?
    pub fn contains_line(&self, line: u32) -> bool {
        line >= self.start_line && line <= self.end_line
    }

    /// Is nested in other range?
    pub fn is_nested_in(&self, other: &FoldingRange) -> bool {
        self.start_line >= other.start_line && self.end_line <= other.end_line
    }

    /// Overlaps with other range?
    pub fn overlaps(&self, other: &FoldingRange) -> bool {
        !(self.end_line < other.start_line || self.start_line > other.end_line)
    }
}

/// Folding kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoldingKind {
    /// Comment block
    Comment,
    /// Import section
    Imports,
    /// Region marker
    Region,
    /// Code block
    Block,
}

impl Default for FoldingKind {
    fn default() -> Self {
        FoldingKind::Region
    }
}

/// Builder for folding ranges
pub struct FoldingRangeBuilder {
    ranges: Vec<FoldingRange>,
}

impl FoldingRangeBuilder {
    pub fn new() -> Self {
        Self { ranges: Vec::new() }
    }

    pub fn add(&mut self, start: u32, end: u32, kind: FoldingKind) -> &mut Self {
        self.ranges.push(FoldingRange::new(start, end, kind));
        self
    }

    pub fn add_comment(&mut self, start: u32, end: u32) -> &mut Self {
        self.add(start, end, FoldingKind::Comment)
    }

    pub fn add_imports(&mut self, start: u32, end: u32) -> &mut Self {
        self.add(start, end, FoldingKind::Imports)
    }

    pub fn add_region(&mut self, start: u32, end: u32) -> &mut Self {
        self.add(start, end, FoldingKind::Region)
    }

    pub fn add_block(&mut self, start: u32, end: u32) -> &mut Self {
        self.add(start, end, FoldingKind::Block)
    }

    pub fn add_with_preview(&mut self, start: u32, end: u32, kind: FoldingKind, preview: &str) -> &mut Self {
        self.ranges.push(FoldingRange::new(start, end, kind).with_preview(preview));
        self
    }

    pub fn build(self) -> Vec<FoldingRange> {
        self.ranges
    }
}

impl Default for FoldingRangeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Find matching bracket pairs for folding
pub fn find_bracket_pairs(content: &str) -> Vec<(u32, u32)> {
    let mut pairs = Vec::new();
    let mut stack: Vec<(u32, usize)> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for (col, ch) in line.char_indices() {
            match ch {
                '{' | '[' | '(' => {
                    stack.push((line_num as u32, col));
                }
                '}' | ']' | ')' => {
                    if let Some((start_line, _)) = stack.pop() {
                        if line_num as u32 > start_line {
                            pairs.push((start_line, line_num as u32));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pairs
}

/// Parse region markers
pub fn find_region_markers(content: &str) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    let mut region_stack: Vec<(u32, String)> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        
        // #region / #endregion
        if trimmed.starts_with("#region") || trimmed.starts_with("// #region") || trimmed.starts_with("/* #region") {
            let name = trimmed
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ");
            region_stack.push((line_num as u32, name));
        } else if trimmed.starts_with("#endregion") || trimmed.starts_with("// #endregion") || trimmed.starts_with("/* #endregion") {
            if let Some((start, name)) = region_stack.pop() {
                let mut range = FoldingRange::new(start, line_num as u32, FoldingKind::Region);
                if !name.is_empty() {
                    range.collapsed_text = Some(name);
                }
                ranges.push(range);
            }
        }
        
        // //#region / //#endregion (VSCode style)
        if trimmed.starts_with("//#region") {
            let name = trimmed[9..].trim().to_string();
            region_stack.push((line_num as u32, name));
        } else if trimmed.starts_with("//#endregion") {
            if let Some((start, name)) = region_stack.pop() {
                let mut range = FoldingRange::new(start, line_num as u32, FoldingKind::Region);
                if !name.is_empty() {
                    range.collapsed_text = Some(name);
                }
                ranges.push(range);
            }
        }
    }

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folding_range() {
        let range = FoldingRange::new(0, 10, FoldingKind::Region);
        assert_eq!(range.line_count(), 11);
        assert!(range.contains_line(5));
        assert!(!range.contains_line(15));
    }

    #[test]
    fn test_bracket_pairs() {
        let content = "fn main() {\n    {\n        println!();\n    }\n}";
        let pairs = find_bracket_pairs(content);
        assert!(!pairs.is_empty());
    }

    #[test]
    fn test_region_markers() {
        let content = "// #region Test\ncode\n// #endregion";
        let ranges = find_region_markers(content);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].collapsed_text, Some("Test".to_string()));
    }
}
