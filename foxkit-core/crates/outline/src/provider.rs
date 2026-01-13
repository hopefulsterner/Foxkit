//! Outline providers

use crate::{DocumentSymbol, Outline, OutlineRequest, Range, Position, SymbolKind};

/// Outline provider trait
pub trait OutlineProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Languages supported
    fn supports(&self, language_id: &str) -> bool;

    /// Provide outline
    fn provide(&self, request: &OutlineRequest) -> Option<Outline>;
}

/// Built-in providers
pub mod builtin {
    use super::*;
    use std::collections::HashSet;

    /// Simple regex-based outline provider (for basic structure)
    pub struct SimpleOutlineProvider {
        languages: HashSet<String>,
    }

    impl SimpleOutlineProvider {
        pub fn new(languages: &[&str]) -> Self {
            Self {
                languages: languages.iter().map(|s| s.to_string()).collect(),
            }
        }

        pub fn rust() -> Self {
            Self::new(&["rust"])
        }

        pub fn javascript() -> Self {
            Self::new(&["javascript", "typescript", "javascriptreact", "typescriptreact"])
        }

        fn parse_rust(&self, content: &str) -> Vec<DocumentSymbol> {
            let mut symbols = Vec::new();
            let mut current_impl: Option<(String, u32, Vec<DocumentSymbol>)> = None;

            for (line_num, line) in content.lines().enumerate() {
                let line_num = line_num as u32;
                let trimmed = line.trim();

                // End of impl block
                if let Some((name, start, children)) = current_impl.take() {
                    if trimmed == "}" && !line.starts_with(' ') {
                        symbols.push(DocumentSymbol {
                            name,
                            detail: None,
                            kind: SymbolKind::Class,
                            tags: Vec::new(),
                            range: Range::new(
                                Position::new(start, 0),
                                Position::new(line_num, 0),
                            ),
                            selection_range: Range::at_line(start),
                            children,
                        });
                        continue;
                    } else {
                        current_impl = Some((name, start, children));
                    }
                }

                // Function
                if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || 
                   trimmed.starts_with("async fn ") || trimmed.starts_with("pub async fn ") {
                    if let Some(name) = extract_name(trimmed, "fn ") {
                        let symbol = DocumentSymbol::new(&name, SymbolKind::Function, Range::at_line(line_num));
                        if let Some((_, _, ref mut children)) = current_impl {
                            children.push(DocumentSymbol::new(&name, SymbolKind::Method, Range::at_line(line_num)));
                        } else {
                            symbols.push(symbol);
                        }
                    }
                }
                // Struct
                else if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
                    if let Some(name) = extract_name(trimmed, "struct ") {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::Struct, Range::at_line(line_num)));
                    }
                }
                // Enum
                else if trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") {
                    if let Some(name) = extract_name(trimmed, "enum ") {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::Enum, Range::at_line(line_num)));
                    }
                }
                // Trait
                else if trimmed.starts_with("trait ") || trimmed.starts_with("pub trait ") {
                    if let Some(name) = extract_name(trimmed, "trait ") {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::Interface, Range::at_line(line_num)));
                    }
                }
                // Impl
                else if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
                    let name = parse_impl_name(trimmed);
                    current_impl = Some((name, line_num, Vec::new()));
                }
                // Const
                else if trimmed.starts_with("const ") || trimmed.starts_with("pub const ") {
                    if let Some(name) = extract_name(trimmed, "const ") {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::Constant, Range::at_line(line_num)));
                    }
                }
                // Mod
                else if trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ") {
                    if let Some(name) = extract_name(trimmed, "mod ") {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::Module, Range::at_line(line_num)));
                    }
                }
            }

            symbols
        }

        fn parse_javascript(&self, content: &str) -> Vec<DocumentSymbol> {
            let mut symbols = Vec::new();

            for (line_num, line) in content.lines().enumerate() {
                let line_num = line_num as u32;
                let trimmed = line.trim();

                // Function declarations
                if trimmed.starts_with("function ") || trimmed.starts_with("async function ") {
                    if let Some(name) = extract_name(trimmed, "function ") {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::Function, Range::at_line(line_num)));
                    }
                }
                // Class declarations
                else if trimmed.starts_with("class ") || trimmed.starts_with("export class ") {
                    if let Some(name) = extract_name(trimmed, "class ") {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::Class, Range::at_line(line_num)));
                    }
                }
                // Interface (TypeScript)
                else if trimmed.starts_with("interface ") || trimmed.starts_with("export interface ") {
                    if let Some(name) = extract_name(trimmed, "interface ") {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::Interface, Range::at_line(line_num)));
                    }
                }
                // Type (TypeScript)
                else if trimmed.starts_with("type ") || trimmed.starts_with("export type ") {
                    if let Some(name) = extract_name(trimmed, "type ") {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::TypeParameter, Range::at_line(line_num)));
                    }
                }
                // Const/let/var with arrow function
                else if (trimmed.starts_with("const ") || trimmed.starts_with("let ") || 
                         trimmed.starts_with("export const ")) && trimmed.contains("=>") {
                    if let Some(name) = extract_const_name(trimmed) {
                        symbols.push(DocumentSymbol::new(&name, SymbolKind::Function, Range::at_line(line_num)));
                    }
                }
            }

            symbols
        }
    }

    impl OutlineProvider for SimpleOutlineProvider {
        fn name(&self) -> &str {
            "simple"
        }

        fn supports(&self, language_id: &str) -> bool {
            self.languages.contains(language_id)
        }

        fn provide(&self, request: &OutlineRequest) -> Option<Outline> {
            let symbols = match request.language_id.as_str() {
                "rust" => self.parse_rust(&request.content),
                "javascript" | "typescript" | "javascriptreact" | "typescriptreact" => {
                    self.parse_javascript(&request.content)
                }
                _ => return None,
            };

            Some(Outline::new(&request.file_path).with_symbols(symbols))
        }
    }

    fn extract_name(line: &str, keyword: &str) -> Option<String> {
        let after_keyword = line.split(keyword).nth(1)?;
        let name: String = after_keyword
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        
        if name.is_empty() { None } else { Some(name) }
    }

    fn extract_const_name(line: &str) -> Option<String> {
        let after_const = line.split("const ").nth(1).or_else(|| line.split("let ").nth(1))?;
        let name: String = after_const
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        
        if name.is_empty() { None } else { Some(name) }
    }

    fn parse_impl_name(line: &str) -> String {
        // Simple impl name extraction
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts.iter()
                .skip(1)
                .find(|p| !p.starts_with('<'))
                .map(|s| s.trim_end_matches('{').trim_end_matches('<'))
                .unwrap_or("impl");
            name.to_string()
        } else {
            "impl".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::builtin::SimpleOutlineProvider;

    #[test]
    fn test_rust_outline() {
        let provider = SimpleOutlineProvider::rust();
        let request = OutlineRequest::new(
            "test.rs",
            r#"
fn main() {
    println!("Hello");
}

pub struct User {
    name: String,
}

impl User {
    fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}
"#,
            "rust",
        );

        let outline = provider.provide(&request).unwrap();
        assert!(outline.symbols.len() >= 2);
    }
}
