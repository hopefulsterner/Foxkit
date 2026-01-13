//! File name search

use std::path::PathBuf;
use crossbeam_channel::Sender;
use ignore::WalkBuilder;
use crate::{SearchResult, SearchHandle, FileMatch, Match};

/// File searcher (search by file name)
pub struct FileSearcher {
    pattern: String,
    case_insensitive: bool,
}

impl FileSearcher {
    pub fn new(pattern: &str) -> Self {
        let case_insensitive = pattern.chars().all(|c| c.is_lowercase() || !c.is_alphabetic());
        Self {
            pattern: pattern.to_string(),
            case_insensitive,
        }
    }

    pub fn search(&self, root: PathBuf, tx: Sender<SearchResult>, handle: SearchHandle) {
        let walker = WalkBuilder::new(&root)
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .git_global(true)
            .build();

        for entry in walker {
            if handle.is_cancelled() {
                break;
            }

            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            if self.matches(file_name) {
                let file_match = FileMatch {
                    path: path.to_path_buf(),
                    matches: vec![Match::new(1, 1, file_name.len(), file_name.to_string())],
                };
                let _ = tx.send(SearchResult::Match(file_match));
            }
        }

        handle.complete();
    }

    fn matches(&self, file_name: &str) -> bool {
        if self.case_insensitive {
            fuzzy_match(&file_name.to_lowercase(), &self.pattern.to_lowercase())
        } else {
            fuzzy_match(file_name, &self.pattern)
        }
    }
}

/// Fuzzy matching algorithm
fn fuzzy_match(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }

    let mut pattern_chars = pattern.chars().peekable();
    
    for c in text.chars() {
        if let Some(&pc) = pattern_chars.peek() {
            if c == pc {
                pattern_chars.next();
            }
        } else {
            break;
        }
    }

    pattern_chars.peek().is_none()
}

/// Score fuzzy match for ranking
pub fn fuzzy_score(text: &str, pattern: &str) -> Option<i32> {
    if pattern.is_empty() {
        return Some(0);
    }

    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    
    let text_chars: Vec<char> = text_lower.chars().collect();
    let pattern_chars: Vec<char> = pattern_lower.chars().collect();

    let mut score = 0i32;
    let mut pattern_idx = 0;
    let mut prev_match = false;
    let mut prev_was_separator = true;

    for (i, &c) in text_chars.iter().enumerate() {
        let is_separator = c == '/' || c == '\\' || c == '_' || c == '-' || c == '.';
        
        if pattern_idx < pattern_chars.len() && c == pattern_chars[pattern_idx] {
            // Match found
            score += 1;
            
            // Bonus for consecutive matches
            if prev_match {
                score += 5;
            }
            
            // Bonus for match at word boundary
            if prev_was_separator {
                score += 10;
            }
            
            // Bonus for match at start
            if i == 0 {
                score += 15;
            }
            
            // Bonus for exact case match
            if text.chars().nth(i) == pattern.chars().nth(pattern_idx) {
                score += 1;
            }
            
            pattern_idx += 1;
            prev_match = true;
        } else {
            prev_match = false;
        }
        
        prev_was_separator = is_separator;
    }

    if pattern_idx == pattern_chars.len() {
        // Penalty for longer paths
        score -= (text.len() - pattern.len()) as i32 / 5;
        Some(score)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("file.txt", "ftxt"));
        assert!(fuzzy_match("FileService.ts", "fs"));
        assert!(!fuzzy_match("test", "xyz"));
    }

    #[test]
    fn test_fuzzy_score() {
        // Exact match should score higher
        let exact = fuzzy_score("file.txt", "file.txt").unwrap();
        let partial = fuzzy_score("my_file.txt", "file").unwrap();
        assert!(exact > partial);
    }
}
