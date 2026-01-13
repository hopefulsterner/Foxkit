//! Task runner

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::broadcast;

use crate::{Task, TaskId, TaskType, TaskEvent};

/// Task runner
pub struct TaskRunner {
    /// Shell to use
    shell: String,
    /// Shell args
    shell_args: Vec<String>,
}

impl TaskRunner {
    pub fn new() -> Self {
        #[cfg(windows)]
        let (shell, shell_args) = ("cmd".to_string(), vec!["/C".to_string()]);
        
        #[cfg(not(windows))]
        let (shell, shell_args) = {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            (shell, vec!["-c".to_string()])
        };

        Self { shell, shell_args }
    }

    /// Run a task
    pub async fn run(
        &self,
        id: TaskId,
        task: &Task,
        events: broadcast::Sender<TaskEvent>,
    ) -> anyhow::Result<TaskHandle> {
        let command = self.build_command(task);
        
        // Notify start
        let _ = events.send(TaskEvent::Started {
            id,
            name: task.name.clone(),
        });

        let mut child = Command::new(&self.shell)
            .args(&self.shell_args)
            .arg(&command)
            .current_dir(task.cwd.as_deref().unwrap_or(std::path::Path::new(".")))
            .envs(&task.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Stream output
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        if let Some(stdout) = stdout {
            let events = events.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = events.send(TaskEvent::Output {
                        id,
                        data: format!("{}\n", line),
                    });
                }
            });
        }

        if let Some(stderr) = stderr {
            let events = events.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = events.send(TaskEvent::Output {
                        id,
                        data: format!("{}\n", line),
                    });
                }
            });
        }

        // Wait for completion in background
        let name = task.name.clone();
        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => {
                    let exit_code = status.code().unwrap_or(-1);
                    let _ = events.send(TaskEvent::Completed { id, exit_code });
                }
                Err(e) => {
                    let _ = events.send(TaskEvent::Failed {
                        id,
                        error: e.to_string(),
                    });
                }
            }
        });

        Ok(TaskHandle {
            id,
            name: task.name.clone(),
        })
    }

    /// Build command string
    fn build_command(&self, task: &Task) -> String {
        match task.task_type {
            TaskType::Shell => {
                if task.args.is_empty() {
                    task.command.clone()
                } else {
                    format!("{} {}", task.command, task.args.join(" "))
                }
            }
            TaskType::Process => {
                format!("{} {}", task.command, task.args.join(" "))
            }
            TaskType::Npm => {
                format!("npm run {}", task.command)
            }
            TaskType::Cargo => {
                if task.args.is_empty() {
                    format!("cargo {}", task.command)
                } else {
                    format!("cargo {} {}", task.command, task.args.join(" "))
                }
            }
            TaskType::Gradle => {
                format!("./gradlew {}", task.command)
            }
            TaskType::Maven => {
                format!("mvn {}", task.command)
            }
            TaskType::Make => {
                format!("make {}", task.command)
            }
            TaskType::Custom => task.command.clone(),
        }
    }
}

impl Default for TaskRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a running task
#[derive(Debug)]
pub struct TaskHandle {
    /// Task ID
    pub id: TaskId,
    /// Task name
    pub name: String,
}

impl TaskHandle {
    /// Cancel the task
    pub fn cancel(&self) {
        // TODO: Implement proper cancellation
        tracing::info!("Cancelling task: {}", self.name);
    }
}
