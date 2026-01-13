//! Code actions

use std::path::PathBuf;
use async_trait::async_trait;

use crate::{WorkspaceEdit, Range};

/// Code action
#[derive(Debug, Clone)]
pub struct CodeAction {
    /// Action title
    pub title: String,
    /// Action kind
    pub kind: CodeActionKind,
    /// Preferred action (shown prominently)
    pub is_preferred: bool,
    /// Edit to apply
    pub edit: Option<WorkspaceEdit>,
    /// Command to run
    pub command: Option<ActionCommand>,
    /// Diagnostics this action fixes
    pub diagnostics: Vec<String>,
    /// Additional data
    pub data: Option<serde_json::Value>,
}

impl CodeAction {
    /// Create a new code action
    pub fn new(title: impl Into<String>, kind: CodeActionKind) -> Self {
        Self {
            title: title.into(),
            kind,
            is_preferred: false,
            edit: None,
            command: None,
            diagnostics: Vec::new(),
            data: None,
        }
    }

    /// Set as preferred
    pub fn preferred(mut self) -> Self {
        self.is_preferred = true;
        self
    }

    /// Set edit
    pub fn with_edit(mut self, edit: WorkspaceEdit) -> Self {
        self.edit = Some(edit);
        self
    }

    /// Get priority for sorting
    pub fn priority(&self) -> u32 {
        if self.is_preferred { 0 }
        else {
            match &self.kind {
                CodeActionKind::QuickFix => 1,
                CodeActionKind::Refactor => 2,
                CodeActionKind::RefactorExtract => 2,
                CodeActionKind::RefactorInline => 2,
                CodeActionKind::RefactorRewrite => 2,
                CodeActionKind::Source => 3,
                CodeActionKind::SourceOrganizeImports => 3,
                CodeActionKind::Custom(_) => 4,
            }
        }
    }
}

/// Code action kind
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeActionKind {
    QuickFix,
    Refactor,
    RefactorExtract,
    RefactorInline,
    RefactorRewrite,
    Source,
    SourceOrganizeImports,
    Custom(String),
}

impl CodeActionKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::QuickFix => "quickfix",
            Self::Refactor => "refactor",
            Self::RefactorExtract => "refactor.extract",
            Self::RefactorInline => "refactor.inline",
            Self::RefactorRewrite => "refactor.rewrite",
            Self::Source => "source",
            Self::SourceOrganizeImports => "source.organizeImports",
            Self::Custom(s) => s,
        }
    }
}

/// Command to execute
#[derive(Debug, Clone)]
pub struct ActionCommand {
    pub id: String,
    pub title: String,
    pub arguments: Vec<serde_json::Value>,
}

/// Code action provider trait
#[async_trait]
pub trait CodeActionProvider: Send + Sync {
    /// Get actions for position
    async fn provide_actions(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        source: &str,
    ) -> anyhow::Result<Vec<CodeAction>>;

    /// Can handle action kind?
    fn can_handle(&self, kind: &CodeActionKind) -> bool;

    /// Execute action
    async fn execute(&self, action: &CodeAction) -> anyhow::Result<WorkspaceEdit>;
}
