//! Ignore rules (.gitignore, .foxkitignore, etc.)

use std::path::{Path, PathBuf};
use globset::{Glob, GlobSet, GlobSetBuilder};

/// Ignore rules
#[derive(Debug, Clone, Default)]
pub struct IgnoreRules {
    patterns: GlobSet,
    negations: GlobSet,
}

impl IgnoreRules {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load ignore rules from directory
    pub fn load(root: &Path) -> anyhow::Result<Self> {
        let mut builder = GlobSetBuilder::new();
        let mut negation_builder = GlobSetBuilder::new();

        // Always ignore some patterns
        for pattern in DEFAULT_IGNORES {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }

        // Load .gitignore
        let gitignore = root.join(".gitignore");
        if gitignore.exists() {
            Self::load_file(&gitignore, &mut builder, &mut negation_builder)?;
        }

        // Load .foxkitignore
        let foxkitignore = root.join(".foxkitignore");
        if foxkitignore.exists() {
            Self::load_file(&foxkitignore, &mut builder, &mut negation_builder)?;
        }

        // Load .ignore (ripgrep style)
        let ignore = root.join(".ignore");
        if ignore.exists() {
            Self::load_file(&ignore, &mut builder, &mut negation_builder)?;
        }

        Ok(Self {
            patterns: builder.build()?,
            negations: negation_builder.build()?,
        })
    }

    fn load_file(
        path: &Path,
        builder: &mut GlobSetBuilder,
        negation_builder: &mut GlobSetBuilder,
    ) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path)?;
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle negation
            let (is_negation, pattern) = if let Some(stripped) = line.strip_prefix('!') {
                (true, stripped)
            } else {
                (false, line)
            };

            // Convert gitignore pattern to glob
            let glob_pattern = convert_gitignore_to_glob(pattern);
            
            if let Ok(glob) = Glob::new(&glob_pattern) {
                if is_negation {
                    negation_builder.add(glob);
                } else {
                    builder.add(glob);
                }
            }
        }

        Ok(())
    }

    /// Check if path is ignored
    pub fn is_ignored(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        // Check if explicitly not ignored
        if self.negations.is_match(&*path_str) {
            return false;
        }
        
        // Check if ignored
        self.patterns.is_match(&*path_str)
    }

    /// Add a pattern
    pub fn add_pattern(&mut self, pattern: &str) -> anyhow::Result<()> {
        let mut builder = GlobSetBuilder::new();
        
        // Copy existing patterns
        // Note: GlobSet doesn't expose patterns, so we rebuild
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
        
        // This is simplified - real impl would need to track patterns
        Ok(())
    }
}

/// Convert gitignore pattern to glob pattern
fn convert_gitignore_to_glob(pattern: &str) -> String {
    let mut glob = String::new();
    let pattern = pattern.trim_end_matches('/');
    
    // If pattern doesn't start with /, it can match anywhere
    if !pattern.starts_with('/') {
        glob.push_str("**/");
    } else {
        glob.push_str(&pattern[1..]);
        return glob;
    }
    
    glob.push_str(pattern);
    
    // If pattern doesn't contain a slash, also match as directory
    if !pattern.contains('/') {
        // Pattern could be file or directory
    }
    
    glob
}

/// Default ignore patterns
const DEFAULT_IGNORES: &[&str] = &[
    "**/.git/**",
    "**/.hg/**",
    "**/.svn/**",
    "**/node_modules/**",
    "**/target/**",
    "**/.DS_Store",
    "**/Thumbs.db",
    "**/*.pyc",
    "**/__pycache__/**",
    "**/.idea/**",
    "**/.vscode/**",
    "**/dist/**",
    "**/build/**",
    "**/.cache/**",
    "**/*.log",
];
