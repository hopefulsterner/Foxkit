//! Folding providers

use crate::range::{FoldingRange, FoldingKind, find_bracket_pairs, find_region_markers};

/// Folding provider trait
pub trait FoldingProvider: Send + Sync {
    /// Provide folding ranges
    fn provide_folding_ranges(&self, content: &str, language: &str) -> Vec<FoldingRange>;
}

/// Indent-based folding provider
pub struct IndentFoldingProvider;

impl FoldingProvider for IndentFoldingProvider {
    fn provide_folding_ranges(&self, content: &str, _language: &str) -> Vec<FoldingRange> {
        let mut ranges = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        if lines.is_empty() {
            return ranges;
        }

        let mut stack: Vec<(u32, usize)> = Vec::new(); // (line, indent)

        for (line_num, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            let indent = line.len() - line.trim_start().len();

            // Pop completed ranges
            while let Some(&(start_line, start_indent)) = stack.last() {
                if indent <= start_indent {
                    stack.pop();
                    if line_num as u32 > start_line + 1 {
                        ranges.push(FoldingRange::new(
                            start_line,
                            (line_num as u32).saturating_sub(1),
                            FoldingKind::Block,
                        ));
                    }
                } else {
                    break;
                }
            }

            // Push new potential range start
            stack.push((line_num as u32, indent));
        }

        // Close remaining ranges
        let last_line = (lines.len() as u32).saturating_sub(1);
        for (start_line, _) in stack {
            if last_line > start_line {
                ranges.push(FoldingRange::new(start_line, last_line, FoldingKind::Block));
            }
        }

        ranges
    }
}

/// Bracket-based folding provider
pub struct BracketFoldingProvider;

impl FoldingProvider for BracketFoldingProvider {
    fn provide_folding_ranges(&self, content: &str, _language: &str) -> Vec<FoldingRange> {
        let pairs = find_bracket_pairs(content);
        pairs
            .into_iter()
            .map(|(start, end)| FoldingRange::new(start, end, FoldingKind::Block))
            .collect()
    }
}

/// Syntax-aware folding provider
pub struct SyntaxFoldingProvider {
    /// Minimum lines to create fold
    pub min_lines: u32,
}

impl Default for SyntaxFoldingProvider {
    fn default() -> Self {
        Self { min_lines: 2 }
    }
}

impl FoldingProvider for SyntaxFoldingProvider {
    fn provide_folding_ranges(&self, content: &str, language: &str) -> Vec<FoldingRange> {
        let mut ranges = Vec::new();

        // Add region markers
        ranges.extend(find_region_markers(content));

        // Add bracket-based folds
        let bracket_ranges = find_bracket_pairs(content);
        for (start, end) in bracket_ranges {
            if end - start >= self.min_lines {
                ranges.push(FoldingRange::new(start, end, FoldingKind::Block));
            }
        }

        // Add comment blocks
        ranges.extend(find_comment_blocks(content, language));

        // Add import sections
        ranges.extend(find_import_sections(content, language));

        // Sort by start line
        ranges.sort_by_key(|r| r.start_line);
        
        // Remove duplicates
        ranges.dedup_by(|a, b| a.start_line == b.start_line && a.end_line == b.end_line);

        ranges
    }
}

/// Find comment blocks
fn find_comment_blocks(content: &str, language: &str) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // Block comments
    let (block_start, block_end) = match language {
        "rust" | "c" | "cpp" | "java" | "javascript" | "typescript" | "go" => ("/*", "*/"),
        "python" => ("\"\"\"", "\"\"\""),
        "html" | "xml" => ("<!--", "-->"),
        _ => ("/*", "*/"),
    };

    let mut in_block = false;
    let mut block_start_line = 0;

    for (line_num, line) in lines.iter().enumerate() {
        if !in_block && line.contains(block_start) {
            in_block = true;
            block_start_line = line_num as u32;
        }
        if in_block && line.contains(block_end) {
            in_block = false;
            if line_num as u32 > block_start_line {
                ranges.push(FoldingRange::new(
                    block_start_line,
                    line_num as u32,
                    FoldingKind::Comment,
                ));
            }
        }
    }

    // Consecutive line comments
    let line_comment = match language {
        "rust" | "c" | "cpp" | "java" | "javascript" | "typescript" | "go" => "//",
        "python" | "bash" | "sh" | "yaml" => "#",
        _ => "//",
    };

    let mut comment_start: Option<u32> = None;

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(line_comment) {
            if comment_start.is_none() {
                comment_start = Some(line_num as u32);
            }
        } else {
            if let Some(start) = comment_start {
                if line_num as u32 > start + 1 {
                    ranges.push(FoldingRange::new(
                        start,
                        (line_num as u32).saturating_sub(1),
                        FoldingKind::Comment,
                    ));
                }
            }
            comment_start = None;
        }
    }

    // Handle trailing comment block
    if let Some(start) = comment_start {
        if lines.len() as u32 > start + 1 {
            ranges.push(FoldingRange::new(
                start,
                (lines.len() as u32).saturating_sub(1),
                FoldingKind::Comment,
            ));
        }
    }

    ranges
}

/// Find import sections
fn find_import_sections(content: &str, language: &str) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let import_keywords: &[&str] = match language {
        "rust" => &["use "],
        "python" => &["import ", "from "],
        "javascript" | "typescript" => &["import ", "export "],
        "java" => &["import "],
        "go" => &["import"],
        "c" | "cpp" => &["#include"],
        _ => &["import "],
    };

    let mut import_start: Option<u32> = None;

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let is_import = import_keywords.iter().any(|kw| trimmed.starts_with(kw));

        if is_import {
            if import_start.is_none() {
                import_start = Some(line_num as u32);
            }
        } else if !trimmed.is_empty() {
            if let Some(start) = import_start {
                if line_num as u32 > start + 1 {
                    ranges.push(FoldingRange::new(
                        start,
                        (line_num as u32).saturating_sub(1),
                        FoldingKind::Imports,
                    ));
                }
            }
            import_start = None;
        }
    }

    // Handle trailing import block
    if let Some(start) = import_start {
        if lines.len() as u32 > start + 1 {
            ranges.push(FoldingRange::new(
                start,
                (lines.len() as u32).saturating_sub(1),
                FoldingKind::Imports,
            ));
        }
    }

    ranges
}

/// Combined folding provider
pub struct CombinedFoldingProvider {
    providers: Vec<Box<dyn FoldingProvider>>,
}

impl CombinedFoldingProvider {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn add_provider(&mut self, provider: Box<dyn FoldingProvider>) {
        self.providers.push(provider);
    }
}

impl Default for CombinedFoldingProvider {
    fn default() -> Self {
        let mut combined = Self::new();
        combined.add_provider(Box::new(SyntaxFoldingProvider::default()));
        combined
    }
}

impl FoldingProvider for CombinedFoldingProvider {
    fn provide_folding_ranges(&self, content: &str, language: &str) -> Vec<FoldingRange> {
        let mut all_ranges = Vec::new();
        
        for provider in &self.providers {
            all_ranges.extend(provider.provide_folding_ranges(content, language));
        }

        // Sort and deduplicate
        all_ranges.sort_by_key(|r| (r.start_line, r.end_line));
        all_ranges.dedup_by(|a, b| a.start_line == b.start_line && a.end_line == b.end_line);

        all_ranges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent_folding() {
        let content = "fn main() {\n    let x = 1;\n    let y = 2;\n}";
        let provider = IndentFoldingProvider;
        let ranges = provider.provide_folding_ranges(content, "rust");
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_bracket_folding() {
        let content = "fn main() {\n    {\n        inner();\n    }\n}";
        let provider = BracketFoldingProvider;
        let ranges = provider.provide_folding_ranges(content, "rust");
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_syntax_folding() {
        let content = "// Comment 1\n// Comment 2\n// Comment 3\nfn main() {\n}";
        let provider = SyntaxFoldingProvider::default();
        let ranges = provider.provide_folding_ranges(content, "rust");
        assert!(ranges.iter().any(|r| r.kind == FoldingKind::Comment));
    }
}
