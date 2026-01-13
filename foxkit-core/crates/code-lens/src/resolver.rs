//! Code lens resolver

use crate::{CodeLens, CodeLensCommand};

/// Code lens resolver for lazy loading
pub struct CodeLensResolver {
    // Would hold LSP client for resolution
}

impl CodeLensResolver {
    pub fn new() -> Self {
        Self {}
    }

    /// Resolve a code lens
    pub async fn resolve(&self, lens: &CodeLens) -> anyhow::Result<CodeLens> {
        // If already resolved, return as-is
        if lens.is_resolved() {
            return Ok(lens.clone());
        }

        let data = lens.data.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No data to resolve"))?;

        let lens_type = data.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown");

        match lens_type {
            "references" => self.resolve_references(lens).await,
            "implementations" => self.resolve_implementations(lens).await,
            _ => Ok(lens.clone()),
        }
    }

    /// Resolve reference count
    async fn resolve_references(&self, lens: &CodeLens) -> anyhow::Result<CodeLens> {
        // Would query LSP for references
        // For now, return placeholder
        let count = 0; // Would be real reference count
        
        let command = CodeLensCommand {
            title: format!("{} references", count),
            command: "foxkit.showReferences".to_string(),
            arguments: vec![lens.data.clone().unwrap_or_default()],
        };

        Ok(CodeLens {
            range: lens.range,
            command: Some(command),
            data: lens.data.clone(),
        })
    }

    /// Resolve implementation count
    async fn resolve_implementations(&self, lens: &CodeLens) -> anyhow::Result<CodeLens> {
        // Would query LSP for implementations
        let count = 0; // Would be real implementation count
        
        let command = CodeLensCommand {
            title: format!("{} implementations", count),
            command: "foxkit.showImplementations".to_string(),
            arguments: vec![lens.data.clone().unwrap_or_default()],
        };

        Ok(CodeLens {
            range: lens.range,
            command: Some(command),
            data: lens.data.clone(),
        })
    }
}

impl Default for CodeLensResolver {
    fn default() -> Self {
        Self::new()
    }
}
