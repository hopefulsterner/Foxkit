//! Monorepo-aware context building
//! 
//! This is what makes Foxkit's AI special - it understands the ENTIRE codebase

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::Result;
use monorepo::{self, Package};

/// AI Context builder - builds rich context for AI from the monorepo
pub struct AiContext {
    /// Workspace root
    workspace_root: Option<PathBuf>,
    /// Current file being edited
    current_file: Option<PathBuf>,
    /// Selected/highlighted code
    selection: Option<Selection>,
    /// Related files (imports, dependents)
    related_files: Vec<PathBuf>,
    /// Package context
    package_context: Option<PackageContext>,
    /// Custom context entries
    custom: HashMap<String, String>,
}

/// Code selection
#[derive(Debug, Clone)]
pub struct Selection {
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
}

/// Package-level context
#[derive(Debug, Clone)]
pub struct PackageContext {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub readme: Option<String>,
}

impl From<Package> for PackageContext {
    fn from(pkg: Package) -> Self {
        Self {
            name: pkg.name,
            path: pkg.path,
            dependencies: pkg.dependencies,
            dependents: Vec::new(), // TODO: Get this from DependencyGraph
            readme: None,           // Loaded separately if needed
        }
    }
}

impl AiContext {
    pub fn new() -> Self {
        Self {
            workspace_root: None,
            current_file: None,
            selection: None,
            related_files: Vec::new(),
            package_context: None,
            custom: HashMap::new(),
        }
    }

    /// Set workspace root
    pub fn with_workspace(mut self, root: impl AsRef<Path>) -> Self {
        self.workspace_root = Some(root.as_ref().to_path_buf());
        self
    }

    /// Set current file
    pub fn with_file(mut self, file: impl AsRef<Path>) -> Self {
        self.current_file = Some(file.as_ref().to_path_buf());
        self
    }

    /// Set selection
    pub fn with_selection(mut self, selection: Selection) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Add related files
    pub fn with_related_files(mut self, files: Vec<PathBuf>) -> Self {
        self.related_files = files;
        self
    }

    /// Set package context
    pub fn with_package(mut self, ctx: PackageContext) -> Self {
        self.package_context = Some(ctx);
        self
    }

    /// Add custom context
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// Build context string for the AI
    pub fn build(&self) -> String {
        let mut parts = Vec::new();

        // Workspace info
        if let Some(root) = &self.workspace_root {
            parts.push(format!("Workspace: {}", root.display()));
        }

        // Package context
        if let Some(pkg) = &self.package_context {
            parts.push(format!("\n## Package: {}", pkg.name));
            parts.push(format!("Path: {}", pkg.path.display()));
            
            if !pkg.dependencies.is_empty() {
                parts.push(format!("Dependencies: {}", pkg.dependencies.join(", ")));
            }
            if !pkg.dependents.is_empty() {
                parts.push(format!("Used by: {}", pkg.dependents.join(", ")));
            }
            if let Some(readme) = &pkg.readme {
                parts.push(format!("\n### README:\n{}", readme));
            }
        }

        // Current file
        if let Some(file) = &self.current_file {
            parts.push(format!("\n## Current File: {}", file.display()));
        }

        // Selection
        if let Some(sel) = &self.selection {
            parts.push(format!(
                "\n## Selection (lines {}-{}):\n```\n{}\n```",
                sel.start_line, sel.end_line, sel.text
            ));
        }

        // Related files
        if !self.related_files.is_empty() {
            parts.push("\n## Related Files:".to_string());
            for file in &self.related_files {
                parts.push(format!("- {}", file.display()));
            }
        }

        // Custom context
        for (key, value) in &self.custom {
            parts.push(format!("\n## {}:\n{}", key, value));
        }

        parts.join("\n")
    }

    /// Estimate token count for context
    pub fn estimate_tokens(&self) -> usize {
        let text = self.build();
        // Rough estimate: ~4 chars per token
        text.len() / 4
    }
}

impl Default for AiContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Build context for a specific task
pub struct ContextBuilder {
    max_tokens: usize,
    include_dependencies: bool,
    include_tests: bool,
    include_docs: bool,
}

impl ContextBuilder {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            include_dependencies: true,
            include_tests: false,
            include_docs: true,
        }
    }

    pub fn with_dependencies(mut self, include: bool) -> Self {
        self.include_dependencies = include;
        self
    }

    pub fn with_tests(mut self, include: bool) -> Self {
        self.include_tests = include;
        self
    }

    pub fn with_docs(mut self, include: bool) -> Self {
        self.include_docs = include;
        self
    }

    /// Build optimized context that fits within token limit
    pub async fn build_for_file(&self, file: &Path) -> Result<AiContext> {
        let mut context = AiContext::new().with_file(file);
        
        // Read current file
        let content = tokio::fs::read_to_string(file).await?;
        context = context.with_custom("File Content", content);
        
        // --- Smart Monorepo-Aware Context Gathering ---
        
        // 1. Detect package for the current file
        if let Some(pkg) = monorepo::find_package_for_path(file).await? {
            let mut pkg_ctx: PackageContext = pkg.clone().into();
            
            // 2. Load README if it exists in the package root
            let readme_path = pkg.path.join("README.md");
            if readme_path.exists() {
                if let Ok(readme_content) = tokio::fs::read_to_string(readme_path).await {
                    pkg_ctx.readme = Some(readme_content);
                }
            }
            
            context = context.with_package(pkg_ctx);
            
            // 3. Include related files (e.g., entry points)
            context = context.with_related_files(pkg.entry_points);
        }
        
        // TODO: Further enhancements
        // - Find imports and include relevant parts
        // - Find tests if requested
        // - Include type definitions
        // - Prune to fit max_tokens
        
        Ok(context)
    }
}
