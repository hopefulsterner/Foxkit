//! Fuzzy matching

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher as FuzzyMatcherTrait;

/// Fuzzy matcher
pub struct FuzzyMatcher {
    matcher: SkimMatcherV2,
}

impl FuzzyMatcher {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Score a string against a pattern
    pub fn score(&self, text: &str, pattern: &str) -> Option<i64> {
        self.matcher.fuzzy_match(text, pattern)
    }

    /// Score with match indices
    pub fn score_with_indices(&self, text: &str, pattern: &str) -> Option<(i64, Vec<usize>)> {
        self.matcher.fuzzy_indices(text, pattern)
    }

    /// Match multiple patterns (AND)
    pub fn match_all(&self, text: &str, patterns: &[&str]) -> Option<i64> {
        let mut total_score = 0i64;
        
        for pattern in patterns {
            match self.score(text, pattern) {
                Some(score) => total_score += score,
                None => return None,
            }
        }
        
        Some(total_score)
    }

    /// Match any pattern (OR)
    pub fn match_any(&self, text: &str, patterns: &[&str]) -> Option<i64> {
        patterns.iter()
            .filter_map(|p| self.score(text, p))
            .max()
    }
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Highlight matched characters in text
pub fn highlight_matches(text: &str, indices: &[usize]) -> Vec<TextSegment> {
    let chars: Vec<char> = text.chars().collect();
    let mut segments = Vec::new();
    let mut current_segment = String::new();
    let mut current_is_match = false;

    for (i, c) in chars.iter().enumerate() {
        let is_match = indices.contains(&i);
        
        if is_match != current_is_match && !current_segment.is_empty() {
            segments.push(TextSegment {
                text: current_segment,
                is_match: current_is_match,
            });
            current_segment = String::new();
        }
        
        current_segment.push(*c);
        current_is_match = is_match;
    }

    if !current_segment.is_empty() {
        segments.push(TextSegment {
            text: current_segment,
            is_match: current_is_match,
        });
    }

    segments
}

/// Text segment with match info
#[derive(Debug, Clone)]
pub struct TextSegment {
    pub text: String,
    pub is_match: bool,
}

/// Path matcher optimized for file paths
pub struct PathMatcher {
    matcher: SkimMatcherV2,
}

impl PathMatcher {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default()
                .use_cache(true)
                .ignore_case(),
        }
    }

    /// Match path with special handling for path separators
    pub fn match_path(&self, path: &str, pattern: &str) -> Option<(i64, Vec<usize>)> {
        // Try matching filename first for higher score
        let filename = path.rsplit('/').next().unwrap_or(path);
        
        if let Some((score, indices)) = self.matcher.fuzzy_indices(filename, pattern) {
            // Adjust indices for full path
            let offset = path.len() - filename.len();
            let adjusted_indices: Vec<_> = indices.into_iter()
                .map(|i| i + offset)
                .collect();
            return Some((score + 100, adjusted_indices)); // Bonus for filename match
        }

        // Fall back to full path match
        self.matcher.fuzzy_indices(path, pattern)
    }

    /// Score path
    pub fn score_path(&self, path: &str, pattern: &str) -> Option<i64> {
        self.match_path(path, pattern).map(|(score, _)| score)
    }
}

impl Default for PathMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Symbol matcher
pub struct SymbolMatcher {
    matcher: SkimMatcherV2,
}

impl SymbolMatcher {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Match symbol with special handling for prefixes
    pub fn match_symbol(&self, symbol: &str, pattern: &str) -> Option<(i64, Vec<usize>)> {
        // Check for prefix filters
        let (kind_filter, search_pattern) = if let Some(rest) = pattern.strip_prefix('@') {
            (Some("symbol"), rest)
        } else if let Some(rest) = pattern.strip_prefix('#') {
            (Some("tag"), rest)
        } else if let Some(rest) = pattern.strip_prefix(':') {
            (Some("line"), rest)
        } else {
            (None, pattern)
        };

        self.matcher.fuzzy_indices(symbol, search_pattern)
    }

    /// Score symbol
    pub fn score_symbol(&self, symbol: &str, pattern: &str) -> Option<i64> {
        self.match_symbol(symbol, pattern).map(|(score, _)| score)
    }
}

impl Default for SymbolMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        let matcher = FuzzyMatcher::new();
        
        assert!(matcher.score("hello world", "hlo").is_some());
        assert!(matcher.score("hello world", "xyz").is_none());
    }

    #[test]
    fn test_highlight_matches() {
        let segments = highlight_matches("hello", &[0, 2]);
        
        assert_eq!(segments.len(), 4);
        assert!(segments[0].is_match);
        assert_eq!(segments[0].text, "h");
        assert!(!segments[1].is_match);
        assert_eq!(segments[1].text, "e");
        assert!(segments[2].is_match);
        assert_eq!(segments[2].text, "l");
    }

    #[test]
    fn test_path_matcher() {
        let matcher = PathMatcher::new();
        
        let result = matcher.score_path("src/components/Button.tsx", "btn");
        assert!(result.is_some());
        
        // Filename match should score higher
        let full_score = matcher.score_path("src/button.rs", "btn").unwrap();
        let name_score = matcher.score_path("src/button_handler.rs", "btn").unwrap();
        // Both should match
        assert!(full_score > 0);
        assert!(name_score > 0);
    }
}
