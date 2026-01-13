//! Extension permission model

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

/// Permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    // File system
    /// Read files in workspace
    FileSystemRead,
    /// Write files in workspace
    FileSystemWrite,
    /// Read files outside workspace
    FileSystemReadExternal,
    /// Write files outside workspace
    FileSystemWriteExternal,
    
    // Network
    /// Make network requests
    Network,
    /// Listen on network ports
    NetworkListen,
    
    // Process
    /// Execute processes
    ProcessExecute,
    /// Execute specific command
    ProcessExecuteCommand(String),
    
    // Clipboard
    /// Read clipboard
    ClipboardRead,
    /// Write clipboard
    ClipboardWrite,
    
    // Editor
    /// Access active editor
    EditorAccess,
    /// Modify documents
    DocumentEdit,
    
    // Terminal
    /// Create terminals
    TerminalCreate,
    /// Send to terminals
    TerminalWrite,
    
    // Debug
    /// Debug sessions
    DebugAccess,
    
    // Secrets
    /// Store secrets
    SecretStorage,
    
    // Environment
    /// Read environment variables
    EnvironmentRead,
    
    // Git
    /// Access git repositories
    GitAccess,
    
    // AI
    /// Use AI completion
    AICompletion,
    /// Use AI agent
    AIAgent,
    
    // Authentication
    /// Request authentication
    Authentication,
    
    // Custom
    /// Custom permission
    Custom(String),
}

impl Permission {
    /// Get human-readable description
    pub fn description(&self) -> &str {
        match self {
            Permission::FileSystemRead => "Read files in the workspace",
            Permission::FileSystemWrite => "Write files in the workspace",
            Permission::FileSystemReadExternal => "Read files outside the workspace",
            Permission::FileSystemWriteExternal => "Write files outside the workspace",
            Permission::Network => "Make network requests",
            Permission::NetworkListen => "Listen on network ports",
            Permission::ProcessExecute => "Execute system processes",
            Permission::ProcessExecuteCommand(_) => "Execute a specific command",
            Permission::ClipboardRead => "Read from clipboard",
            Permission::ClipboardWrite => "Write to clipboard",
            Permission::EditorAccess => "Access the active editor",
            Permission::DocumentEdit => "Modify documents",
            Permission::TerminalCreate => "Create terminals",
            Permission::TerminalWrite => "Send commands to terminals",
            Permission::DebugAccess => "Access debug sessions",
            Permission::SecretStorage => "Store sensitive data",
            Permission::EnvironmentRead => "Read environment variables",
            Permission::GitAccess => "Access git repositories",
            Permission::AICompletion => "Use AI code completion",
            Permission::AIAgent => "Use AI agent capabilities",
            Permission::Authentication => "Request user authentication",
            Permission::Custom(_) => "Custom permission",
        }
    }

    /// Risk level (higher = more dangerous)
    pub fn risk_level(&self) -> u8 {
        match self {
            Permission::FileSystemRead => 2,
            Permission::FileSystemWrite => 4,
            Permission::FileSystemReadExternal => 6,
            Permission::FileSystemWriteExternal => 8,
            Permission::Network => 5,
            Permission::NetworkListen => 7,
            Permission::ProcessExecute => 9,
            Permission::ProcessExecuteCommand(_) => 7,
            Permission::ClipboardRead => 3,
            Permission::ClipboardWrite => 2,
            Permission::EditorAccess => 1,
            Permission::DocumentEdit => 3,
            Permission::TerminalCreate => 5,
            Permission::TerminalWrite => 6,
            Permission::DebugAccess => 4,
            Permission::SecretStorage => 3,
            Permission::EnvironmentRead => 4,
            Permission::GitAccess => 3,
            Permission::AICompletion => 2,
            Permission::AIAgent => 5,
            Permission::Authentication => 3,
            Permission::Custom(_) => 5,
        }
    }

    /// Does this permission imply another?
    pub fn implies(&self, other: &Permission) -> bool {
        match (self, other) {
            // Write implies read
            (Permission::FileSystemWrite, Permission::FileSystemRead) => true,
            (Permission::FileSystemWriteExternal, Permission::FileSystemReadExternal) => true,
            (Permission::FileSystemWriteExternal, Permission::FileSystemWrite) => true,
            (Permission::FileSystemWriteExternal, Permission::FileSystemRead) => true,
            (Permission::FileSystemReadExternal, Permission::FileSystemRead) => true,
            // Network listen implies network
            (Permission::NetworkListen, Permission::Network) => true,
            // Process execute implies specific command
            (Permission::ProcessExecute, Permission::ProcessExecuteCommand(_)) => true,
            // Terminal write implies create
            (Permission::TerminalWrite, Permission::TerminalCreate) => true,
            // AI agent implies completion
            (Permission::AIAgent, Permission::AICompletion) => true,
            _ => false,
        }
    }
}

/// Set of permissions
#[derive(Debug, Clone, Default)]
pub struct PermissionSet {
    granted: HashSet<Permission>,
}

impl PermissionSet {
    pub fn new() -> Self {
        Self {
            granted: HashSet::new(),
        }
    }

    /// Grant a permission
    pub fn grant(&mut self, permission: Permission) {
        self.granted.insert(permission);
    }

    /// Revoke a permission
    pub fn revoke(&mut self, permission: &Permission) {
        self.granted.remove(permission);
    }

    /// Check if permission is granted (including implied)
    pub fn has(&self, permission: &Permission) -> bool {
        if self.granted.contains(permission) {
            return true;
        }
        
        // Check if any granted permission implies this one
        for granted in &self.granted {
            if granted.implies(permission) {
                return true;
            }
        }
        
        false
    }

    /// Get all granted permissions
    pub fn all(&self) -> impl Iterator<Item = &Permission> {
        self.granted.iter()
    }

    /// Clear all permissions
    pub fn clear(&mut self) {
        self.granted.clear();
    }

    /// Number of granted permissions
    pub fn len(&self) -> usize {
        self.granted.len()
    }

    pub fn is_empty(&self) -> bool {
        self.granted.is_empty()
    }

    /// Total risk score
    pub fn risk_score(&self) -> u32 {
        self.granted.iter().map(|p| p.risk_level() as u32).sum()
    }
}

/// Permission request from extension
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// Requested permission
    pub permission: Permission,
    /// Reason for request
    pub reason: Option<String>,
    /// Is it optional?
    pub optional: bool,
}

impl PermissionRequest {
    pub fn new(permission: Permission) -> Self {
        Self {
            permission,
            reason: None,
            optional: false,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}
