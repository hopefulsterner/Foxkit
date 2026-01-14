//! Soft wrap support for the editor
//!
//! Provides word wrapping without modifying the underlying buffer.

use std::ops::Range;

/// Soft wrap configuration
#[derive(Debug, Clone)]
pub struct SoftWrapConfig {
    /// Enable soft wrapping
    pub enabled: bool,
    /// Wrap at column (0 = viewport width)
    pub wrap_column: usize,
    /// Minimum characters before wrapping
    pub min_wrap_width: usize,
    /// Indent wrapped lines
    pub indent_wrapped_lines: bool,
    /// Indent amount (spaces)
    pub wrap_indent: usize,
    /// Wrap on word boundaries when possible
    pub word_wrap: bool,
}

impl Default for SoftWrapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            wrap_column: 0, // Use viewport width
            min_wrap_width: 20,
            indent_wrapped_lines: true,
            wrap_indent: 2,
            word_wrap: true,
        }
    }
}

/// A wrapped line segment
#[derive(Debug, Clone)]
pub struct WrapSegment {
    /// Start offset in the original line (byte offset)
    pub start: usize,
    /// End offset in the original line (byte offset)
    pub end: usize,
    /// Display indent for this segment
    pub indent: usize,
    /// Is this the first segment of the line?
    pub is_first: bool,
    /// Is this the last segment of the line?
    pub is_last: bool,
}

/// Wrapped line information
#[derive(Debug, Clone)]
pub struct WrappedLine {
    /// Original line number (1-indexed)
    pub line_number: usize,
    /// Segments after wrapping
    pub segments: Vec<WrapSegment>,
}

impl WrappedLine {
    /// Create a non-wrapped line (single segment)
    pub fn unwrapped(line_number: usize, len: usize) -> Self {
        Self {
            line_number,
            segments: vec![WrapSegment {
                start: 0,
                end: len,
                indent: 0,
                is_first: true,
                is_last: true,
            }],
        }
    }

    /// Number of display rows this line takes
    pub fn row_count(&self) -> usize {
        self.segments.len()
    }
}

/// Soft wrap engine
pub struct SoftWrapEngine {
    config: SoftWrapConfig,
    /// Cached wrap results: line_number -> WrappedLine
    cache: Vec<Option<WrappedLine>>,
    /// Total display rows
    total_rows: usize,
}

impl SoftWrapEngine {
    /// Create a new soft wrap engine
    pub fn new(config: SoftWrapConfig) -> Self {
        Self {
            config,
            cache: Vec::new(),
            total_rows: 0,
        }
    }

    /// Update configuration
    pub fn set_config(&mut self, config: SoftWrapConfig) {
        self.config = config;
        self.invalidate_all();
    }

    /// Get configuration
    pub fn config(&self) -> &SoftWrapConfig {
        &self.config
    }

    /// Wrap a single line
    pub fn wrap_line(&self, line: &str, max_width: usize) -> WrappedLine {
        let line_number = 0; // Caller should set this
        
        if !self.config.enabled || line.is_empty() {
            return WrappedLine::unwrapped(line_number, line.len());
        }

        let effective_width = if self.config.wrap_column > 0 {
            self.config.wrap_column.min(max_width)
        } else {
            max_width
        };

        if effective_width < self.config.min_wrap_width {
            return WrappedLine::unwrapped(line_number, line.len());
        }

        let mut segments = Vec::new();
        let mut current_start = 0;
        let mut current_width = 0;
        let mut last_break_point = 0;
        let mut last_break_width = 0;
        let is_first_segment = true;

        let chars: Vec<(usize, char)> = line.char_indices().collect();
        
        for (idx, (byte_pos, ch)) in chars.iter().enumerate() {
            let char_width = if *ch == '\t' { 4 } else { 1 };
            
            // Track word boundaries
            if self.config.word_wrap && is_word_boundary(*ch) && *byte_pos > current_start {
                last_break_point = *byte_pos;
                last_break_width = current_width;
            }

            current_width += char_width;

            // Calculate effective width for this segment
            let segment_max_width = if segments.is_empty() {
                effective_width
            } else {
                effective_width.saturating_sub(self.config.wrap_indent)
            };

            if current_width > segment_max_width && current_width > 1 {
                // Need to wrap
                let break_point = if self.config.word_wrap && last_break_point > current_start {
                    // Wrap at word boundary
                    last_break_point
                } else {
                    // Wrap at current position
                    *byte_pos
                };

                segments.push(WrapSegment {
                    start: current_start,
                    end: break_point,
                    indent: if segments.is_empty() { 0 } else { self.config.wrap_indent },
                    is_first: segments.is_empty(),
                    is_last: false,
                });

                current_start = break_point;
                // Skip whitespace at start of new segment
                while current_start < line.len() {
                    let c = line[current_start..].chars().next().unwrap_or(' ');
                    if c.is_whitespace() && c != '\n' {
                        current_start += c.len_utf8();
                    } else {
                        break;
                    }
                }
                current_width = 0;
                last_break_point = current_start;
                last_break_width = 0;
            }
        }

        // Add final segment
        if current_start < line.len() || segments.is_empty() {
            segments.push(WrapSegment {
                start: current_start,
                end: line.len(),
                indent: if segments.is_empty() { 0 } else { self.config.wrap_indent },
                is_first: segments.is_empty(),
                is_last: true,
            });
        }

        // Mark last segment
        if let Some(last) = segments.last_mut() {
            last.is_last = true;
        }

        WrappedLine {
            line_number,
            segments,
        }
    }

    /// Wrap all lines in the buffer
    pub fn wrap_buffer(&mut self, lines: &[&str], max_width: usize) {
        self.cache.clear();
        self.total_rows = 0;

        for (i, line) in lines.iter().enumerate() {
            let mut wrapped = self.wrap_line(line, max_width);
            wrapped.line_number = i + 1;
            self.total_rows += wrapped.row_count();
            self.cache.push(Some(wrapped));
        }
    }

    /// Get wrapped line
    pub fn get_wrapped_line(&self, line_number: usize) -> Option<&WrappedLine> {
        self.cache.get(line_number.saturating_sub(1))?.as_ref()
    }

    /// Invalidate cache for a line
    pub fn invalidate_line(&mut self, line_number: usize) {
        if let Some(entry) = self.cache.get_mut(line_number.saturating_sub(1)) {
            *entry = None;
        }
    }

    /// Invalidate all cache
    pub fn invalidate_all(&mut self) {
        self.cache.clear();
        self.total_rows = 0;
    }

    /// Get total display rows
    pub fn total_display_rows(&self) -> usize {
        self.total_rows
    }

    /// Convert display row to buffer line and segment
    pub fn display_row_to_line(&self, display_row: usize) -> Option<(usize, usize)> {
        let mut current_row = 0;
        
        for (line_idx, wrapped) in self.cache.iter().enumerate() {
            if let Some(wrapped) = wrapped {
                let row_count = wrapped.row_count();
                if display_row < current_row + row_count {
                    return Some((line_idx + 1, display_row - current_row));
                }
                current_row += row_count;
            }
        }
        
        None
    }

    /// Convert buffer line and column to display row and column
    pub fn line_col_to_display(&self, line: usize, col: usize) -> Option<(usize, usize)> {
        let mut display_row = 0;
        
        for (line_idx, wrapped) in self.cache.iter().enumerate() {
            if let Some(wrapped) = wrapped {
                if line_idx + 1 == line {
                    // Find which segment contains this column
                    for (seg_idx, segment) in wrapped.segments.iter().enumerate() {
                        if col >= segment.start && col <= segment.end {
                            let display_col = segment.indent + (col - segment.start);
                            return Some((display_row + seg_idx, display_col));
                        }
                    }
                    // Column is past end, use last segment
                    if let Some(last) = wrapped.segments.last() {
                        let display_col = last.indent + (last.end - last.start);
                        return Some((display_row + wrapped.segments.len() - 1, display_col));
                    }
                }
                display_row += wrapped.row_count();
            }
        }
        
        None
    }
}

impl Default for SoftWrapEngine {
    fn default() -> Self {
        Self::new(SoftWrapConfig::default())
    }
}

/// Check if a character is a word boundary
fn is_word_boundary(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '-' | '/' | '\\' | '.' | ',' | ';' | ':' | '!' | '?' | '(' | ')' | '[' | ']' | '{' | '}')
}

/// Iterator over display rows
pub struct DisplayRowIterator<'a> {
    engine: &'a SoftWrapEngine,
    current_line: usize,
    current_segment: usize,
    display_row: usize,
}

impl<'a> DisplayRowIterator<'a> {
    pub fn new(engine: &'a SoftWrapEngine) -> Self {
        Self {
            engine,
            current_line: 0,
            current_segment: 0,
            display_row: 0,
        }
    }
}

impl<'a> Iterator for DisplayRowIterator<'a> {
    type Item = (usize, &'a WrapSegment); // (line_number, segment)

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let wrapped = self.engine.cache.get(self.current_line)?.as_ref()?;
            
            if self.current_segment < wrapped.segments.len() {
                let segment = &wrapped.segments[self.current_segment];
                self.current_segment += 1;
                self.display_row += 1;
                return Some((wrapped.line_number, segment));
            }
            
            self.current_line += 1;
            self.current_segment = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_wrap_short_line() {
        let engine = SoftWrapEngine::new(SoftWrapConfig::default());
        let wrapped = engine.wrap_line("Hello", 80);
        
        assert_eq!(wrapped.segments.len(), 1);
        assert_eq!(wrapped.segments[0].start, 0);
        assert_eq!(wrapped.segments[0].end, 5);
    }

    #[test]
    fn test_wrap_long_line() {
        let engine = SoftWrapEngine::new(SoftWrapConfig::default());
        let line = "This is a very long line that should definitely wrap at some point because it exceeds the maximum width";
        let wrapped = engine.wrap_line(line, 40);
        
        assert!(wrapped.segments.len() > 1, "Long line should wrap");
    }

    #[test]
    fn test_wrap_at_word_boundary() {
        let config = SoftWrapConfig {
            word_wrap: true,
            min_wrap_width: 5, // Lower minimum to test small widths
            ..Default::default()
        };
        let engine = SoftWrapEngine::new(config);
        let line = "Hello World Test";
        let wrapped = engine.wrap_line(line, 10);
        
        // Should wrap at word boundary, not in middle of word
        assert!(wrapped.segments.len() >= 2, "Line should wrap with width 10");
    }

    #[test]
    fn test_wrap_disabled() {
        let config = SoftWrapConfig {
            enabled: false,
            ..Default::default()
        };
        let engine = SoftWrapEngine::new(config);
        let line = "This is a very long line that would normally wrap";
        let wrapped = engine.wrap_line(line, 20);
        
        assert_eq!(wrapped.segments.len(), 1, "Should not wrap when disabled");
    }

    #[test]
    fn test_display_row_mapping() {
        let mut engine = SoftWrapEngine::new(SoftWrapConfig::default());
        let lines: Vec<&str> = vec!["Short", "This is a much longer line that will wrap", "End"];
        engine.wrap_buffer(&lines, 20);
        
        // First line maps to itself
        let (line, segment) = engine.display_row_to_line(0).unwrap();
        assert_eq!(line, 1);
        assert_eq!(segment, 0);
    }
}
