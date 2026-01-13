//! # Foxkit Code Actions
//!
//! Quick fixes, refactorings, and source actions.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Code actions service
pub struct CodeActionsService {
    /// Registered providers
    providers: RwLock<Vec<Arc<dyn CodeActionProvider>>>,
    /// Events
    events: broadcast::Sender<CodeActionsEvent>,
    /// Configuration
    config: RwLock<CodeActionsConfig>,
}

impl CodeActionsService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            providers: RwLock::new(Vec::new()),
            events,
            config: RwLock::new(CodeActionsConfig::default()),
        }
    }

    /// Register provider
    pub fn register_provider(&self, provider: Arc<dyn CodeActionProvider>) {
        self.providers.write().push(provider);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<CodeActionsEvent> {
        self.events.subscribe()
    }

    /// Configure code actions
    pub fn configure(&self, config: CodeActionsConfig) {
        *self.config.write() = config;
    }

    /// Get code actions for range
    pub async fn get_code_actions(
        &self,
        file: &PathBuf,
        range: CodeActionRange,
        context: CodeActionContext,
    ) -> Vec<CodeAction> {
        let providers = self.providers.read().clone();
        let config = self.config.read().clone();
        let mut actions = Vec::new();

        for provider in providers {
            match provider.provide_actions(file, &range, &context).await {
                Ok(provider_actions) => {
                    for action in provider_actions {
                        // Filter by kind if configured
                        if config.enabled_kinds.is_empty() 
                            || config.enabled_kinds.contains(&action.kind)
                        {
                            actions.push(action);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Code action provider failed: {}", e);
                }
            }
        }

        // Sort by kind priority
        actions.sort_by(|a, b| {
            let a_priority = a.kind.priority();
            let b_priority = b.kind.priority();
            a_priority.cmp(&b_priority)
        });

        actions
    }

    /// Execute code action
    pub async fn execute(&self, action: &CodeAction) -> anyhow::Result<()> {
        let _ = self.events.send(CodeActionsEvent::Executing {
            title: action.title.clone(),
        });

        // Would apply workspace edit or execute command
        if let Some(ref edit) = action.edit {
            // Apply edit
        }

        if let Some(ref command) = action.command {
            // Execute command
        }

        let _ = self.events.send(CodeActionsEvent::Executed {
            title: action.title.clone(),
        });

        Ok(())
    }

    /// Get preferred action (for auto-fix)
    pub async fn get_preferred_action(
        &self,
        file: &PathBuf,
        range: CodeActionRange,
        context: CodeActionContext,
    ) -> Option<CodeAction> {
        let actions = self.get_code_actions(file, range, context).await;
        actions.into_iter().find(|a| a.is_preferred)
    }
}

impl Default for CodeActionsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Code action provider trait
#[async_trait::async_trait]
pub trait CodeActionProvider: Send + Sync {
    /// Provider ID
    fn id(&self) -> &str;

    /// Provide code actions
    async fn provide_actions(
        &self,
        file: &PathBuf,
        range: &CodeActionRange,
        context: &CodeActionContext,
    ) -> anyhow::Result<Vec<CodeAction>>;

    /// Resolve code action (lazy loading)
    async fn resolve(&self, action: &CodeAction) -> anyhow::Result<CodeAction> {
        Ok(action.clone())
    }
}

/// Code action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAction {
    /// Display title
    pub title: String,
    /// Kind
    pub kind: CodeActionKind,
    /// Diagnostics this action addresses
    pub diagnostics: Vec<ActionDiagnostic>,
    /// Is preferred action
    pub is_preferred: bool,
    /// Is disabled
    pub disabled: Option<DisabledReason>,
    /// Workspace edit
    pub edit: Option<WorkspaceEdit>,
    /// Command to execute
    pub command: Option<ActionCommand>,
    /// Data for resolution
    #[serde(skip)]
    pub data: Option<serde_json::Value>,
}

impl CodeAction {
    pub fn new(title: impl Into<String>, kind: CodeActionKind) -> Self {
        Self {
            title: title.into(),
            kind,
            diagnostics: Vec::new(),
            is_preferred: false,
            disabled: None,
            edit: None,
            command: None,
            data: None,
        }
    }

    pub fn quick_fix(title: impl Into<String>) -> Self {
        Self::new(title, CodeActionKind::QuickFix)
    }

    pub fn refactor(title: impl Into<String>) -> Self {
        Self::new(title, CodeActionKind::Refactor)
    }

    pub fn source(title: impl Into<String>) -> Self {
        Self::new(title, CodeActionKind::Source)
    }

    pub fn with_edit(mut self, edit: WorkspaceEdit) -> Self {
        self.edit = Some(edit);
        self
    }

    pub fn with_command(mut self, command: ActionCommand) -> Self {
        self.command = Some(command);
        self
    }

    pub fn preferred(mut self) -> Self {
        self.is_preferred = true;
        self
    }

    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.disabled = Some(DisabledReason { reason: reason.into() });
        self
    }
}

/// Code action kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodeActionKind {
    /// Quick fix for diagnostics
    QuickFix,
    /// General refactoring
    Refactor,
    /// Extract refactoring
    RefactorExtract,
    /// Inline refactoring
    RefactorInline,
    /// Rewrite refactoring
    RefactorRewrite,
    /// Source action
    Source,
    /// Organize imports
    SourceOrganizeImports,
    /// Fix all
    SourceFixAll,
}

impl CodeActionKind {
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

    pub fn priority(&self) -> u32 {
        match self {
            Self::QuickFix => 0,
            Self::RefactorExtract => 1,
            Self::RefactorInline => 2,
            Self::RefactorRewrite => 3,
            Self::Refactor => 4,
            Self::SourceOrganizeImports => 5,
            Self::SourceFixAll => 6,
            Self::Source => 7,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::QuickFix => "$(lightbulb)",
            Self::Refactor | Self::RefactorExtract | Self::RefactorInline | Self::RefactorRewrite => "$(edit)",
            Self::Source | Self::SourceOrganizeImports | Self::SourceFixAll => "$(wand)",
        }
    }
}

/// Code action range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeActionRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl CodeActionRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn point(line: u32, col: u32) -> Self {
        Self { start_line: line, start_col: col, end_line: line, end_col: col }
    }
}

/// Code action context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeActionContext {
    /// Diagnostics at range
    pub diagnostics: Vec<ActionDiagnostic>,
    /// Requested kinds
    pub only: Vec<CodeActionKind>,
    /// Trigger kind
    pub trigger_kind: CodeActionTrigger,
}

impl CodeActionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_diagnostics(mut self, diagnostics: Vec<ActionDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn with_only(mut self, kinds: Vec<CodeActionKind>) -> Self {
        self.only = kinds;
        self
    }
}

/// Code action trigger
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum CodeActionTrigger {
    /// Invoked manually
    #[default]
    Invoked,
    /// Automatic (e.g., on save)
    Automatic,
}

/// Action diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDiagnostic {
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub range: CodeActionRange,
    pub code: Option<String>,
    pub source: Option<String>,
}

/// Diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Workspace edit
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    pub changes: HashMap<PathBuf, Vec<TextEdit>>,
}

impl WorkspaceEdit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_edit(&mut self, file: PathBuf, edit: TextEdit) {
        self.changes.entry(file).or_default().push(edit);
    }
}

/// Text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: CodeActionRange,
    pub new_text: String,
}

/// Action command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCommand {
    pub title: String,
    pub command: String,
    pub arguments: Vec<serde_json::Value>,
}

impl ActionCommand {
    pub fn new(title: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            command: command.into(),
            arguments: Vec::new(),
        }
    }

    pub fn with_args(mut self, args: Vec<serde_json::Value>) -> Self {
        self.arguments = args;
        self
    }
}

/// Disabled reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisabledReason {
    pub reason: String,
}

/// Code actions configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeActionsConfig {
    /// Enabled action kinds
    pub enabled_kinds: Vec<CodeActionKind>,
    /// Auto-fix on save
    pub fix_on_save: bool,
    /// Organize imports on save
    pub organize_imports_on_save: bool,
}

impl Default for CodeActionsConfig {
    fn default() -> Self {
        Self {
            enabled_kinds: Vec::new(), // All kinds
            fix_on_save: false,
            organize_imports_on_save: false,
        }
    }
}

/// Code actions event
#[derive(Debug, Clone)]
pub enum CodeActionsEvent {
    Executing { title: String },
    Executed { title: String },
    Failed { title: String, error: String },
}

/// Lightbulb widget state
pub struct LightbulbWidget {
    /// Is visible
    visible: bool,
    /// Position
    position: (u32, u32),
    /// Available actions
    actions: Vec<CodeAction>,
    /// Has quick fix
    has_quick_fix: bool,
}

impl LightbulbWidget {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (0, 0),
            actions: Vec::new(),
            has_quick_fix: false,
        }
    }

    pub fn show(&mut self, position: (u32, u32), actions: Vec<CodeAction>) {
        self.position = position;
        self.has_quick_fix = actions.iter().any(|a| a.kind == CodeActionKind::QuickFix);
        self.actions = actions;
        self.visible = !self.actions.is_empty();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.actions.clear();
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn position(&self) -> (u32, u32) {
        self.position
    }

    pub fn actions(&self) -> &[CodeAction] {
        &self.actions
    }

    pub fn has_quick_fix(&self) -> bool {
        self.has_quick_fix
    }

    pub fn icon(&self) -> &'static str {
        if self.has_quick_fix {
            "$(lightbulb-autofix)"
        } else {
            "$(lightbulb)"
        }
    }
}

impl Default for LightbulbWidget {
    fn default() -> Self {
        Self::new()
    }
}
