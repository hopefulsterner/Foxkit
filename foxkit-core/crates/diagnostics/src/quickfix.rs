//! Quick Fix Integration
//!
//! Diagnostic-based code actions and fixes.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Quick fix for a diagnostic
#[derive(Debug, Clone)]
pub struct QuickFix {
    /// Unique ID
    pub id: String,
    /// Display title
    pub title: String,
    /// Kind of fix
    pub kind: QuickFixKind,
    /// Edits to apply
    pub edits: Vec<QuickFixEdit>,
    /// Is this the preferred fix?
    pub is_preferred: bool,
    /// Diagnostic codes this fix applies to
    pub diagnostic_codes: Vec<String>,
}

/// Kind of quick fix
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickFixKind {
    /// Quick fix for an error/warning
    QuickFix,
    /// Refactoring suggestion
    Refactor,
    /// Extract code
    RefactorExtract,
    /// Inline code
    RefactorInline,
    /// Rewrite code
    RefactorRewrite,
    /// Source action (organize imports, etc.)
    Source,
    /// Organize imports
    SourceOrganizeImports,
    /// Fix all
    SourceFixAll,
}

impl QuickFixKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::QuickFix => "quickfix",
            Self::Refactor => "refactor",
            Self::RefactorExtract => "refactor.extract",
            Self::RefactorInline => "refactor.inline",
            Self::RefactorRewrite => "refactor.rewrite",
            Self::Source => "source",
            Self::SourceOrganizeImports => "source.organizeImports",
            Self::SourceFixAll => "source.fixAll",
        }
    }
}

/// Quick fix edit
#[derive(Debug, Clone)]
pub struct QuickFixEdit {
    /// File URI
    pub file_uri: String,
    /// Range to replace
    pub range: QuickFixRange,
    /// New text
    pub new_text: String,
}

/// Range for quick fix
#[derive(Debug, Clone, Copy)]
pub struct QuickFixRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Quick fix provider trait
pub trait QuickFixProvider: Send + Sync {
    /// Get fixes for a diagnostic
    fn get_fixes(&self, diagnostic: &DiagnosticInfo) -> Vec<QuickFix>;
    
    /// Get all available fixes for a range
    fn get_fixes_in_range(&self, file_uri: &str, range: QuickFixRange) -> Vec<QuickFix>;
}

/// Diagnostic info for fix lookup
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    pub code: Option<String>,
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub source: Option<String>,
    pub range: QuickFixRange,
    pub file_uri: String,
}

/// Diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Built-in quick fix registry
pub struct QuickFixRegistry {
    /// Registered fix providers by language
    providers: RwLock<HashMap<String, Vec<Arc<dyn QuickFixProvider>>>>,
    /// Built-in fixes by diagnostic code
    builtin_fixes: RwLock<HashMap<String, Vec<QuickFix>>>,
}

impl QuickFixRegistry {
    pub fn new() -> Self {
        let registry = Self {
            providers: RwLock::new(HashMap::new()),
            builtin_fixes: RwLock::new(HashMap::new()),
        };
        registry.register_builtin_fixes();
        registry
    }

    /// Register a provider for a language
    pub fn register_provider(&self, language: &str, provider: Arc<dyn QuickFixProvider>) {
        self.providers
            .write()
            .entry(language.to_string())
            .or_default()
            .push(provider);
    }

    /// Register a built-in fix for a diagnostic code
    pub fn register_builtin(&self, code: &str, fix: QuickFix) {
        self.builtin_fixes
            .write()
            .entry(code.to_string())
            .or_default()
            .push(fix);
    }

    /// Get fixes for a diagnostic
    pub fn get_fixes(&self, language: &str, diagnostic: &DiagnosticInfo) -> Vec<QuickFix> {
        let mut fixes = Vec::new();

        // Get from providers
        if let Some(providers) = self.providers.read().get(language) {
            for provider in providers {
                fixes.extend(provider.get_fixes(diagnostic));
            }
        }

        // Get built-in fixes
        if let Some(code) = &diagnostic.code {
            if let Some(builtin) = self.builtin_fixes.read().get(code) {
                fixes.extend(builtin.iter().cloned());
            }
        }

        fixes
    }

    fn register_builtin_fixes(&self) {
        // Rust fixes
        self.register_rust_fixes();
        // TypeScript fixes
        self.register_typescript_fixes();
        // Python fixes
        self.register_python_fixes();
    }

    fn register_rust_fixes(&self) {
        // Unused import
        self.register_builtin("unused_imports", QuickFix {
            id: "rust.remove_unused_import".to_string(),
            title: "Remove unused import".to_string(),
            kind: QuickFixKind::QuickFix,
            edits: Vec::new(), // Will be populated by provider
            is_preferred: true,
            diagnostic_codes: vec!["unused_imports".to_string()],
        });

        // Missing semicolon
        self.register_builtin("E0308", QuickFix {
            id: "rust.add_semicolon".to_string(),
            title: "Add semicolon".to_string(),
            kind: QuickFixKind::QuickFix,
            edits: Vec::new(),
            is_preferred: true,
            diagnostic_codes: vec!["E0308".to_string()],
        });

        // Dead code
        self.register_builtin("dead_code", QuickFix {
            id: "rust.add_allow_dead_code".to_string(),
            title: "Add #[allow(dead_code)]".to_string(),
            kind: QuickFixKind::QuickFix,
            edits: Vec::new(),
            is_preferred: false,
            diagnostic_codes: vec!["dead_code".to_string()],
        });
    }

    fn register_typescript_fixes(&self) {
        // Missing import
        self.register_builtin("2304", QuickFix {
            id: "ts.add_import".to_string(),
            title: "Add import".to_string(),
            kind: QuickFixKind::QuickFix,
            edits: Vec::new(),
            is_preferred: true,
            diagnostic_codes: vec!["2304".to_string()],
        });

        // Missing semicolon
        self.register_builtin("1005", QuickFix {
            id: "ts.add_semicolon".to_string(),
            title: "Add semicolon".to_string(),
            kind: QuickFixKind::QuickFix,
            edits: Vec::new(),
            is_preferred: true,
            diagnostic_codes: vec!["1005".to_string()],
        });
    }

    fn register_python_fixes(&self) {
        // Undefined name
        self.register_builtin("F821", QuickFix {
            id: "py.add_import".to_string(),
            title: "Add import".to_string(),
            kind: QuickFixKind::QuickFix,
            edits: Vec::new(),
            is_preferred: true,
            diagnostic_codes: vec!["F821".to_string()],
        });

        // Unused import
        self.register_builtin("F401", QuickFix {
            id: "py.remove_unused_import".to_string(),
            title: "Remove unused import".to_string(),
            kind: QuickFixKind::QuickFix,
            edits: Vec::new(),
            is_preferred: true,
            diagnostic_codes: vec!["F401".to_string()],
        });
    }
}

impl Default for QuickFixRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Diagnostic grouping
pub struct DiagnosticGroup {
    pub name: String,
    pub diagnostics: Vec<DiagnosticInfo>,
}

/// Group diagnostics by various criteria
pub struct DiagnosticGrouper;

impl DiagnosticGrouper {
    /// Group by file
    pub fn by_file(diagnostics: Vec<DiagnosticInfo>) -> HashMap<String, Vec<DiagnosticInfo>> {
        let mut groups: HashMap<String, Vec<DiagnosticInfo>> = HashMap::new();
        for diag in diagnostics {
            groups.entry(diag.file_uri.clone()).or_default().push(diag);
        }
        groups
    }

    /// Group by severity
    pub fn by_severity(diagnostics: Vec<DiagnosticInfo>) -> HashMap<DiagnosticSeverity, Vec<DiagnosticInfo>> {
        let mut groups: HashMap<DiagnosticSeverity, Vec<DiagnosticInfo>> = HashMap::new();
        for diag in diagnostics {
            groups.entry(diag.severity).or_default().push(diag);
        }
        groups
    }

    /// Group by source (linter name)
    pub fn by_source(diagnostics: Vec<DiagnosticInfo>) -> HashMap<String, Vec<DiagnosticInfo>> {
        let mut groups: HashMap<String, Vec<DiagnosticInfo>> = HashMap::new();
        for diag in diagnostics {
            let source = diag.source.clone().unwrap_or_else(|| "unknown".to_string());
            groups.entry(source).or_default().push(diag);
        }
        groups
    }

    /// Group by diagnostic code
    pub fn by_code(diagnostics: Vec<DiagnosticInfo>) -> HashMap<String, Vec<DiagnosticInfo>> {
        let mut groups: HashMap<String, Vec<DiagnosticInfo>> = HashMap::new();
        for diag in diagnostics {
            let code = diag.code.clone().unwrap_or_else(|| "no-code".to_string());
            groups.entry(code).or_default().push(diag);
        }
        groups
    }
}

/// Diagnostic filter
pub struct DiagnosticFilter {
    /// Minimum severity
    pub min_severity: Option<DiagnosticSeverity>,
    /// Include sources
    pub include_sources: Option<Vec<String>>,
    /// Exclude sources
    pub exclude_sources: Option<Vec<String>>,
    /// Include codes
    pub include_codes: Option<Vec<String>>,
    /// Exclude codes
    pub exclude_codes: Option<Vec<String>>,
}

impl DiagnosticFilter {
    pub fn new() -> Self {
        Self {
            min_severity: None,
            include_sources: None,
            exclude_sources: None,
            include_codes: None,
            exclude_codes: None,
        }
    }

    pub fn errors_only() -> Self {
        Self {
            min_severity: Some(DiagnosticSeverity::Error),
            ..Self::new()
        }
    }

    pub fn warnings_and_above() -> Self {
        Self {
            min_severity: Some(DiagnosticSeverity::Warning),
            ..Self::new()
        }
    }

    /// Apply filter to diagnostics
    pub fn apply(&self, diagnostics: Vec<DiagnosticInfo>) -> Vec<DiagnosticInfo> {
        diagnostics.into_iter().filter(|d| self.matches(d)).collect()
    }

    fn matches(&self, diag: &DiagnosticInfo) -> bool {
        // Check severity
        if let Some(min) = &self.min_severity {
            if !self.severity_at_least(diag.severity, *min) {
                return false;
            }
        }

        // Check source inclusion
        if let Some(sources) = &self.include_sources {
            if let Some(source) = &diag.source {
                if !sources.contains(source) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check source exclusion
        if let Some(sources) = &self.exclude_sources {
            if let Some(source) = &diag.source {
                if sources.contains(source) {
                    return false;
                }
            }
        }

        // Check code inclusion
        if let Some(codes) = &self.include_codes {
            if let Some(code) = &diag.code {
                if !codes.contains(code) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check code exclusion
        if let Some(codes) = &self.exclude_codes {
            if let Some(code) = &diag.code {
                if codes.contains(code) {
                    return false;
                }
            }
        }

        true
    }

    fn severity_at_least(&self, severity: DiagnosticSeverity, min: DiagnosticSeverity) -> bool {
        let severity_level = match severity {
            DiagnosticSeverity::Error => 0,
            DiagnosticSeverity::Warning => 1,
            DiagnosticSeverity::Information => 2,
            DiagnosticSeverity::Hint => 3,
        };
        let min_level = match min {
            DiagnosticSeverity::Error => 0,
            DiagnosticSeverity::Warning => 1,
            DiagnosticSeverity::Information => 2,
            DiagnosticSeverity::Hint => 3,
        };
        severity_level <= min_level
    }
}

impl Default for DiagnosticFilter {
    fn default() -> Self {
        Self::new()
    }
}
