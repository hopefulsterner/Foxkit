//! Task runner integration
//! 
//! Orchestrates build, test, and other tasks across the monorepo

use std::path::PathBuf;
use std::collections::HashMap;
use tokio::process::Command;
use tokio::sync::mpsc;
use anyhow::Result;

/// A task to run
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task ID
    pub id: TaskId,
    /// Task name/label
    pub name: String,
    /// Command to run
    pub command: String,
    /// Arguments
    pub args: Vec<String>,
    /// Working directory
    pub cwd: PathBuf,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Task type
    pub task_type: TaskType,
    /// Current status
    pub status: TaskStatus,
    /// Associated package (if any)
    pub package: Option<String>,
    /// Depends on other tasks
    pub depends_on: Vec<TaskId>,
}

/// Task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Build,
    Test,
    Lint,
    Run,
    Watch,
    Custom,
}

/// Task execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed { exit_code: Option<i32>, error: String },
    Cancelled,
}

impl Task {
    /// Create a new build task
    pub fn build(id: TaskId, name: &str, command: &str, cwd: PathBuf) -> Self {
        Self {
            id,
            name: name.to_string(),
            command: command.to_string(),
            args: vec![],
            cwd,
            env: HashMap::new(),
            task_type: TaskType::Build,
            status: TaskStatus::Pending,
            package: None,
            depends_on: vec![],
        }
    }

    /// Create a test task
    pub fn test(id: TaskId, name: &str, command: &str, cwd: PathBuf) -> Self {
        Self {
            id,
            name: name.to_string(),
            command: command.to_string(),
            args: vec![],
            cwd,
            env: HashMap::new(),
            task_type: TaskType::Test,
            status: TaskStatus::Pending,
            package: None,
            depends_on: vec![],
        }
    }

    /// Add arguments
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Associate with a package
    pub fn for_package(mut self, package: &str) -> Self {
        self.package = Some(package.to_string());
        self
    }

    /// Add dependency
    pub fn depends_on(mut self, task_id: TaskId) -> Self {
        self.depends_on.push(task_id);
        self
    }
}

/// Task runner - executes and orchestrates tasks
pub struct TaskRunner {
    tasks: HashMap<TaskId, Task>,
    next_id: u64,
    /// Sender for task output
    output_tx: Option<mpsc::UnboundedSender<TaskOutput>>,
}

/// Task output event
#[derive(Debug, Clone)]
pub struct TaskOutput {
    pub task_id: TaskId,
    pub output_type: OutputType,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    Stdout,
    Stderr,
    Status,
}

impl TaskRunner {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_id: 1,
            output_tx: None,
        }
    }

    /// Set output channel
    pub fn with_output(mut self, tx: mpsc::UnboundedSender<TaskOutput>) -> Self {
        self.output_tx = Some(tx);
        self
    }

    /// Register a task
    pub fn register(&mut self, mut task: Task) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id += 1;
        task.id = id;
        self.tasks.insert(id, task);
        id
    }

    /// Run a task
    pub async fn run(&mut self, id: TaskId) -> Result<TaskStatus> {
        // Check dependencies first
        if let Some(task) = self.tasks.get(&id) {
            for dep_id in task.depends_on.clone() {
                let dep_status = self.run(dep_id).await?;
                if !matches!(dep_status, TaskStatus::Success) {
                    return Ok(TaskStatus::Failed {
                        exit_code: None,
                        error: format!("Dependency {:?} failed", dep_id),
                    });
                }
            }
        }

        let task = self.tasks.get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        
        task.status = TaskStatus::Running;
        self.send_output(id, OutputType::Status, format!("Running: {}", task.name));

        // Build command
        let mut cmd = Command::new(&task.command);
        cmd.args(&task.args);
        cmd.current_dir(&task.cwd);
        cmd.envs(&task.env);
        
        // Capture output
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Spawn
        let mut child = cmd.spawn()?;
        
        // Read output
        if let Some(stdout) = child.stdout.take() {
            let tx = self.output_tx.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Some(tx) = &tx {
                        let _ = tx.send(TaskOutput {
                            task_id: id,
                            output_type: OutputType::Stdout,
                            content: line,
                        });
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let tx = self.output_tx.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Some(tx) = &tx {
                        let _ = tx.send(TaskOutput {
                            task_id: id,
                            output_type: OutputType::Stderr,
                            content: line,
                        });
                    }
                }
            });
        }

        // Wait for completion
        let status = child.wait().await?;
        
        let task_status = if status.success() {
            TaskStatus::Success
        } else {
            TaskStatus::Failed {
                exit_code: status.code(),
                error: "Process exited with non-zero code".to_string(),
            }
        };

        if let Some(task) = self.tasks.get_mut(&id) {
            task.status = task_status.clone();
        }

        self.send_output(id, OutputType::Status, format!("Completed: {:?}", task_status));

        Ok(task_status)
    }

    /// Run multiple tasks in parallel
    pub async fn run_parallel(&mut self, ids: Vec<TaskId>) -> Result<Vec<(TaskId, TaskStatus)>> {
        let mut handles = Vec::new();
        
        for id in ids {
            // Clone needed data for the task
            if let Some(task) = self.tasks.get(&id).cloned() {
                let output_tx = self.output_tx.clone();
                
                let handle = tokio::spawn(async move {
                    // Execute task (simplified - real impl would be more complex)
                    let mut cmd = Command::new(&task.command);
                    cmd.args(&task.args);
                    cmd.current_dir(&task.cwd);
                    cmd.envs(&task.env);
                    
                    match cmd.status().await {
                        Ok(status) if status.success() => (id, TaskStatus::Success),
                        Ok(status) => (id, TaskStatus::Failed {
                            exit_code: status.code(),
                            error: "Non-zero exit".to_string(),
                        }),
                        Err(e) => (id, TaskStatus::Failed {
                            exit_code: None,
                            error: e.to_string(),
                        }),
                    }
                });
                
                handles.push(handle);
            }
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                if let Some(task) = self.tasks.get_mut(&result.0) {
                    task.status = result.1.clone();
                }
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Get task by ID
    pub fn get(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(&id)
    }

    /// List all tasks
    pub fn list(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }

    fn send_output(&self, task_id: TaskId, output_type: OutputType, content: String) {
        if let Some(tx) = &self.output_tx {
            let _ = tx.send(TaskOutput {
                task_id,
                output_type,
                content,
            });
        }
    }
}

impl Default for TaskRunner {
    fn default() -> Self {
        Self::new()
    }
}
