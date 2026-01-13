//! Workspace-level intelligence

use std::path::Path;
use anyhow::Result;

/// Workspace type detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceType {
    /// Nx monorepo
    Nx,
    /// Turborepo
    Turborepo,
    /// Lerna
    Lerna,
    /// Yarn workspaces
    YarnWorkspaces,
    /// Pnpm workspaces
    PnpmWorkspaces,
    /// Npm workspaces
    NpmWorkspaces,
    /// Cargo workspace
    CargoWorkspace,
    /// Go workspace
    GoWorkspace,
    /// Bazel
    Bazel,
    /// Buck/Buck2
    Buck,
    /// Pants
    Pants,
    /// Plain monorepo (no specific tooling)
    Plain,
}

/// Detect workspace type from root
pub fn detect_workspace_type(root: &Path) -> WorkspaceType {
    // Check for specific build tools first
    if root.join("nx.json").exists() {
        return WorkspaceType::Nx;
    }
    if root.join("turbo.json").exists() {
        return WorkspaceType::Turborepo;
    }
    if root.join("lerna.json").exists() {
        return WorkspaceType::Lerna;
    }
    if root.join("WORKSPACE").exists() || root.join("WORKSPACE.bazel").exists() {
        return WorkspaceType::Bazel;
    }
    if root.join("BUCK").exists() || root.join(".buckconfig").exists() {
        return WorkspaceType::Buck;
    }
    if root.join("pants.toml").exists() || root.join("pants.ini").exists() {
        return WorkspaceType::Pants;
    }
    
    // Check Cargo workspace
    if let Ok(content) = std::fs::read_to_string(root.join("Cargo.toml")) {
        if content.contains("[workspace]") {
            return WorkspaceType::CargoWorkspace;
        }
    }
    
    // Check Go workspace
    if root.join("go.work").exists() {
        return WorkspaceType::GoWorkspace;
    }
    
    // Check package.json workspaces
    if let Ok(content) = std::fs::read_to_string(root.join("package.json")) {
        if content.contains("\"workspaces\"") {
            if root.join("pnpm-workspace.yaml").exists() {
                return WorkspaceType::PnpmWorkspaces;
            }
            if root.join("yarn.lock").exists() {
                return WorkspaceType::YarnWorkspaces;
            }
            return WorkspaceType::NpmWorkspaces;
        }
    }
    
    // Check pnpm workspace
    if root.join("pnpm-workspace.yaml").exists() {
        return WorkspaceType::PnpmWorkspaces;
    }
    
    WorkspaceType::Plain
}

/// Get recommended commands for workspace type
pub fn get_workspace_commands(ws_type: &WorkspaceType) -> WorkspaceCommands {
    match ws_type {
        WorkspaceType::Nx => WorkspaceCommands {
            install: "pnpm install".into(),
            build: "nx build".into(),
            test: "nx test".into(),
            lint: "nx lint".into(),
            affected_build: Some("nx affected --target=build".into()),
            affected_test: Some("nx affected --target=test".into()),
            graph: Some("nx graph".into()),
        },
        WorkspaceType::Turborepo => WorkspaceCommands {
            install: "pnpm install".into(),
            build: "turbo build".into(),
            test: "turbo test".into(),
            lint: "turbo lint".into(),
            affected_build: Some("turbo build --filter=...[origin/main]".into()),
            affected_test: Some("turbo test --filter=...[origin/main]".into()),
            graph: None,
        },
        WorkspaceType::CargoWorkspace => WorkspaceCommands {
            install: "cargo fetch".into(),
            build: "cargo build".into(),
            test: "cargo test".into(),
            lint: "cargo clippy".into(),
            affected_build: None,
            affected_test: None,
            graph: Some("cargo tree".into()),
        },
        _ => WorkspaceCommands {
            install: "npm install".into(),
            build: "npm run build".into(),
            test: "npm test".into(),
            lint: "npm run lint".into(),
            affected_build: None,
            affected_test: None,
            graph: None,
        },
    }
}

/// Common workspace commands
#[derive(Debug, Clone)]
pub struct WorkspaceCommands {
    pub install: String,
    pub build: String,
    pub test: String,
    pub lint: String,
    pub affected_build: Option<String>,
    pub affected_test: Option<String>,
    pub graph: Option<String>,
}
