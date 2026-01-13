//! Snippet registry

use std::collections::HashMap;
use std::path::Path;
use crate::{Snippet, SnippetFile, SnippetDefinition, SnippetParser, snippet::SnippetBody};

/// Registry of all snippets
pub struct SnippetRegistry {
    /// Snippets by language
    by_language: HashMap<String, Vec<Snippet>>,
    /// Global snippets
    global: Vec<Snippet>,
}

impl SnippetRegistry {
    pub fn new() -> Self {
        Self {
            by_language: HashMap::new(),
            global: Vec::new(),
        }
    }

    /// Load with built-in snippets
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        
        for (lang, file) in crate::builtin_snippets() {
            registry.load_file(&file, Some(&lang));
        }
        
        registry
    }

    /// Load snippets from file
    pub fn load_from_file(&mut self, path: &Path) -> anyhow::Result<()> {
        let file = SnippetFile::from_file(path)?;
        
        // Determine language from filename
        let language = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
        
        self.load_file(&file, language.as_deref());
        Ok(())
    }

    /// Load snippets from a SnippetFile
    pub fn load_file(&mut self, file: &SnippetFile, default_language: Option<&str>) {
        for (name, def) in &file.snippets {
            let snippets = self.definition_to_snippets(name, def);
            
            for snippet in snippets {
                // Determine scope
                if let Some(ref scope) = def.scope {
                    for lang in scope.split(',') {
                        let lang = lang.trim();
                        self.by_language
                            .entry(lang.to_string())
                            .or_default()
                            .push(snippet.clone());
                    }
                } else if let Some(lang) = default_language {
                    self.by_language
                        .entry(lang.to_string())
                        .or_default()
                        .push(snippet);
                } else {
                    self.global.push(snippet);
                }
            }
        }
    }

    /// Get snippets for language
    pub fn get(&self, language: &str) -> Vec<&Snippet> {
        let mut snippets: Vec<&Snippet> = self.global.iter().collect();
        
        if let Some(lang_snippets) = self.by_language.get(language) {
            snippets.extend(lang_snippets.iter());
        }
        
        snippets
    }

    /// Get snippet by prefix
    pub fn get_by_prefix(&self, language: &str, prefix: &str) -> Option<&Snippet> {
        self.get(language)
            .into_iter()
            .find(|s| s.prefix == prefix)
    }

    /// Get completions matching prefix
    pub fn completions(&self, language: &str, prefix: &str) -> Vec<&Snippet> {
        self.get(language)
            .into_iter()
            .filter(|s| s.prefix.starts_with(prefix))
            .collect()
    }

    /// Add a snippet
    pub fn add(&mut self, snippet: Snippet, language: Option<&str>) {
        if let Some(lang) = language {
            self.by_language
                .entry(lang.to_string())
                .or_default()
                .push(snippet);
        } else {
            self.global.push(snippet);
        }
    }

    /// Remove snippet by prefix
    pub fn remove(&mut self, language: Option<&str>, prefix: &str) {
        if let Some(lang) = language {
            if let Some(snippets) = self.by_language.get_mut(lang) {
                snippets.retain(|s| s.prefix != prefix);
            }
        } else {
            self.global.retain(|s| s.prefix != prefix);
        }
    }

    /// Get all languages with snippets
    pub fn languages(&self) -> Vec<&str> {
        self.by_language.keys().map(|s| s.as_str()).collect()
    }

    /// Count total snippets
    pub fn count(&self) -> usize {
        self.global.len() + self.by_language.values().map(|v| v.len()).sum::<usize>()
    }

    fn definition_to_snippets(&self, name: &str, def: &SnippetDefinition) -> Vec<Snippet> {
        let body_text = def.body_text();
        let body = SnippetParser::parse(&body_text);
        
        def.prefixes()
            .into_iter()
            .map(|prefix| {
                let mut snippet = Snippet::new(name, prefix, body.clone());
                snippet.description = def.description.clone();
                snippet.scope = def.scope.clone();
                snippet
            })
            .collect()
    }
}

impl Default for SnippetRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = SnippetRegistry::with_builtins();
        
        // Check rust snippets
        let rust_snippets = registry.get("rust");
        assert!(!rust_snippets.is_empty());
        
        // Find fn snippet
        let fn_snippet = registry.get_by_prefix("rust", "fn");
        assert!(fn_snippet.is_some());
    }
}
