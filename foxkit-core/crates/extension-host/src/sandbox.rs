//! Extension sandbox - isolates extension execution

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::Permission;

/// Sandbox configuration
pub struct Sandbox {
    /// Allowed file paths (read)
    allowed_read_paths: HashSet<PathBuf>,
    /// Allowed file paths (write)
    allowed_write_paths: HashSet<PathBuf>,
    /// Allowed network hosts
    allowed_hosts: HashSet<String>,
    /// Allowed environment variables
    allowed_env_vars: HashSet<String>,
    /// Allowed commands
    allowed_commands: HashSet<String>,
    /// Max memory (bytes)
    max_memory: usize,
    /// Max CPU time (ms)
    max_cpu_time: u64,
    /// Max file size (bytes)
    max_file_size: usize,
}

impl Sandbox {
    pub fn new() -> Self {
        Self {
            allowed_read_paths: HashSet::new(),
            allowed_write_paths: HashSet::new(),
            allowed_hosts: HashSet::new(),
            allowed_env_vars: HashSet::new(),
            allowed_commands: HashSet::new(),
            max_memory: 256 * 1024 * 1024, // 256 MB
            max_cpu_time: 30_000,           // 30 seconds
            max_file_size: 10 * 1024 * 1024, // 10 MB
        }
    }

    /// Configure sandbox from permissions
    pub fn from_permissions(permissions: &[Permission], workspace_root: &Path) -> Self {
        let mut sandbox = Self::new();
        
        for permission in permissions {
            match permission {
                Permission::FileSystemRead => {
                    sandbox.allow_read_path(workspace_root.to_path_buf());
                }
                Permission::FileSystemWrite => {
                    sandbox.allow_write_path(workspace_root.to_path_buf());
                }
                Permission::FileSystemReadExternal => {
                    // Allow home directory
                    if let Some(home) = dirs::home_dir() {
                        sandbox.allow_read_path(home);
                    }
                }
                Permission::Network => {
                    sandbox.allow_all_hosts();
                }
                Permission::ProcessExecuteCommand(cmd) => {
                    sandbox.allow_command(cmd.clone());
                }
                Permission::ProcessExecute => {
                    sandbox.allow_all_commands();
                }
                Permission::EnvironmentRead => {
                    sandbox.allow_env_var("PATH");
                    sandbox.allow_env_var("HOME");
                    sandbox.allow_env_var("USER");
                }
                _ => {}
            }
        }
        
        sandbox
    }

    /// Allow reading from a path
    pub fn allow_read_path(&mut self, path: PathBuf) {
        self.allowed_read_paths.insert(path);
    }

    /// Allow writing to a path
    pub fn allow_write_path(&mut self, path: PathBuf) {
        self.allowed_write_paths.insert(path);
    }

    /// Allow network access to host
    pub fn allow_host(&mut self, host: String) {
        self.allowed_hosts.insert(host);
    }

    /// Allow all hosts
    pub fn allow_all_hosts(&mut self) {
        self.allowed_hosts.insert("*".to_string());
    }

    /// Allow reading environment variable
    pub fn allow_env_var(&mut self, var: &str) {
        self.allowed_env_vars.insert(var.to_string());
    }

    /// Allow executing a command
    pub fn allow_command(&mut self, cmd: String) {
        self.allowed_commands.insert(cmd);
    }

    /// Allow all commands
    pub fn allow_all_commands(&mut self) {
        self.allowed_commands.insert("*".to_string());
    }

    /// Set memory limit
    pub fn set_max_memory(&mut self, bytes: usize) {
        self.max_memory = bytes;
    }

    /// Set CPU time limit
    pub fn set_max_cpu_time(&mut self, ms: u64) {
        self.max_cpu_time = ms;
    }

    // Check methods

    /// Can read from path?
    pub fn can_read(&self, path: &Path) -> bool {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        for allowed in &self.allowed_read_paths {
            let allowed = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            if path.starts_with(&allowed) {
                return true;
            }
        }
        
        false
    }

    /// Can write to path?
    pub fn can_write(&self, path: &Path) -> bool {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        
        for allowed in &self.allowed_write_paths {
            let allowed = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            if path.starts_with(&allowed) {
                return true;
            }
        }
        
        false
    }

    /// Can access host?
    pub fn can_access_host(&self, host: &str) -> bool {
        self.allowed_hosts.contains("*") || self.allowed_hosts.contains(host)
    }

    /// Can read env var?
    pub fn can_read_env(&self, var: &str) -> bool {
        self.allowed_env_vars.contains(var)
    }

    /// Can execute command?
    pub fn can_execute(&self, cmd: &str) -> bool {
        if self.allowed_commands.contains("*") {
            return true;
        }
        
        // Check command name (without path)
        let cmd_name = Path::new(cmd)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(cmd);
        
        self.allowed_commands.contains(cmd_name) || self.allowed_commands.contains(cmd)
    }

    /// Get memory limit
    pub fn max_memory(&self) -> usize {
        self.max_memory
    }

    /// Get CPU time limit
    pub fn max_cpu_time(&self) -> u64 {
        self.max_cpu_time
    }

    /// Get max file size
    pub fn max_file_size(&self) -> usize {
        self.max_file_size
    }
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Sandbox violation
#[derive(Debug, Clone)]
pub enum SandboxViolation {
    /// Attempted to read unauthorized path
    UnauthorizedRead(PathBuf),
    /// Attempted to write unauthorized path
    UnauthorizedWrite(PathBuf),
    /// Attempted to access unauthorized host
    UnauthorizedHost(String),
    /// Attempted to read unauthorized env var
    UnauthorizedEnvVar(String),
    /// Attempted to execute unauthorized command
    UnauthorizedCommand(String),
    /// Exceeded memory limit
    MemoryLimitExceeded(usize),
    /// Exceeded CPU time
    CpuTimeExceeded(u64),
    /// File size too large
    FileTooLarge(usize),
}

impl std::fmt::Display for SandboxViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnauthorizedRead(path) => write!(f, "Unauthorized read: {:?}", path),
            Self::UnauthorizedWrite(path) => write!(f, "Unauthorized write: {:?}", path),
            Self::UnauthorizedHost(host) => write!(f, "Unauthorized host access: {}", host),
            Self::UnauthorizedEnvVar(var) => write!(f, "Unauthorized env var: {}", var),
            Self::UnauthorizedCommand(cmd) => write!(f, "Unauthorized command: {}", cmd),
            Self::MemoryLimitExceeded(bytes) => write!(f, "Memory limit exceeded: {} bytes", bytes),
            Self::CpuTimeExceeded(ms) => write!(f, "CPU time exceeded: {} ms", ms),
            Self::FileTooLarge(size) => write!(f, "File too large: {} bytes", size),
        }
    }
}

impl std::error::Error for SandboxViolation {}

mod dirs {
    use std::path::PathBuf;
    
    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}
