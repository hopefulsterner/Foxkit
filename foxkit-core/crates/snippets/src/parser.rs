//! Snippet parser

use crate::snippet::{SnippetBody, SnippetPart, TabStop, Variable, Transform};

/// Parse snippet body text
pub struct SnippetParser;

impl SnippetParser {
    /// Parse snippet body string
    pub fn parse(body: &str) -> SnippetBody {
        let mut parts = Vec::new();
        let mut chars = body.chars().peekable();
        let mut text_buf = String::new();

        while let Some(c) = chars.next() {
            if c == '$' {
                // Flush text buffer
                if !text_buf.is_empty() {
                    parts.push(SnippetPart::Text(text_buf.clone()));
                    text_buf.clear();
                }

                match chars.peek() {
                    Some(&'{') => {
                        // ${...} complex placeholder
                        chars.next();
                        let content = Self::read_until_matching_brace(&mut chars);
                        if let Some(part) = Self::parse_placeholder(&content) {
                            parts.push(part);
                        }
                    }
                    Some(&c) if c.is_ascii_digit() => {
                        // $N simple tab stop
                        let num = Self::read_number(&mut chars);
                        parts.push(SnippetPart::TabStop(TabStop::new(num)));
                    }
                    Some(&c) if c.is_ascii_alphabetic() || c == '_' => {
                        // $VAR simple variable
                        let name = Self::read_identifier(&mut chars);
                        parts.push(SnippetPart::Variable(Variable::new(&name)));
                    }
                    Some(&'$') => {
                        // $$ escaped dollar sign
                        chars.next();
                        text_buf.push('$');
                    }
                    _ => {
                        text_buf.push('$');
                    }
                }
            } else if c == '\\' {
                // Escape sequence
                if let Some(&next) = chars.peek() {
                    if matches!(next, '$' | '}' | '\\') {
                        chars.next();
                        text_buf.push(next);
                    } else {
                        text_buf.push(c);
                    }
                } else {
                    text_buf.push(c);
                }
            } else {
                text_buf.push(c);
            }
        }

        // Flush remaining text
        if !text_buf.is_empty() {
            parts.push(SnippetPart::Text(text_buf));
        }

        SnippetBody::new(parts)
    }

    fn read_until_matching_brace<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> String {
        let mut content = String::new();
        let mut depth = 1;

        while let Some(c) = chars.next() {
            if c == '{' {
                depth += 1;
                content.push(c);
            } else if c == '}' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                content.push(c);
            } else if c == '\\' {
                content.push(c);
                if let Some(next) = chars.next() {
                    content.push(next);
                }
            } else {
                content.push(c);
            }
        }

        content
    }

    fn read_number<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> usize {
        let mut num = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                num.push(chars.next().unwrap());
            } else {
                break;
            }
        }
        num.parse().unwrap_or(0)
    }

    fn read_identifier<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> String {
        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                name.push(chars.next().unwrap());
            } else {
                break;
            }
        }
        name
    }

    fn parse_placeholder(content: &str) -> Option<SnippetPart> {
        let content = content.trim();
        
        // Check for choice: ${1|one,two,three|}
        if let Some((num_part, rest)) = content.split_once('|') {
            let num: usize = num_part.parse().ok()?;
            let choices_str = rest.strip_suffix('|')?;
            let choices: Vec<String> = choices_str.split(',').map(|s| s.to_string()).collect();
            return Some(SnippetPart::Choice(TabStop::new(num), choices));
        }

        // Check for placeholder with default: ${1:default}
        if let Some((num_part, default)) = content.split_once(':') {
            if let Ok(num) = num_part.parse::<usize>() {
                let mut ts = TabStop::new(num);
                ts.placeholder = Some(default.to_string());
                return Some(SnippetPart::TabStop(ts));
            } else {
                // Variable with default: ${VAR:default}
                let mut var = Variable::new(num_part);
                var.default = Some(default.to_string());
                return Some(SnippetPart::Variable(var));
            }
        }

        // Check for transform: ${1/regex/replacement/flags}
        if content.contains('/') {
            let parts: Vec<&str> = content.splitn(4, '/').collect();
            if parts.len() >= 3 {
                let num: usize = parts[0].parse().ok()?;
                let mut ts = TabStop::new(num);
                ts.transform = Some(Transform {
                    regex: parts[1].to_string(),
                    replacement: parts.get(2).unwrap_or(&"").to_string(),
                    flags: parts.get(3).unwrap_or(&"").to_string(),
                });
                return Some(SnippetPart::TabStop(ts));
            }
        }

        // Simple tab stop: ${1}
        if let Ok(num) = content.parse::<usize>() {
            return Some(SnippetPart::TabStop(TabStop::new(num)));
        }

        // Simple variable: ${VAR}
        if content.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Some(SnippetPart::Variable(Variable::new(content)));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_simple_tabstop() {
        let body = SnippetParser::parse("Hello $1!");
        let mut values = HashMap::new();
        values.insert(1, "World".to_string());
        assert_eq!(body.expand(&values), "Hello World!");
    }

    #[test]
    fn test_placeholder() {
        let body = SnippetParser::parse("function ${1:name}() {}");
        let values = HashMap::new();
        assert_eq!(body.expand(&values), "function name() {}");
    }

    #[test]
    fn test_multiple_tabstops() {
        let body = SnippetParser::parse("$1 + $2 = $0");
        let mut values = HashMap::new();
        values.insert(1, "1".to_string());
        values.insert(2, "2".to_string());
        values.insert(0, "3".to_string());
        assert_eq!(body.expand(&values), "1 + 2 = 3");
    }
}
