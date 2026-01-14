//! # Foxkit Task
//!
//! Task runner, build system, and watch mode.

pub mod config;
pub mod runner;
pub mod scheduler;
pub mod watcher;

pub use scheduler::{TaskScheduler, TaskGraph, SchedulerEvent, QueuedTask};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub use config::TaskConfig;
pub use runner::{TaskRunner, TaskHandle};
pub use watcher::FileWatcher;

/// Task ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

impl TaskId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// Task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task name
    pub name: String,
    /// Task type
    #[serde(rename = "type", default)]
    pub task_type: TaskType,
    /// Command to run
    pub command: String,
    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Problem matcher
    pub problem_matcher: Option<String>,
    /// Depends on other tasks
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Run in background
    #[serde(default)]
    pub background: bool,
    /// Watch patterns
    #[serde(default)]
    pub watch: Vec<String>,
    /// Group
    pub group: Option<TaskGroup>,
    /// Presentation options
    #[serde(default)]
    pub presentation: TaskPresentation,
}

impl Task {
    /// Create a shell task
    pub fn shell(name: &str, command: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Shell,
            command: command.to_string(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            problem_matcher: None,
            depends_on: Vec::new(),
            background: false,
            watch: Vec::new(),
            group: None,
            presentation: TaskPresentation::default(),
        }
    }

    /// Create a process task
    pub fn process(name: &str, command: &str, args: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Process,
            command: command.to_string(),
            args,
            cwd: None,
            env: HashMap::new(),
            problem_matcher: None,
            depends_on: Vec::new(),
            background: false,
            watch: Vec::new(),
            group: None,
            presentation: TaskPresentation::default(),
        }
    }

    /// Create npm task
    pub fn npm(name: &str, script: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Npm,
            command: script.to_string(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            problem_matcher: Some("$tsc".to_string()),
            depends_on: Vec::new(),
            background: false,
            watch: Vec::new(),
            group: None,
            presentation: TaskPresentation::default(),
        }
    }

    /// Create cargo task
    pub fn cargo(name: &str, subcommand: &str, args: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Cargo,
            command: subcommand.to_string(),
            args,
            cwd: None,
            env: HashMap::new(),
            problem_matcher: Some("$rustc".to_string()),
            depends_on: Vec::new(),
            background: false,
            watch: Vec::new(),
            group: None,
            presentation: TaskPresentation::default(),
        }
    }

    /// Add dependency
    pub fn depends(mut self, task: &str) -> Self {
        self.depends_on.push(task.to_string());
        self
    }

    /// Set working directory
    pub fn in_dir(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Add environment variable
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Set as build task
    pub fn as_build(mut self) -> Self {
        self.group = Some(TaskGroup::Build { is_default: true });
        self
    }

    /// Set as test task
    pub fn as_test(mut self) -> Self {
        self.group = Some(TaskGroup::Test { is_default: true });
        self
    }

    /// Run in background
    pub fn in_background(mut self) -> Self {
        self.background = true;
        self
    }

    /// Add watch patterns
    pub fn watching(mut self, patterns: Vec<String>) -> Self {
        self.watch = patterns;
        self.background = true;
        self
    }
}

/// Task type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    #[default]
    Shell,
    Process,
    Npm,
    Cargo,
    Gradle,
    Maven,
    Make,
    Custom,
}

/// Task group
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskGroup {
    Build { is_default: bool },
    Test { is_default: bool },
    None,
}

/// Task presentation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskPresentation {
    /// Reveal terminal
    #[serde(default)]
    pub reveal: RevealKind,
    /// Echo command
    #[serde(default = "default_true")]
    pub echo: bool,
    /// Focus terminal
    #[serde(default)]
    pub focus: bool,
    /// Panel sharing
    #[serde(default)]
    pub panel: PanelKind,
    /// Show reuse message
    #[serde(default = "default_true")]
    pub show_reuse_message: bool,
    /// Clear terminal
    #[serde(default)]
    pub clear: bool,
}

fn default_true() -> bool { true }

/// Reveal kind
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RevealKind {
    #[default]
    Always,
    Silent,
    Never,
}

/// Panel kind
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanelKind {
    #[default]
    Shared,
    Dedicated,
    New,
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Task event
#[derive(Debug, Clone)]
pub enum TaskEvent {
    Started { id: TaskId, name: String },
    Output { id: TaskId, data: String },
    Completed { id: TaskId, exit_code: i32 },
    Failed { id: TaskId, error: String },
}

/// Task service
pub struct TaskService {
    /// Task definitions
    tasks: RwLock<HashMap<String, Task>>,
    /// Running tasks
    running: RwLock<HashMap<TaskId, TaskHandle>>,
    /// Event broadcaster
    events: broadcast::Sender<TaskEvent>,
    /// Task runner
    runner: Arc<TaskRunner>,
}

impl TaskService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(100);
        Self {
            tasks: RwLock::new(HashMap::new()),
            running: RwLock::new(HashMap::new()),
            events,
            runner: Arc::new(TaskRunner::new()),
        }
    }

    /// Register a task
    pub fn register(&self, task: Task) {
        self.tasks.write().insert(task.name.clone(), task);
    }

    /// Get task by name
    pub fn get(&self, name: &str) -> Option<Task> {
        self.tasks.read().get(name).cloned()
    }

    /// List all tasks
    pub fn list(&self) -> Vec<Task> {
        self.tasks.read().values().cloned().collect()
    }

    /// Run a task by name
    pub async fn run(&self, name: &str) -> anyhow::Result<TaskId> {
        let task = self.get(name)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", name))?;

        // Run dependencies first
        for dep in &task.depends_on {
            self.run(dep).await?;
        }

        let id = TaskId::new();
        let handle = self.runner.run(id, &task, self.events.clone()).await?;
        
        self.running.write().insert(id, handle);
        Ok(id)
    }

    /// Cancel a running task
    pub fn cancel(&self, id: TaskId) -> bool {
        if let Some(handle) = self.running.write().remove(&id) {
            handle.cancel();
            true
        } else {
            false
        }
    }

    /// Subscribe to task events
    pub fn subscribe(&self) -> broadcast::Receiver<TaskEvent> {
        self.events.subscribe()
    }

    /// Get running task IDs
    pub fn running_tasks(&self) -> Vec<TaskId> {
        self.running.read().keys().copied().collect()
    }

    /// Load tasks from file (tasks.json)
    pub fn load_from_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let config: TaskConfig = serde_json::from_str(&content)?;
        
        for task in config.tasks {
            self.register(task);
        }

        Ok(())
    }

    /// Auto-detect tasks from project
    pub fn auto_detect(&self, project_root: &std::path::Path) {
        // Detect npm tasks
        let package_json = project_root.join("package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) {
                        for (name, _) in scripts {
                            self.register(Task::npm(&format!("npm: {}", name), name));
                        }
                    }
                }
            }
        }

        // Detect cargo tasks
        let cargo_toml = project_root.join("Cargo.toml");
        if cargo_toml.exists() {
            self.register(Task::cargo("cargo: build", "build", vec![]));
            self.register(Task::cargo("cargo: check", "check", vec![]));
            self.register(Task::cargo("cargo: test", "test", vec![]));
            self.register(Task::cargo("cargo: run", "run", vec![]));
            self.register(Task::cargo("cargo: clippy", "clippy", vec![]));
        }

        // Detect Makefile tasks
        let makefile = project_root.join("Makefile");
        if makefile.exists() {
            if let Ok(content) = std::fs::read_to_string(&makefile) {
                for line in content.lines() {
                    if let Some(target) = line.strip_suffix(':') {
                        if !target.starts_with('.') && !target.contains(' ') {
                            self.register(Task::shell(&format!("make: {}", target), &format!("make {}", target)));
                        }
                    }
                }
            }
        }
    }
}

impl Default for TaskService {
    fn default() -> Self {
        Self::new()
    }
}
