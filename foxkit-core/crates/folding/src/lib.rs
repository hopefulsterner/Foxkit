//! # Foxkit Folding
//!
//! Code folding regions management.

pub mod range;
pub mod provider;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use range::{FoldingRange, FoldingKind};
pub use provider::FoldingProvider;

/// Folding service
pub struct FoldingService {
    /// Providers by language
    providers: HashMap<String, Box<dyn FoldingProvider>>,
    /// Default provider
    default_provider: Option<Box<dyn FoldingProvider>>,
    /// Folded ranges per document
    folded_ranges: HashMap<String, Vec<usize>>,
}

impl FoldingService {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: Some(Box::new(provider::IndentFoldingProvider)),
            folded_ranges: HashMap::new(),
        }
    }

    /// Register provider for language
    pub fn register_provider(&mut self, language: &str, provider: Box<dyn FoldingProvider>) {
        self.providers.insert(language.to_string(), provider);
    }

    /// Set default provider
    pub fn set_default_provider(&mut self, provider: Box<dyn FoldingProvider>) {
        self.default_provider = Some(provider);
    }

    /// Get folding ranges for document
    pub fn get_ranges(&self, content: &str, language: &str) -> Vec<FoldingRange> {
        if let Some(provider) = self.providers.get(language) {
            return provider.provide_folding_ranges(content, language);
        }
        
        if let Some(ref provider) = self.default_provider {
            return provider.provide_folding_ranges(content, language);
        }
        
        Vec::new()
    }

    /// Toggle fold at line
    pub fn toggle_fold(&mut self, doc_uri: &str, line: usize, ranges: &[FoldingRange]) {
        let folded = self.folded_ranges.entry(doc_uri.to_string()).or_default();
        
        // Find range containing this line
        for (idx, range) in ranges.iter().enumerate() {
            if range.start_line == line as u32 {
                if let Some(pos) = folded.iter().position(|&i| i == idx) {
                    folded.remove(pos);
                } else {
                    folded.push(idx);
                }
                return;
            }
        }
    }

    /// Fold at line
    pub fn fold(&mut self, doc_uri: &str, line: usize, ranges: &[FoldingRange]) {
        let folded = self.folded_ranges.entry(doc_uri.to_string()).or_default();
        
        for (idx, range) in ranges.iter().enumerate() {
            if range.start_line == line as u32 && !folded.contains(&idx) {
                folded.push(idx);
                return;
            }
        }
    }

    /// Unfold at line
    pub fn unfold(&mut self, doc_uri: &str, line: usize, ranges: &[FoldingRange]) {
        let folded = self.folded_ranges.entry(doc_uri.to_string()).or_default();
        
        for (idx, range) in ranges.iter().enumerate() {
            if range.start_line == line as u32 {
                if let Some(pos) = folded.iter().position(|&i| i == idx) {
                    folded.remove(pos);
                }
                return;
            }
        }
    }

    /// Fold all
    pub fn fold_all(&mut self, doc_uri: &str, ranges: &[FoldingRange]) {
        let folded = self.folded_ranges.entry(doc_uri.to_string()).or_default();
        folded.clear();
        folded.extend(0..ranges.len());
    }

    /// Unfold all
    pub fn unfold_all(&mut self, doc_uri: &str) {
        self.folded_ranges.remove(doc_uri);
    }

    /// Fold all at level
    pub fn fold_level(&mut self, doc_uri: &str, level: u32, ranges: &[FoldingRange]) {
        let folded = self.folded_ranges.entry(doc_uri.to_string()).or_default();
        folded.clear();
        
        for (idx, range) in ranges.iter().enumerate() {
            if self.get_range_level(range, ranges) >= level {
                folded.push(idx);
            }
        }
    }

    /// Get fold level of range
    fn get_range_level(&self, range: &FoldingRange, all_ranges: &[FoldingRange]) -> u32 {
        let mut level = 1;
        for other in all_ranges {
            if other.start_line < range.start_line && other.end_line > range.end_line {
                level += 1;
            }
        }
        level
    }

    /// Is line hidden by fold?
    pub fn is_line_hidden(&self, doc_uri: &str, line: usize, ranges: &[FoldingRange]) -> bool {
        let folded = match self.folded_ranges.get(doc_uri) {
            Some(f) => f,
            None => return false,
        };

        for &idx in folded {
            if let Some(range) = ranges.get(idx) {
                if line as u32 > range.start_line && line as u32 <= range.end_line {
                    return true;
                }
            }
        }
        false
    }

    /// Get visible lines
    pub fn visible_lines(&self, doc_uri: &str, total_lines: usize, ranges: &[FoldingRange]) -> Vec<usize> {
        (0..total_lines)
            .filter(|&line| !self.is_line_hidden(doc_uri, line, ranges))
            .collect()
    }

    /// Is range folded?
    pub fn is_folded(&self, doc_uri: &str, range_idx: usize) -> bool {
        self.folded_ranges
            .get(doc_uri)
            .map(|f| f.contains(&range_idx))
            .unwrap_or(false)
    }

    /// Get folded range indices
    pub fn folded_indices(&self, doc_uri: &str) -> &[usize] {
        self.folded_ranges.get(doc_uri).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

impl Default for FoldingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Folding decoration for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldingDecoration {
    /// Line number
    pub line: u32,
    /// Is collapsed?
    pub collapsed: bool,
    /// Number of hidden lines
    pub hidden_lines: u32,
    /// Collapsed text preview
    pub preview: Option<String>,
}

impl FoldingDecoration {
    pub fn from_range(range: &FoldingRange, collapsed: bool) -> Self {
        Self {
            line: range.start_line,
            collapsed,
            hidden_lines: range.end_line - range.start_line,
            preview: range.collapsed_text.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folding_service() {
        let mut service = FoldingService::new();
        let content = "fn main() {\n    println!(\"Hello\");\n}\n";
        
        let ranges = service.get_ranges(content, "rust");
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_fold_toggle() {
        let mut service = FoldingService::new();
        let ranges = vec![
            FoldingRange::new(0, 2, FoldingKind::Region),
        ];
        
        service.toggle_fold("test.rs", 0, &ranges);
        assert!(service.is_folded("test.rs", 0));
        
        service.toggle_fold("test.rs", 0, &ranges);
        assert!(!service.is_folded("test.rs", 0));
    }
}
