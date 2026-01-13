//! Diagnostic source

use crate::Diagnostic;

/// A source of diagnostics
pub trait DiagnosticSource: Send + Sync {
    /// Source name
    fn name(&self) -> &str;

    /// Check file for diagnostics
    fn check(&self, uri: &str, content: &str) -> Vec<Diagnostic>;

    /// Supported file extensions
    fn extensions(&self) -> &[&str];

    /// Is this source enabled?
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Built-in diagnostic sources
pub mod builtin {
    use super::*;
    use crate::{Range, Position};

    /// Simple trailing whitespace checker
    pub struct TrailingWhitespaceChecker;

    impl DiagnosticSource for TrailingWhitespaceChecker {
        fn name(&self) -> &str {
            "trailing-whitespace"
        }

        fn check(&self, _uri: &str, content: &str) -> Vec<Diagnostic> {
            let mut diagnostics = Vec::new();

            for (line_num, line) in content.lines().enumerate() {
                let trimmed = line.trim_end();
                if trimmed.len() < line.len() {
                    let start = trimmed.len();
                    let end = line.len();
                    
                    let range = Range::new(
                        Position::new(line_num as u32, start as u32),
                        Position::new(line_num as u32, end as u32),
                    );
                    
                    diagnostics.push(
                        Diagnostic::hint("Trailing whitespace", range)
                            .with_source(self.name())
                            .with_tag(crate::DiagnosticTag::Unnecessary)
                    );
                }
            }

            diagnostics
        }

        fn extensions(&self) -> &[&str] {
            &["*"]
        }
    }

    /// TODO comment finder
    pub struct TodoFinder;

    impl DiagnosticSource for TodoFinder {
        fn name(&self) -> &str {
            "todo"
        }

        fn check(&self, _uri: &str, content: &str) -> Vec<Diagnostic> {
            let mut diagnostics = Vec::new();
            let patterns = ["TODO", "FIXME", "HACK", "XXX"];

            for (line_num, line) in content.lines().enumerate() {
                for pattern in &patterns {
                    if let Some(col) = line.find(pattern) {
                        let range = Range::new(
                            Position::new(line_num as u32, col as u32),
                            Position::new(line_num as u32, (col + pattern.len()) as u32),
                        );
                        
                        diagnostics.push(
                            Diagnostic::info(&format!("{} comment", pattern), range)
                                .with_source(self.name())
                        );
                    }
                }
            }

            diagnostics
        }

        fn extensions(&self) -> &[&str] {
            &["*"]
        }
    }
}
