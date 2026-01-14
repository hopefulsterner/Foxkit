//! Word boundary detection and text classification
//!
//! Provides utilities for word-wise cursor movement and selection.

/// Character classification for word boundaries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharClass {
    /// Whitespace (space, tab, newline)
    Whitespace,
    /// Word characters (letters, digits, underscore)
    Word,
    /// Punctuation and symbols
    Punctuation,
    /// Line ending
    LineEnding,
}

impl CharClass {
    /// Classify a character
    pub fn of(c: char) -> Self {
        if c == '\n' || c == '\r' {
            CharClass::LineEnding
        } else if c.is_whitespace() {
            CharClass::Whitespace
        } else if c.is_alphanumeric() || c == '_' {
            CharClass::Word
        } else {
            CharClass::Punctuation
        }
    }
}

/// Find the start of the current word
pub fn word_start(text: &str, offset: usize) -> usize {
    if offset == 0 || text.is_empty() {
        return 0;
    }

    let bytes = text.as_bytes();
    let mut pos = offset.min(text.len());
    
    // Move back to valid char boundary
    while pos > 0 && !text.is_char_boundary(pos) {
        pos -= 1;
    }
    
    if pos == 0 {
        return 0;
    }

    // Get the class of character before cursor
    let char_before = text[..pos].chars().last().unwrap();
    let initial_class = CharClass::of(char_before);

    // Skip whitespace
    if initial_class == CharClass::Whitespace {
        while pos > 0 {
            if !text.is_char_boundary(pos - 1) {
                pos -= 1;
                continue;
            }
            let c = text[..pos].chars().last().unwrap();
            if CharClass::of(c) != CharClass::Whitespace {
                break;
            }
            pos -= c.len_utf8();
        }
        if pos == 0 {
            return 0;
        }
    }

    // Get class at new position
    let target_class = text[..pos].chars().last().map(CharClass::of).unwrap_or(CharClass::Whitespace);

    // Move back while same class
    while pos > 0 {
        // Find previous char
        let mut prev_pos = pos - 1;
        while prev_pos > 0 && !text.is_char_boundary(prev_pos) {
            prev_pos -= 1;
        }
        
        let c = text[prev_pos..pos].chars().next().unwrap();
        if CharClass::of(c) != target_class {
            break;
        }
        pos = prev_pos;
    }

    pos
}

/// Find the end of the current word
pub fn word_end(text: &str, offset: usize) -> usize {
    if offset >= text.len() || text.is_empty() {
        return text.len();
    }

    let mut pos = offset;
    
    // Move to valid char boundary
    while pos < text.len() && !text.is_char_boundary(pos) {
        pos += 1;
    }

    if pos >= text.len() {
        return text.len();
    }

    // Get the class of character at cursor
    let char_at = text[pos..].chars().next().unwrap();
    let initial_class = CharClass::of(char_at);

    // If on whitespace, skip it first
    if initial_class == CharClass::Whitespace {
        while pos < text.len() {
            let c = text[pos..].chars().next().unwrap();
            if CharClass::of(c) != CharClass::Whitespace {
                break;
            }
            pos += c.len_utf8();
        }
        if pos >= text.len() {
            return text.len();
        }
    }

    // Get class at new position
    let target_class = text[pos..].chars().next().map(CharClass::of).unwrap_or(CharClass::Whitespace);

    // Move forward while same class
    while pos < text.len() {
        let c = text[pos..].chars().next().unwrap();
        if CharClass::of(c) != target_class {
            break;
        }
        pos += c.len_utf8();
    }

    pos
}

/// Find the next word boundary (for Ctrl+Right)
pub fn next_word_boundary(text: &str, offset: usize) -> usize {
    word_end(text, offset)
}

/// Find the previous word boundary (for Ctrl+Left)
pub fn prev_word_boundary(text: &str, offset: usize) -> usize {
    word_start(text, offset)
}

/// Select word at offset (returns start, end)
pub fn word_at(text: &str, offset: usize) -> (usize, usize) {
    let start = word_start(text, offset);
    let end = word_end(text, offset);
    (start, end)
}

/// Check if character is a word character
pub fn is_word_char(c: char) -> bool {
    CharClass::of(c) == CharClass::Word
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_start() {
        let text = "hello world";
        assert_eq!(word_start(text, 7), 6); // 'w' -> start of "world"
        assert_eq!(word_start(text, 5), 0); // end of "hello"
        assert_eq!(word_start(text, 0), 0);
    }

    #[test]
    fn test_word_end() {
        let text = "hello world";
        assert_eq!(word_end(text, 0), 5);  // start -> end of "hello"
        assert_eq!(word_end(text, 6), 11); // 'w' -> end of "world"
    }

    #[test]
    fn test_word_at() {
        let text = "fn main() {}";
        assert_eq!(word_at(text, 0), (0, 2));   // "fn"
        assert_eq!(word_at(text, 3), (3, 7));   // "main"
    }
}
