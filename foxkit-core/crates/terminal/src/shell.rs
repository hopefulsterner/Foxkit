//! Shell detection and configuration

use std::path::PathBuf;

/// Shell configuration
#[derive(Debug, Clone)]
pub struct Shell {
    /// Shell executable path
    pub path: PathBuf,
    /// Shell name
    pub name: String,
    /// Additional arguments
    pub args: Vec<String>,
}

impl Shell {
    /// Detect default shell
    pub fn detect() -> Self {
        #[cfg(unix)]
        {
            if let Ok(shell) = std::env::var("SHELL") {
                let path = PathBuf::from(&shell);
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("sh")
                    .to_string();
                
                return Self {
                    path,
                    name,
                    args: vec!["-l".to_string()], // Login shell
                };
            }
        }
        
        #[cfg(windows)]
        {
            if let Ok(comspec) = std::env::var("COMSPEC") {
                return Self {
                    path: PathBuf::from(&comspec),
                    name: "cmd".to_string(),
                    args: vec![],
                };
            }
        }
        
        // Fallback
        Self {
            path: PathBuf::from("/bin/sh"),
            name: "sh".to_string(),
            args: vec![],
        }
    }

    /// Create a bash shell
    pub fn bash() -> Self {
        Self {
            path: PathBuf::from("/bin/bash"),
            name: "bash".to_string(),
            args: vec!["-l".to_string()],
        }
    }

    /// Create a zsh shell
    pub fn zsh() -> Self {
        Self {
            path: PathBuf::from("/bin/zsh"),
            name: "zsh".to_string(),
            args: vec!["-l".to_string()],
        }
    }

    /// Create a fish shell
    pub fn fish() -> Self {
        Self {
            path: PathBuf::from("/usr/bin/fish"),
            name: "fish".to_string(),
            args: vec!["-l".to_string()],
        }
    }

    /// Get shell command line
    pub fn command(&self) -> String {
        let mut parts = vec![self.path.to_string_lossy().to_string()];
        parts.extend(self.args.clone());
        parts.join(" ")
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::detect()
    }
}
