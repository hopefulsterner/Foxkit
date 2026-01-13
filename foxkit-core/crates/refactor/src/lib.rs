//! # Foxkit Refactor
//!
//! Code refactoring engine with rename, extract, and more.

pub mod actions;
pub mod edit;
pub mod rename;
pub mod extract;
pub mod inline;

use std::path::PathBuf;
use std::collections::HashMap;

pub use actions::{CodeAction, CodeActionKind, CodeActionProvider};
pub use edit::{TextEdit, WorkspaceEdit, FileEdit};
pub use rename::{RenameProvider, RenameResult};
pub use extract::{ExtractProvider, ExtractResult};
pub use inline::InlineProvider;

/// Refactoring service
pub struct RefactorService {
    /// Registered action providers
    providers: Vec<Box<dyn CodeActionProvider>>,
}

impl RefactorService {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register an action provider
    pub fn register<P: CodeActionProvider + 'static>(&mut self, provider: P) {
        self.providers.push(Box::new(provider));
    }

    /// Get available actions at position
    pub async fn actions_at(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        source: &str,
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        for provider in &self.providers {
            if let Ok(mut provided) = provider.provide_actions(file, start, end, source).await {
                actions.append(&mut provided);
            }
        }

        // Sort by priority
        actions.sort_by_key(|a| a.priority());
        actions
    }

    /// Execute a refactoring action
    pub async fn execute(&self, action: &CodeAction) -> anyhow::Result<WorkspaceEdit> {
        // Find provider for action
        for provider in &self.providers {
            if provider.can_handle(&action.kind) {
                return provider.execute(action).await;
            }
        }

        anyhow::bail!("No provider for action: {:?}", action.kind)
    }
}

impl Default for RefactorService {
    fn default() -> Self {
        let mut service = Self::new();
        
        // Register built-in providers
        service.register(rename::BuiltinRenameProvider);
        service.register(extract::BuiltinExtractProvider);
        service.register(inline::BuiltinInlineProvider);
        
        service
    }
}

/// Position in source
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Range in source
#[derive(Debug, Clone, Copy)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn point(line: usize, column: usize) -> Self {
        Self {
            start: Position { line, column },
            end: Position { line, column },
        }
    }

    pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start: Position { line: start_line, column: start_col },
            end: Position { line: end_line, column: end_col },
        }
    }
}
