//! Completion providers

use crate::{CompletionContext, CompletionItem, CompletionList};

/// Completion provider trait
pub trait CompletionProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Should this provider provide completions?
    fn should_provide(&self, context: &CompletionContext) -> bool;

    /// Provide completions
    fn provide(&self, context: &CompletionContext) -> CompletionList;

    /// Resolve additional item details
    fn resolve(&self, item: &CompletionItem) -> Option<CompletionItem> {
        None
    }
}

/// Built-in providers
pub mod builtin {
    use super::*;
    use crate::item::CompletionKind;

    /// Keyword completion provider
    pub struct KeywordProvider {
        keywords: std::collections::HashMap<String, Vec<String>>,
    }

    impl KeywordProvider {
        pub fn new() -> Self {
            let mut keywords: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
            
            // Rust keywords
            keywords.insert("rust".to_string(), vec![
                "fn", "let", "mut", "const", "static", "struct", "enum", "trait",
                "impl", "pub", "mod", "use", "crate", "self", "super", "where",
                "if", "else", "match", "loop", "while", "for", "in", "break",
                "continue", "return", "async", "await", "move", "ref", "type",
                "dyn", "unsafe", "extern", "macro_rules",
            ].into_iter().map(String::from).collect());

            // JavaScript/TypeScript keywords
            keywords.insert("javascript".to_string(), vec![
                "function", "const", "let", "var", "class", "extends", "implements",
                "interface", "type", "enum", "import", "export", "from", "default",
                "if", "else", "switch", "case", "for", "while", "do", "break",
                "continue", "return", "try", "catch", "finally", "throw", "async",
                "await", "yield", "new", "this", "super", "typeof", "instanceof",
            ].into_iter().map(String::from).collect());
            keywords.insert("typescript".to_string(), keywords["javascript"].clone());

            // Python keywords
            keywords.insert("python".to_string(), vec![
                "def", "class", "if", "elif", "else", "for", "while", "try",
                "except", "finally", "with", "as", "import", "from", "return",
                "yield", "raise", "pass", "break", "continue", "lambda", "and",
                "or", "not", "in", "is", "True", "False", "None", "global",
                "nonlocal", "async", "await",
            ].into_iter().map(String::from).collect());

            Self { keywords }
        }
    }

    impl Default for KeywordProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl CompletionProvider for KeywordProvider {
        fn name(&self) -> &str {
            "keywords"
        }

        fn should_provide(&self, context: &CompletionContext) -> bool {
            !context.word.is_empty() && self.keywords.contains_key(&context.language_id)
        }

        fn provide(&self, context: &CompletionContext) -> CompletionList {
            let keywords = match self.keywords.get(&context.language_id) {
                Some(kw) => kw,
                None => return CompletionList::empty(),
            };

            let word_lower = context.word.to_lowercase();
            let items: Vec<_> = keywords
                .iter()
                .filter(|kw| kw.to_lowercase().starts_with(&word_lower))
                .map(|kw| CompletionItem::keyword(kw))
                .collect();

            CompletionList::new(items)
        }
    }

    /// Word-based completion (from document)
    pub struct WordProvider {
        /// Minimum word length
        min_length: usize,
    }

    impl WordProvider {
        pub fn new() -> Self {
            Self { min_length: 3 }
        }

        pub fn with_min_length(min_length: usize) -> Self {
            Self { min_length }
        }

        /// Extract words from text
        pub fn extract_words(text: &str) -> Vec<String> {
            let mut words = std::collections::HashSet::new();
            let mut current = String::new();

            for c in text.chars() {
                if c.is_alphanumeric() || c == '_' {
                    current.push(c);
                } else {
                    if current.len() >= 3 {
                        words.insert(current.clone());
                    }
                    current.clear();
                }
            }

            if current.len() >= 3 {
                words.insert(current);
            }

            words.into_iter().collect()
        }
    }

    impl Default for WordProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl CompletionProvider for WordProvider {
        fn name(&self) -> &str {
            "words"
        }

        fn should_provide(&self, context: &CompletionContext) -> bool {
            context.word.len() >= 2
        }

        fn provide(&self, _context: &CompletionContext) -> CompletionList {
            // In a real implementation, this would scan the document
            CompletionList::empty()
        }
    }

    /// Path completion provider
    pub struct PathProvider;

    impl CompletionProvider for PathProvider {
        fn name(&self) -> &str {
            "path"
        }

        fn should_provide(&self, context: &CompletionContext) -> bool {
            context.trigger_character == Some('/') 
                || context.trigger_character == Some('\\')
                || context.word.starts_with("./")
                || context.word.starts_with("../")
        }

        fn provide(&self, _context: &CompletionContext) -> CompletionList {
            // In a real implementation, this would list files/directories
            CompletionList::empty()
        }
    }
}
