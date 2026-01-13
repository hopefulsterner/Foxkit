//! Chunk utilities (additional helpers)

use unicode_segmentation::UnicodeSegmentation;

/// Count grapheme clusters in a string
pub fn grapheme_count(s: &str) -> usize {
    s.graphemes(true).count()
}

/// Get grapheme at index
pub fn grapheme_at(s: &str, idx: usize) -> Option<&str> {
    s.graphemes(true).nth(idx)
}

/// Find word boundaries
pub fn word_boundaries(s: &str) -> Vec<usize> {
    let mut boundaries = vec![0];
    let mut offset = 0;
    
    for word in s.unicode_words() {
        if let Some(start) = s[offset..].find(word) {
            let word_start = offset + start;
            let word_end = word_start + word.len();
            
            if boundaries.last() != Some(&word_start) {
                boundaries.push(word_start);
            }
            boundaries.push(word_end);
            offset = word_end;
        }
    }
    
    if boundaries.last() != Some(&s.len()) && !s.is_empty() {
        boundaries.push(s.len());
    }
    
    boundaries
}

/// Find next word boundary from offset
pub fn next_word_boundary(s: &str, offset: usize) -> usize {
    let boundaries = word_boundaries(s);
    for &b in &boundaries {
        if b > offset {
            return b;
        }
    }
    s.len()
}

/// Find previous word boundary from offset
pub fn prev_word_boundary(s: &str, offset: usize) -> usize {
    let boundaries = word_boundaries(s);
    for &b in boundaries.iter().rev() {
        if b < offset {
            return b;
        }
    }
    0
}
