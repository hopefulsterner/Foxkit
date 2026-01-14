//! Code Actions for AI Chat
//!
//! Context-aware AI actions that can be triggered from the editor,
//! like "Explain this", "Fix error", "Generate tests", etc.

use serde::{Deserialize, Serialize};

/// A code action that can be triggered by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAction {
    /// Unique identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Kind of action
    pub kind: CodeActionKind,
    /// When this action should be shown
    pub trigger: ActionTrigger,
    /// Icon for the action
    pub icon: Option<String>,
    /// Keyboard shortcut
    pub keybinding: Option<String>,
}

/// Kind of code action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeActionKind {
    /// Quick fix for an error
    QuickFix,
    /// Refactoring suggestion
    Refactor,
    /// Code generation
    Generate,
    /// Documentation generation
    Document,
    /// Explanation/learning
    Explain,
    /// Test generation
    Test,
    /// Performance optimization
    Optimize,
    /// Security improvement
    Security,
    /// Generic AI action
    Ai,
}

/// When to show/trigger the action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionTrigger {
    /// Always available
    Always,
    /// When there's a selection
    OnSelection,
    /// When there's an error at cursor
    OnError,
    /// When there's a warning
    OnWarning,
    /// On specific symbol types
    OnSymbol(Vec<SymbolKind>),
    /// On specific language
    OnLanguage(Vec<String>),
    /// Custom predicate
    Custom(String),
}

/// Symbol kinds for triggering actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Struct,
    Enum,
    Variable,
    Constant,
    Import,
    Module,
}

/// Registry of available code actions
pub struct CodeActionRegistry {
    actions: Vec<CodeAction>,
}

impl CodeActionRegistry {
    pub fn new() -> Self {
        let mut registry = Self { actions: vec![] };
        registry.register_builtin_actions();
        registry
    }

    fn register_builtin_actions(&mut self) {
        // Quick Fix Actions
        self.register(CodeAction {
            id: "ai.fix.error".into(),
            title: "Fix with AI".into(),
            description: "Use AI to automatically fix this error".into(),
            kind: CodeActionKind::QuickFix,
            trigger: ActionTrigger::OnError,
            icon: Some("sparkle".into()),
            keybinding: Some("Ctrl+Shift+.".into()),
        });

        self.register(CodeAction {
            id: "ai.fix.all".into(),
            title: "Fix all errors in file".into(),
            description: "Use AI to fix all errors in the current file".into(),
            kind: CodeActionKind::QuickFix,
            trigger: ActionTrigger::OnError,
            icon: Some("sparkle".into()),
            keybinding: None,
        });

        // Explain Actions
        self.register(CodeAction {
            id: "ai.explain.selection".into(),
            title: "Explain Selection".into(),
            description: "Get an AI explanation of the selected code".into(),
            kind: CodeActionKind::Explain,
            trigger: ActionTrigger::OnSelection,
            icon: Some("question".into()),
            keybinding: Some("Ctrl+Shift+E".into()),
        });

        self.register(CodeAction {
            id: "ai.explain.function".into(),
            title: "Explain Function".into(),
            description: "Get an AI explanation of this function".into(),
            kind: CodeActionKind::Explain,
            trigger: ActionTrigger::OnSymbol(vec![SymbolKind::Function, SymbolKind::Method]),
            icon: Some("question".into()),
            keybinding: None,
        });

        self.register(CodeAction {
            id: "ai.explain.error".into(),
            title: "Explain Error".into(),
            description: "Get an AI explanation of this error and how to fix it".into(),
            kind: CodeActionKind::Explain,
            trigger: ActionTrigger::OnError,
            icon: Some("question".into()),
            keybinding: None,
        });

        // Generate Actions
        self.register(CodeAction {
            id: "ai.generate.implementation".into(),
            title: "Generate Implementation".into(),
            description: "AI generates an implementation based on the signature/interface".into(),
            kind: CodeActionKind::Generate,
            trigger: ActionTrigger::OnSymbol(vec![SymbolKind::Function, SymbolKind::Method]),
            icon: Some("sparkle".into()),
            keybinding: None,
        });

        self.register(CodeAction {
            id: "ai.generate.types".into(),
            title: "Generate Types".into(),
            description: "AI generates TypeScript/type definitions for this code".into(),
            kind: CodeActionKind::Generate,
            trigger: ActionTrigger::OnSelection,
            icon: Some("symbol-interface".into()),
            keybinding: None,
        });

        // Documentation Actions
        self.register(CodeAction {
            id: "ai.doc.generate".into(),
            title: "Generate Documentation".into(),
            description: "AI generates documentation comments for this code".into(),
            kind: CodeActionKind::Document,
            trigger: ActionTrigger::OnSymbol(vec![
                SymbolKind::Function,
                SymbolKind::Method,
                SymbolKind::Class,
                SymbolKind::Interface,
            ]),
            icon: Some("book".into()),
            keybinding: Some("Ctrl+Shift+D".into()),
        });

        self.register(CodeAction {
            id: "ai.doc.readme".into(),
            title: "Generate README".into(),
            description: "AI generates a README for this module/package".into(),
            kind: CodeActionKind::Document,
            trigger: ActionTrigger::Always,
            icon: Some("markdown".into()),
            keybinding: None,
        });

        // Test Actions
        self.register(CodeAction {
            id: "ai.test.generate".into(),
            title: "Generate Unit Tests".into(),
            description: "AI generates unit tests for this function".into(),
            kind: CodeActionKind::Test,
            trigger: ActionTrigger::OnSymbol(vec![SymbolKind::Function, SymbolKind::Method]),
            icon: Some("beaker".into()),
            keybinding: Some("Ctrl+Shift+T".into()),
        });

        self.register(CodeAction {
            id: "ai.test.edge".into(),
            title: "Generate Edge Case Tests".into(),
            description: "AI generates tests for edge cases and error conditions".into(),
            kind: CodeActionKind::Test,
            trigger: ActionTrigger::OnSymbol(vec![SymbolKind::Function, SymbolKind::Method]),
            icon: Some("beaker".into()),
            keybinding: None,
        });

        // Refactor Actions
        self.register(CodeAction {
            id: "ai.refactor.improve".into(),
            title: "Improve Code".into(),
            description: "AI suggests improvements for code quality and readability".into(),
            kind: CodeActionKind::Refactor,
            trigger: ActionTrigger::OnSelection,
            icon: Some("wrench".into()),
            keybinding: None,
        });

        self.register(CodeAction {
            id: "ai.refactor.extract".into(),
            title: "Extract to Function".into(),
            description: "AI extracts selection to a new function with appropriate name".into(),
            kind: CodeActionKind::Refactor,
            trigger: ActionTrigger::OnSelection,
            icon: Some("symbol-method".into()),
            keybinding: None,
        });

        self.register(CodeAction {
            id: "ai.refactor.rename".into(),
            title: "Suggest Better Name".into(),
            description: "AI suggests a more descriptive name for this symbol".into(),
            kind: CodeActionKind::Refactor,
            trigger: ActionTrigger::OnSymbol(vec![
                SymbolKind::Function,
                SymbolKind::Variable,
                SymbolKind::Class,
            ]),
            icon: Some("edit".into()),
            keybinding: None,
        });

        self.register(CodeAction {
            id: "ai.refactor.simplify".into(),
            title: "Simplify Code".into(),
            description: "AI simplifies complex code while preserving behavior".into(),
            kind: CodeActionKind::Refactor,
            trigger: ActionTrigger::OnSelection,
            icon: Some("minimize".into()),
            keybinding: None,
        });

        // Optimize Actions
        self.register(CodeAction {
            id: "ai.optimize.performance".into(),
            title: "Optimize Performance".into(),
            description: "AI suggests performance optimizations".into(),
            kind: CodeActionKind::Optimize,
            trigger: ActionTrigger::OnSelection,
            icon: Some("zap".into()),
            keybinding: None,
        });

        self.register(CodeAction {
            id: "ai.optimize.memory".into(),
            title: "Optimize Memory Usage".into(),
            description: "AI suggests memory optimizations".into(),
            kind: CodeActionKind::Optimize,
            trigger: ActionTrigger::OnSelection,
            icon: Some("circuit-board".into()),
            keybinding: None,
        });

        // Security Actions
        self.register(CodeAction {
            id: "ai.security.audit".into(),
            title: "Security Audit".into(),
            description: "AI checks for security vulnerabilities".into(),
            kind: CodeActionKind::Security,
            trigger: ActionTrigger::OnSelection,
            icon: Some("shield".into()),
            keybinding: None,
        });

        self.register(CodeAction {
            id: "ai.security.fix".into(),
            title: "Fix Security Issue".into(),
            description: "AI fixes the identified security vulnerability".into(),
            kind: CodeActionKind::Security,
            trigger: ActionTrigger::OnWarning, // Security warnings
            icon: Some("shield-check".into()),
            keybinding: None,
        });

        // Convert Actions
        self.register(CodeAction {
            id: "ai.convert.async".into(),
            title: "Convert to Async".into(),
            description: "AI converts synchronous code to async/await".into(),
            kind: CodeActionKind::Refactor,
            trigger: ActionTrigger::OnSymbol(vec![SymbolKind::Function, SymbolKind::Method]),
            icon: Some("sync".into()),
            keybinding: None,
        });

        self.register(CodeAction {
            id: "ai.convert.language".into(),
            title: "Convert to Another Language".into(),
            description: "AI converts code to a different programming language".into(),
            kind: CodeActionKind::Generate,
            trigger: ActionTrigger::OnSelection,
            icon: Some("replace".into()),
            keybinding: None,
        });
    }

    /// Register a code action
    pub fn register(&mut self, action: CodeAction) {
        self.actions.push(action);
    }

    /// Get all actions
    pub fn all(&self) -> &[CodeAction] {
        &self.actions
    }

    /// Get actions by kind
    pub fn by_kind(&self, kind: CodeActionKind) -> Vec<&CodeAction> {
        self.actions.iter().filter(|a| a.kind == kind).collect()
    }

    /// Get actions available for a given context
    pub fn available_for(&self, context: &ActionContext) -> Vec<&CodeAction> {
        self.actions
            .iter()
            .filter(|action| self.matches_trigger(&action.trigger, context))
            .collect()
    }

    fn matches_trigger(&self, trigger: &ActionTrigger, context: &ActionContext) -> bool {
        match trigger {
            ActionTrigger::Always => true,
            ActionTrigger::OnSelection => context.has_selection,
            ActionTrigger::OnError => context.has_error,
            ActionTrigger::OnWarning => context.has_warning,
            ActionTrigger::OnSymbol(kinds) => {
                context.symbol_kind.map(|k| kinds.contains(&k)).unwrap_or(false)
            }
            ActionTrigger::OnLanguage(languages) => {
                context.language.as_ref().map(|l| languages.contains(l)).unwrap_or(false)
            }
            ActionTrigger::Custom(_) => true, // Would need custom evaluation
        }
    }

    /// Find action by ID
    pub fn find(&self, id: &str) -> Option<&CodeAction> {
        self.actions.iter().find(|a| a.id == id)
    }
}

impl Default for CodeActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for determining available actions
#[derive(Debug, Clone, Default)]
pub struct ActionContext {
    pub has_selection: bool,
    pub has_error: bool,
    pub has_warning: bool,
    pub symbol_kind: Option<SymbolKind>,
    pub language: Option<String>,
    pub file_path: Option<String>,
}

/// Result of executing a code action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeActionResult {
    pub action_id: String,
    pub success: bool,
    pub message: Option<String>,
    pub edits: Vec<TextEdit>,
    pub show_in_chat: bool,
}

/// A text edit to apply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub file_path: String,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub new_text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = CodeActionRegistry::new();
        assert!(!registry.all().is_empty());
    }

    #[test]
    fn test_available_on_selection() {
        let registry = CodeActionRegistry::new();
        let context = ActionContext {
            has_selection: true,
            ..Default::default()
        };
        let available = registry.available_for(&context);
        assert!(available.iter().any(|a| a.id == "ai.explain.selection"));
    }

    #[test]
    fn test_available_on_error() {
        let registry = CodeActionRegistry::new();
        let context = ActionContext {
            has_error: true,
            ..Default::default()
        };
        let available = registry.available_for(&context);
        assert!(available.iter().any(|a| a.id == "ai.fix.error"));
    }

    #[test]
    fn test_by_kind() {
        let registry = CodeActionRegistry::new();
        let quick_fixes = registry.by_kind(CodeActionKind::QuickFix);
        assert!(!quick_fixes.is_empty());
    }
}
