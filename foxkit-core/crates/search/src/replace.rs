//! Search and replace

use std::fs;
use std::path::PathBuf;
use regex::Regex;

use crate::{SearchQuery, FileMatch, Match};

/// Replacer for search and replace
pub struct Replacer {
    query: SearchQuery,
    replacement: String,
    preserve_case: bool,
}

impl Replacer {
    pub fn new(query: SearchQuery, replacement: &str) -> Self {
        Self {
            query,
            replacement: replacement.to_string(),
            preserve_case: false,
        }
    }

    pub fn with_preserve_case(mut self, preserve: bool) -> Self {
        self.preserve_case = preserve;
        self
    }

    /// Preview replacements in file
    pub fn preview(&self, file_match: &FileMatch) -> Vec<ReplacementPreview> {
        let mut previews = Vec::new();
        
        for m in &file_match.matches {
            let matched = m.matched_text();
            let replacement = self.get_replacement(matched);
            
            let new_line = format!(
                "{}{}{}",
                m.prefix(),
                replacement,
                m.suffix()
            );
            
            previews.push(ReplacementPreview {
                line: m.line,
                old_text: m.text.clone(),
                new_text: new_line,
                matched: matched.to_string(),
                replacement,
            });
        }
        
        previews
    }

    /// Apply replacements to file
    pub fn apply(&self, path: &PathBuf) -> Result<ReplaceResult, ReplaceError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ReplaceError::IoError(e.to_string()))?;

        let regex = self.query.to_regex()
            .map_err(|e| ReplaceError::RegexError(e.to_string()))?;

        let (new_content, count) = self.replace_all(&regex, &content);

        if count > 0 {
            fs::write(path, &new_content)
                .map_err(|e| ReplaceError::IoError(e.to_string()))?;
        }

        Ok(ReplaceResult {
            path: path.clone(),
            replacements: count,
        })
    }

    fn replace_all(&self, regex: &Regex, content: &str) -> (String, usize) {
        let mut result = String::with_capacity(content.len());
        let mut last_end = 0;
        let mut count = 0;

        for mat in regex.find_iter(content) {
            result.push_str(&content[last_end..mat.start()]);
            
            let matched = mat.as_str();
            let replacement = self.get_replacement(matched);
            result.push_str(&replacement);
            
            last_end = mat.end();
            count += 1;
        }

        result.push_str(&content[last_end..]);
        (result, count)
    }

    fn get_replacement(&self, matched: &str) -> String {
        if self.preserve_case {
            preserve_case(&self.replacement, matched)
        } else {
            // Handle capture group references like $1, $2
            self.replacement.clone()
        }
    }
}

/// Preview of a replacement
#[derive(Debug, Clone)]
pub struct ReplacementPreview {
    pub line: usize,
    pub old_text: String,
    pub new_text: String,
    pub matched: String,
    pub replacement: String,
}

/// Result of replacement
#[derive(Debug, Clone)]
pub struct ReplaceResult {
    pub path: PathBuf,
    pub replacements: usize,
}

/// Replacement error
#[derive(Debug, Clone)]
pub enum ReplaceError {
    IoError(String),
    RegexError(String),
}

/// Preserve case when replacing
fn preserve_case(replacement: &str, matched: &str) -> String {
    if matched.is_empty() || replacement.is_empty() {
        return replacement.to_string();
    }

    // Check case patterns
    let is_all_upper = matched.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
    let is_all_lower = matched.chars().all(|c| !c.is_alphabetic() || c.is_lowercase());
    let is_title_case = matched.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && matched.chars().skip(1).all(|c| !c.is_alphabetic() || c.is_lowercase());

    if is_all_upper {
        replacement.to_uppercase()
    } else if is_all_lower {
        replacement.to_lowercase()
    } else if is_title_case {
        let mut result = String::new();
        let mut chars = replacement.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_uppercase().next().unwrap_or(first));
        }
        for c in chars {
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
        result
    } else {
        replacement.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preserve_case() {
        assert_eq!(preserve_case("world", "HELLO"), "WORLD");
        assert_eq!(preserve_case("world", "hello"), "world");
        assert_eq!(preserve_case("world", "Hello"), "World");
    }
}
