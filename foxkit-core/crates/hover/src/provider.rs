//! Hover providers

use crate::{Hover, HoverBuilder, HoverParams, Range, Position};

/// Hover provider trait
pub trait HoverProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Should this provider provide hover?
    fn should_provide(&self, params: &HoverParams) -> bool;

    /// Provide hover
    fn provide(&self, params: &HoverParams) -> Option<Hover>;
}

/// Built-in providers
pub mod builtin {
    use super::*;
    use std::collections::HashMap;

    /// Color hover provider (shows color previews)
    pub struct ColorHoverProvider;

    impl ColorHoverProvider {
        /// Parse hex color
        fn parse_hex(color: &str) -> Option<(u8, u8, u8, u8)> {
            let hex = color.trim_start_matches('#');
            match hex.len() {
                3 => {
                    let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                    let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                    let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                    Some((r, g, b, 255))
                }
                6 => {
                    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                    Some((r, g, b, 255))
                }
                8 => {
                    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                    let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                    Some((r, g, b, a))
                }
                _ => None,
            }
        }
    }

    impl HoverProvider for ColorHoverProvider {
        fn name(&self) -> &str {
            "color"
        }

        fn should_provide(&self, params: &HoverParams) -> bool {
            params.word.as_ref().map(|w| w.starts_with('#')).unwrap_or(false)
        }

        fn provide(&self, params: &HoverParams) -> Option<Hover> {
            let word = params.word.as_ref()?;
            let (r, g, b, a) = Self::parse_hex(word)?;
            
            let markdown = format!(
                "**Color Preview**\n\n\
                `{}` → rgb({}, {}, {}) rgba({}, {}, {}, {:.2})",
                word, r, g, b, r, g, b, a as f32 / 255.0
            );
            
            Some(Hover::markdown(&markdown))
        }
    }

    /// Keyword documentation provider
    pub struct KeywordHoverProvider {
        docs: HashMap<(String, String), String>,
    }

    impl KeywordHoverProvider {
        pub fn new() -> Self {
            let mut docs = HashMap::new();
            
            // Rust keywords
            docs.insert(
                ("rust".to_string(), "fn".to_string()),
                "Defines a function.\n\n```rust\nfn name(param: Type) -> ReturnType { }\n```".to_string(),
            );
            docs.insert(
                ("rust".to_string(), "let".to_string()),
                "Declares a variable binding.\n\n```rust\nlet x = 5;\nlet mut y = 10;\n```".to_string(),
            );
            docs.insert(
                ("rust".to_string(), "struct".to_string()),
                "Defines a struct type.\n\n```rust\nstruct Name {\n    field: Type,\n}\n```".to_string(),
            );
            docs.insert(
                ("rust".to_string(), "enum".to_string()),
                "Defines an enumeration type.\n\n```rust\nenum Name {\n    Variant1,\n    Variant2(Type),\n}\n```".to_string(),
            );
            docs.insert(
                ("rust".to_string(), "impl".to_string()),
                "Implements methods or traits for a type.\n\n```rust\nimpl Type {\n    fn method(&self) { }\n}\n```".to_string(),
            );
            docs.insert(
                ("rust".to_string(), "trait".to_string()),
                "Defines a trait (interface).\n\n```rust\ntrait Name {\n    fn method(&self);\n}\n```".to_string(),
            );
            docs.insert(
                ("rust".to_string(), "match".to_string()),
                "Pattern matching expression.\n\n```rust\nmatch value {\n    Pattern => expr,\n    _ => default,\n}\n```".to_string(),
            );
            docs.insert(
                ("rust".to_string(), "async".to_string()),
                "Marks a function or block as asynchronous.\n\n```rust\nasync fn fetch() { }\nlet future = async { };\n```".to_string(),
            );

            // JavaScript keywords
            docs.insert(
                ("javascript".to_string(), "const".to_string()),
                "Declares a constant (block-scoped, cannot be reassigned).\n\n```javascript\nconst x = 5;\n```".to_string(),
            );
            docs.insert(
                ("javascript".to_string(), "let".to_string()),
                "Declares a block-scoped variable.\n\n```javascript\nlet x = 5;\n```".to_string(),
            );
            docs.insert(
                ("javascript".to_string(), "async".to_string()),
                "Declares an async function.\n\n```javascript\nasync function fetch() { }\nconst fn = async () => { };\n```".to_string(),
            );

            Self { docs }
        }
    }

    impl Default for KeywordHoverProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HoverProvider for KeywordHoverProvider {
        fn name(&self) -> &str {
            "keyword"
        }

        fn should_provide(&self, params: &HoverParams) -> bool {
            params.word.is_some() && !params.language_id.is_empty()
        }

        fn provide(&self, params: &HoverParams) -> Option<Hover> {
            let word = params.word.as_ref()?;
            let key = (params.language_id.clone(), word.clone());
            
            self.docs.get(&key).map(|doc| Hover::markdown(doc))
        }
    }

    /// URL hover provider
    pub struct UrlHoverProvider;

    impl UrlHoverProvider {
        fn is_url(text: &str) -> bool {
            text.starts_with("http://") || text.starts_with("https://")
        }
    }

    impl HoverProvider for UrlHoverProvider {
        fn name(&self) -> &str {
            "url"
        }

        fn should_provide(&self, params: &HoverParams) -> bool {
            params.word.as_ref().map(|w| Self::is_url(w)).unwrap_or(false)
        }

        fn provide(&self, params: &HoverParams) -> Option<Hover> {
            let url = params.word.as_ref()?;
            if !Self::is_url(url) {
                return None;
            }
            
            Some(Hover::markdown(&format!(
                "**URL**: [{}]({})\n\nCtrl+Click to follow link",
                url, url
            )))
        }
    }
}
