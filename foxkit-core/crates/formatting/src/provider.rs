//! Formatting providers

use crate::{FormatRequest, FormatResult, TextEdit, Range, Position, FormattingOptions};

/// Formatting provider trait
pub trait FormattingProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Languages supported
    fn supports(&self, language_id: &str) -> bool;

    /// Format document or range
    fn format(&self, request: &FormatRequest) -> Option<FormatResult>;

    /// Characters that trigger format on type
    fn format_on_type_chars(&self) -> &[char] {
        &[]
    }

    /// Format after typing a character
    fn format_on_type(&self, request: &FormatRequest, char: char) -> Option<FormatResult> {
        None
    }
}

/// Built-in providers
pub mod builtin {
    use super::*;
    use std::collections::HashSet;

    /// Simple whitespace formatter
    pub struct WhitespaceFormatter;

    impl FormattingProvider for WhitespaceFormatter {
        fn name(&self) -> &str {
            "whitespace"
        }

        fn supports(&self, _language_id: &str) -> bool {
            true // Works for all languages
        }

        fn format(&self, request: &FormatRequest) -> Option<FormatResult> {
            let mut edits = Vec::new();
            let options = &request.options;

            for (line_num, line) in request.content.lines().enumerate() {
                let line_num = line_num as u32;

                // Skip if outside range
                if let Some(range) = &request.range {
                    if line_num < range.start.line || line_num > range.end.line {
                        continue;
                    }
                }

                // Trim trailing whitespace
                if options.trim_trailing_whitespace {
                    let trimmed = line.trim_end();
                    if trimmed.len() < line.len() {
                        edits.push(TextEdit::new(
                            Range::new(
                                Position::new(line_num, trimmed.len() as u32),
                                Position::new(line_num, line.len() as u32),
                            ),
                            "",
                        ));
                    }
                }

                // Convert tabs to spaces or vice versa
                let leading_ws: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                if !leading_ws.is_empty() {
                    let converted = convert_indentation(&leading_ws, options);
                    if converted != leading_ws {
                        edits.push(TextEdit::new(
                            Range::new(
                                Position::new(line_num, 0),
                                Position::new(line_num, leading_ws.len() as u32),
                            ),
                            &converted,
                        ));
                    }
                }
            }

            if edits.is_empty() {
                None
            } else {
                Some(FormatResult::edits(edits))
            }
        }
    }

    fn convert_indentation(ws: &str, options: &FormattingOptions) -> String {
        if options.insert_spaces {
            // Convert tabs to spaces
            ws.chars()
                .map(|c| {
                    if c == '\t' {
                        " ".repeat(options.tab_size as usize)
                    } else {
                        c.to_string()
                    }
                })
                .collect()
        } else {
            // Convert spaces to tabs
            let space_count: usize = ws.chars().filter(|c| *c == ' ').count();
            let tab_count = ws.chars().filter(|c| *c == '\t').count();
            let total_tabs = tab_count + space_count / options.tab_size as usize;
            let remaining_spaces = space_count % options.tab_size as usize;
            
            format!(
                "{}{}",
                "\t".repeat(total_tabs),
                " ".repeat(remaining_spaces)
            )
        }
    }

    /// Auto-indent formatter (format on type)
    pub struct AutoIndentFormatter {
        languages: HashSet<String>,
        increase_indent: Vec<char>, // Characters that increase indent
        decrease_indent: Vec<char>, // Characters that decrease indent
    }

    impl AutoIndentFormatter {
        pub fn new() -> Self {
            Self {
                languages: ["rust", "javascript", "typescript", "c", "cpp", "java", "go"]
                    .iter().map(|s| s.to_string()).collect(),
                increase_indent: vec!['{', '(', '['],
                decrease_indent: vec!['}', ')', ']'],
            }
        }
    }

    impl Default for AutoIndentFormatter {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FormattingProvider for AutoIndentFormatter {
        fn name(&self) -> &str {
            "auto-indent"
        }

        fn supports(&self, language_id: &str) -> bool {
            self.languages.contains(language_id)
        }

        fn format(&self, _request: &FormatRequest) -> Option<FormatResult> {
            // This provider only works on type
            None
        }

        fn format_on_type_chars(&self) -> &[char] {
            &['\n', '}', ']', ')']
        }

        fn format_on_type(&self, request: &FormatRequest, typed_char: char) -> Option<FormatResult> {
            if typed_char == '\n' {
                // Auto-indent new line
                // Find current line and determine indent
                let lines: Vec<&str> = request.content.lines().collect();
                if let Some(range) = &request.range {
                    let prev_line = range.start.line.saturating_sub(1) as usize;
                    if prev_line < lines.len() {
                        let prev = lines[prev_line];
                        let current_indent: String = prev.chars()
                            .take_while(|c| c.is_whitespace())
                            .collect();
                        
                        // Check if prev line ends with opener
                        let trimmed = prev.trim_end();
                        let indent = if trimmed.ends_with('{') || 
                                       trimmed.ends_with('(') || 
                                       trimmed.ends_with('[') {
                            format!("{}{}", current_indent, request.options.indent())
                        } else {
                            current_indent
                        };

                        if !indent.is_empty() {
                            return Some(FormatResult::edits(vec![
                                TextEdit::insert(
                                    Position::new(range.start.line, 0),
                                    &indent,
                                )
                            ]));
                        }
                    }
                }
            } else if self.decrease_indent.contains(&typed_char) {
                // Decrease indent for closing brackets
                if let Some(range) = &request.range {
                    let lines: Vec<&str> = request.content.lines().collect();
                    let line_num = range.start.line as usize;
                    if line_num < lines.len() {
                        let line = lines[line_num];
                        let trimmed = line.trim();
                        
                        // If line only contains the closing bracket
                        if trimmed.len() == 1 && trimmed.chars().next() == Some(typed_char) {
                            let current_indent: String = line.chars()
                                .take_while(|c| c.is_whitespace())
                                .collect();
                            
                            // Reduce indent by one level
                            let indent_str = request.options.indent();
                            if current_indent.len() >= indent_str.len() {
                                let new_indent = &current_indent[..current_indent.len() - indent_str.len()];
                                return Some(FormatResult::edits(vec![
                                    TextEdit::new(
                                        Range::new(
                                            Position::new(line_num as u32, 0),
                                            Position::new(line_num as u32, current_indent.len() as u32),
                                        ),
                                        new_indent,
                                    )
                                ]));
                            }
                        }
                    }
                }
            }

            None
        }
    }
}
