//! Task Scheduler
//!
//! Parallel task scheduling with dependencies.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::{broadcast, mpsc, Semaphore};

use super::{Task, TaskId};

/// Task scheduler for parallel execution
pub struct TaskScheduler {
    /// Maximum parallel tasks
    max_parallel: usize,
    /// Running tasks
    running: RwLock<HashMap<TaskId, RunningTask>>,
    /// Task queue
    queue: RwLock<VecDeque<QueuedTask>>,
    /// Completed tasks
    completed: RwLock<HashSet<String>>,
    /// Event sender
    event_tx: broadcast::Sender<SchedulerEvent>,
    /// Semaphore for parallelism control
    semaphore: Arc<Semaphore>,
}

/// A task in the queue
#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub id: TaskId,
    pub task: Task,
    pub priority: i32,
    pub dependencies: Vec<String>,
}

/// A running task
#[derive(Debug)]
pub struct RunningTask {
    pub id: TaskId,
    pub task: Task,
    pub started_at: std::time::Instant,
    pub cancel_tx: Option<mpsc::Sender<()>>,
}

/// Scheduler events
#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    TaskQueued { id: TaskId, name: String },
    TaskStarted { id: TaskId, name: String },
    TaskCompleted { id: TaskId, name: String, success: bool, duration_ms: u64 },
    TaskCancelled { id: TaskId, name: String },
    TaskOutput { id: TaskId, output: String },
    QueueEmpty,
}

impl TaskScheduler {
    pub fn new(max_parallel: usize) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            max_parallel,
            running: RwLock::new(HashMap::new()),
            queue: RwLock::new(VecDeque::new()),
            completed: RwLock::new(HashSet::new()),
            event_tx,
            semaphore: Arc::new(Semaphore::new(max_parallel)),
        }
    }

    /// Queue a task for execution
    pub fn queue(&self, task: Task, priority: i32) -> TaskId {
        let id = TaskId::new();
        let dependencies = task.depends_on.clone();
        
        self.queue.write().push_back(QueuedTask {
            id,
            task: task.clone(),
            priority,
            dependencies,
        });
        
        // Sort by priority (higher first)
        self.queue.write().make_contiguous().sort_by(|a, b| b.priority.cmp(&a.priority));
        
        let _ = self.event_tx.send(SchedulerEvent::TaskQueued {
            id,
            name: task.name.clone(),
        });
        
        id
    }

    /// Check if a task is ready to run (all dependencies completed)
    fn is_ready(&self, task: &QueuedTask) -> bool {
        let completed = self.completed.read();
        task.dependencies.iter().all(|dep| completed.contains(dep))
    }

    /// Get next ready task from queue
    fn pop_ready(&self) -> Option<QueuedTask> {
        let mut queue = self.queue.write();
        
        for i in 0..queue.len() {
            if self.is_ready(&queue[i]) {
                return Some(queue.remove(i).unwrap());
            }
        }
        
        None
    }

    /// Run the scheduler
    pub async fn run(&self) {
        loop {
            // Wait for a permit
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            
            // Get next ready task
            let task = match self.pop_ready() {
                Some(t) => t,
                None => {
                    // No ready tasks, check if we're done
                    drop(permit);
                    if self.queue.read().is_empty() && self.running.read().is_empty() {
                        let _ = self.event_tx.send(SchedulerEvent::QueueEmpty);
                        break;
                    }
                    // Wait a bit and try again
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            
            // Execute task
            let id = task.id;
            let name = task.task.name.clone();
            let event_tx = self.event_tx.clone();
            let completed = Arc::new(self.completed.clone());
            
            let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
            
            self.running.write().insert(id, RunningTask {
                id,
                task: task.task.clone(),
                started_at: std::time::Instant::now(),
                cancel_tx: Some(cancel_tx),
            });
            
            let _ = event_tx.send(SchedulerEvent::TaskStarted {
                id,
                name: name.clone(),
            });
            
            let task_clone = task.task.clone();
            let running = Arc::new(self.running.clone());
            
            tokio::spawn(async move {
                let started = std::time::Instant::now();
                
                // Execute the task
                let result = tokio::select! {
                    result = Self::execute_task(&task_clone, &event_tx, id) => result,
                    _ = cancel_rx.recv() => Err(anyhow::anyhow!("Cancelled")),
                };
                
                let duration_ms = started.elapsed().as_millis() as u64;
                let success = result.is_ok();
                
                // Remove from running
                running.write().remove(&id);
                
                // Mark as completed
                if success {
                    completed.write().insert(name.clone());
                }
                
                let _ = event_tx.send(SchedulerEvent::TaskCompleted {
                    id,
                    name,
                    success,
                    duration_ms,
                });
                
                drop(permit);
            });
        }
    }

    async fn execute_task(task: &Task, event_tx: &broadcast::Sender<SchedulerEvent>, id: TaskId) -> anyhow::Result<()> {
        let mut cmd = tokio::process::Command::new(&task.command);
        cmd.args(&task.args);
        
        if let Some(cwd) = &task.cwd {
            cmd.current_dir(cwd);
        }
        
        for (key, value) in &task.env {
            cmd.env(key, value);
        }
        
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        
        let mut child = cmd.spawn()?;
        
        // Read output
        if let Some(stdout) = child.stdout.take() {
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let mut reader = tokio::io::BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let _ = event_tx.send(SchedulerEvent::TaskOutput {
                        id,
                        output: line,
                    });
                }
            });
        }
        
        let status = child.wait().await?;
        
        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task failed with exit code: {:?}", status.code()))
        }
    }

    /// Cancel a running task
    pub fn cancel(&self, id: TaskId) {
        if let Some(task) = self.running.write().remove(&id) {
            if let Some(cancel_tx) = task.cancel_tx {
                let _ = cancel_tx.try_send(());
            }
            let _ = self.event_tx.send(SchedulerEvent::TaskCancelled {
                id,
                name: task.task.name.clone(),
            });
        } else {
            // Remove from queue if not running
            self.queue.write().retain(|t| t.id != id);
        }
    }

    /// Cancel all tasks
    pub fn cancel_all(&self) {
        let running_ids: Vec<TaskId> = self.running.read().keys().copied().collect();
        for id in running_ids {
            self.cancel(id);
        }
        self.queue.write().clear();
    }

    /// Get running tasks
    pub fn running_tasks(&self) -> Vec<(TaskId, String, u64)> {
        self.running.read().values().map(|t| {
            (t.id, t.task.name.clone(), t.started_at.elapsed().as_millis() as u64)
        }).collect()
    }

    /// Get queued tasks
    pub fn queued_tasks(&self) -> Vec<(TaskId, String)> {
        self.queue.read().iter().map(|t| (t.id, t.task.name.clone())).collect()
    }

    /// Subscribe to scheduler events
    pub fn subscribe(&self) -> broadcast::Receiver<SchedulerEvent> {
        self.event_tx.subscribe()
    }

    /// Set max parallel tasks
    pub fn set_max_parallel(&mut self, max: usize) {
        self.max_parallel = max;
        // Note: This won't affect already acquired permits
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new(num_cpus::get())
    }
}

/// Task graph for dependency-based execution
pub struct TaskGraph {
    tasks: HashMap<String, Task>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self { tasks: HashMap::new() }
    }

    /// Add a task to the graph
    pub fn add(&mut self, task: Task) {
        self.tasks.insert(task.name.clone(), task);
    }

    /// Get execution order (topological sort)
    pub fn execution_order(&self) -> Vec<String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        
        for (name, task) in &self.tasks {
            in_degree.entry(name.clone()).or_insert(0);
            for dep in &task.depends_on {
                graph.entry(dep.clone()).or_default().push(name.clone());
                *in_degree.entry(name.clone()).or_insert(0) += 1;
            }
        }
        
        let mut queue: VecDeque<String> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();
        
        let mut order = Vec::new();
        let mut remaining = in_degree.clone();
        
        while let Some(node) = queue.pop_front() {
            order.push(node.clone());
            
            if let Some(dependents) = graph.get(&node) {
                for dependent in dependents {
                    if let Some(deg) = remaining.get_mut(dependent) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }
        
        order
    }

    /// Get parallel execution stages
    pub fn parallel_stages(&self) -> Vec<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        
        for (name, task) in &self.tasks {
            in_degree.entry(name.clone()).or_insert(0);
            for dep in &task.depends_on {
                graph.entry(dep.clone()).or_default().push(name.clone());
                *in_degree.entry(name.clone()).or_insert(0) += 1;
            }
        }
        
        let mut stages = Vec::new();
        let mut queue: VecDeque<String> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();
        
        let mut remaining = in_degree.clone();
        
        while !queue.is_empty() {
            let stage: Vec<String> = queue.drain(..).collect();
            
            for node in &stage {
                if let Some(dependents) = graph.get(node) {
                    for dependent in dependents {
                        if let Some(deg) = remaining.get_mut(dependent) {
                            *deg -= 1;
                            if *deg == 0 {
                                queue.push_back(dependent.clone());
                            }
                        }
                    }
                }
            }
            
            stages.push(stage);
        }
        
        stages
    }
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}
