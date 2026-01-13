//! Diagnostic collection

use std::collections::HashMap;
use crate::{Diagnostic, Severity};

/// Collection of diagnostics from a single source
#[derive(Debug)]
pub struct DiagnosticCollection {
    /// Collection name (source)
    name: String,
    /// Diagnostics by file URI
    diagnostics: HashMap<String, Vec<Diagnostic>>,
}

impl DiagnosticCollection {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            diagnostics: HashMap::new(),
        }
    }

    /// Set diagnostics for a file
    pub fn set(&mut self, uri: &str, diagnostics: Vec<Diagnostic>) {
        if diagnostics.is_empty() {
            self.diagnostics.remove(uri);
        } else {
            // Set source on all diagnostics
            let diagnostics: Vec<Diagnostic> = diagnostics
                .into_iter()
                .map(|mut d| {
                    if d.source.is_none() {
                        d.source = Some(self.name.clone());
                    }
                    d
                })
                .collect();
            self.diagnostics.insert(uri.to_string(), diagnostics);
        }
    }

    /// Get diagnostics for file
    pub fn get(&self, uri: &str) -> Vec<&Diagnostic> {
        self.diagnostics
            .get(uri)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Delete diagnostics for file
    pub fn delete(&mut self, uri: &str) {
        self.diagnostics.remove(uri);
    }

    /// Clear all diagnostics
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    /// Get all file URIs with diagnostics
    pub fn uris(&self) -> Vec<&str> {
        self.diagnostics.keys().map(|s| s.as_str()).collect()
    }

    /// Get collection name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Has diagnostics?
    pub fn has(&self, uri: &str) -> bool {
        self.diagnostics.contains_key(uri)
    }

    /// Total diagnostic count
    pub fn count(&self) -> usize {
        self.diagnostics.values().map(|v| v.len()).sum()
    }

    /// Error count
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .values()
            .flat_map(|v| v.iter())
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    /// Warning count
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .values()
            .flat_map(|v| v.iter())
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// Get all diagnostics grouped by severity
    pub fn by_severity(&self) -> HashMap<Severity, Vec<(&str, &Diagnostic)>> {
        let mut result: HashMap<Severity, Vec<(&str, &Diagnostic)>> = HashMap::new();
        
        for (uri, diagnostics) in &self.diagnostics {
            for diagnostic in diagnostics {
                result
                    .entry(diagnostic.severity)
                    .or_default()
                    .push((uri.as_str(), diagnostic));
            }
        }
        
        result
    }

    /// Iterate all diagnostics
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Diagnostic)> {
        self.diagnostics
            .iter()
            .flat_map(|(uri, diags)| diags.iter().map(move |d| (uri.as_str(), d)))
    }

    /// Filter diagnostics
    pub fn filter<F>(&self, predicate: F) -> Vec<(&str, &Diagnostic)>
    where
        F: Fn(&Diagnostic) -> bool,
    {
        self.iter().filter(|(_, d)| predicate(d)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Range;

    #[test]
    fn test_collection() {
        let mut collection = DiagnosticCollection::new("test");
        
        let diag = Diagnostic::error("Test error", Range::at_line(1));
        collection.set("file:///test.rs", vec![diag]);
        
        assert_eq!(collection.count(), 1);
        assert_eq!(collection.error_count(), 1);
        
        collection.delete("file:///test.rs");
        assert_eq!(collection.count(), 0);
    }
}
