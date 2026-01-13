//! Project tasks

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};

/// A task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub command: String,
    pub group: TaskGroup,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub problem_matcher: Option<String>,
    #[serde(default)]
    pub presentation: TaskPresentation,
}

impl Task {
    pub fn new(name: &str, command: &str, group: TaskGroup) -> Self {
        Self {
            name: name.to_string(),
            command: command.to_string(),
            group,
            is_default: false,
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            problem_matcher: None,
            presentation: TaskPresentation::default(),
        }
    }

    pub fn with_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Build full command
    pub fn full_command(&self) -> String {
        if self.args.is_empty() {
            self.command.clone()
        } else {
            format!("{} {}", self.command, self.args.join(" "))
        }
    }
}

/// Task group
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskGroup {
    #[default]
    None,
    Build,
    Test,
    Clean,
    Rebuild,
}

impl TaskGroup {
    pub fn label(&self) -> &'static str {
        match self {
            TaskGroup::None => "none",
            TaskGroup::Build => "build",
            TaskGroup::Test => "test",
            TaskGroup::Clean => "clean",
            TaskGroup::Rebuild => "rebuild",
        }
    }
}

/// Task presentation options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskPresentation {
    /// Show output
    #[serde(default = "default_reveal")]
    pub reveal: RevealKind,
    /// Echo command
    #[serde(default = "default_true")]
    pub echo: bool,
    /// Focus terminal
    #[serde(default)]
    pub focus: bool,
    /// Panel behavior
    #[serde(default)]
    pub panel: PanelKind,
    /// Show reuse message
    #[serde(default = "default_true")]
    pub show_reuse_message: bool,
    /// Clear terminal before run
    #[serde(default)]
    pub clear: bool,
}

fn default_reveal() -> RevealKind { RevealKind::Always }
fn default_true() -> bool { true }

/// Reveal behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RevealKind {
    #[default]
    Always,
    Silent,
    Never,
}

/// Panel behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanelKind {
    #[default]
    Shared,
    Dedicated,
    New,
}

/// Task runner
pub struct TaskRunner {
    cwd: PathBuf,
}

impl TaskRunner {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }

    /// Run a task
    pub fn run(&self, task: &Task) -> anyhow::Result<TaskExecution> {
        let cwd = task.cwd.as_ref().unwrap_or(&self.cwd);
        
        // Parse command
        let parts: Vec<&str> = task.command.split_whitespace().collect();
        let (program, args) = if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        } else {
            (parts[0], &parts[1..])
        };

        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.args(&task.args);
        cmd.current_dir(cwd);
        cmd.envs(&task.env);
        
        // Inherit stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn()?;
        
        Ok(TaskExecution {
            task_name: task.name.clone(),
            child,
        })
    }

    /// Run and wait
    pub fn run_sync(&self, task: &Task) -> anyhow::Result<TaskResult> {
        let mut execution = self.run(task)?;
        execution.wait()
    }
}

/// Running task execution
pub struct TaskExecution {
    pub task_name: String,
    child: std::process::Child,
}

impl TaskExecution {
    /// Wait for completion
    pub fn wait(&mut self) -> anyhow::Result<TaskResult> {
        let output = self.child.wait_with_output()?;
        
        Ok(TaskResult {
            task_name: self.task_name.clone(),
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Kill the task
    pub fn kill(&mut self) -> anyhow::Result<()> {
        self.child.kill()?;
        Ok(())
    }
}

/// Task result
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_name: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}
