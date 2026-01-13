//! # Foxkit Code Lens
//!
//! Inline code annotations and actions.

pub mod provider;
pub mod resolver;
pub mod commands;

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use provider::{CodeLensProvider, BuiltinProviders};
pub use resolver::CodeLensResolver;
pub use commands::CodeLensCommand;

/// Code lens service
pub struct CodeLensService {
    /// Registered providers
    providers: RwLock<Vec<Arc<dyn CodeLensProvider>>>,
    /// Cached lenses by file
    cache: RwLock<HashMap<PathBuf, Vec<CodeLens>>>,
    /// Resolver
    resolver: CodeLensResolver,
}

impl CodeLensService {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
            cache: RwLock::new(HashMap::new()),
            resolver: CodeLensResolver::new(),
        }
    }

    /// Register a provider
    pub fn register<P: CodeLensProvider + 'static>(&self, provider: P) {
        self.providers.write().push(Arc::new(provider));
    }

    /// Get code lenses for file
    pub async fn get_lenses(&self, file: &PathBuf, content: &str) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        for provider in self.providers.read().iter() {
            if let Ok(mut provided) = provider.provide_lenses(file, content).await {
                lenses.append(&mut provided);
            }
        }

        // Sort by line
        lenses.sort_by_key(|l| l.range.start.line);

        // Cache
        self.cache.write().insert(file.clone(), lenses.clone());

        lenses
    }

    /// Resolve a code lens (lazy loading)
    pub async fn resolve(&self, lens: &CodeLens) -> anyhow::Result<CodeLens> {
        self.resolver.resolve(lens).await
    }

    /// Execute code lens command
    pub async fn execute(&self, command: &CodeLensCommand) -> anyhow::Result<()> {
        command.execute().await
    }

    /// Invalidate cache for file
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
    }

    /// Clear all cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

impl Default for CodeLensService {
    fn default() -> Self {
        let service = Self::new();
        
        // Register built-in providers
        for provider in BuiltinProviders::all() {
            service.providers.write().push(provider);
        }
        
        service
    }
}

/// Code lens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLens {
    /// Range where the lens applies
    pub range: Range,
    /// Command to execute (may be None if needs resolution)
    pub command: Option<CodeLensCommand>,
    /// Data for lazy resolution
    pub data: Option<serde_json::Value>,
}

impl CodeLens {
    pub fn new(range: Range) -> Self {
        Self {
            range,
            command: None,
            data: None,
        }
    }

    pub fn with_command(mut self, command: CodeLensCommand) -> Self {
        self.command = Some(command);
        self
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Is this lens resolved?
    pub fn is_resolved(&self) -> bool {
        self.command.is_some()
    }
}

/// Range in document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            start: Position { line: start_line, character: start_col },
            end: Position { line: end_line, character: end_col },
        }
    }

    pub fn line(line: u32) -> Self {
        Self::new(line, 0, line, 0)
    }
}

/// Position in document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Reference count lens
#[derive(Debug, Clone)]
pub struct ReferenceCount {
    pub count: usize,
    pub locations: Vec<ReferenceLocation>,
}

/// Reference location
#[derive(Debug, Clone)]
pub struct ReferenceLocation {
    pub path: PathBuf,
    pub line: u32,
    pub column: u32,
}

/// Test lens data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestLensData {
    pub test_name: String,
    pub test_file: PathBuf,
    pub kind: TestKind,
}

/// Test kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TestKind {
    Unit,
    Integration,
    Benchmark,
    DocTest,
}

/// Implementation lens data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationLensData {
    pub symbol: String,
    pub implementations: Vec<ImplementationInfo>,
}

/// Implementation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationInfo {
    pub name: String,
    pub path: PathBuf,
    pub line: u32,
}
