//! Jupyter-like Notebook Support for Foxkit
//!
//! Interactive notebooks with code cells, markdown, and rich outputs.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Unique notebook identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NotebookId(pub Uuid);
impl NotebookId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl Default for NotebookId { fn default() -> Self { Self::new() } }

/// Unique cell identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellId(pub Uuid);
impl CellId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl Default for CellId { fn default() -> Self { Self::new() } }

/// Cell kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellKind { Code, Markdown }

/// Notebook cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCell {
    pub id: CellId,
    pub kind: CellKind,
    pub language: String,
    pub source: String,
    pub outputs: Vec<CellOutput>,
    pub metadata: CellMetadata,
    pub execution_order: Option<u32>,
}

/// Cell metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CellMetadata {
    pub editable: bool,
    pub collapsed: bool,
    pub scrolled: bool,
    pub tags: Vec<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Cell output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellOutput {
    pub output_type: OutputType,
    pub data: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Output type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputType { ExecuteResult, DisplayData, Stream, Error }

/// Execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState { Idle, Busy, Starting, Stopping }

/// Kernel info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelInfo {
    pub id: String,
    pub name: String,
    pub language: String,
    pub display_name: String,
}

/// Notebook document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookDocument {
    pub id: NotebookId,
    pub uri: String,
    pub cells: Vec<NotebookCell>,
    pub metadata: NotebookMetadata,
}

/// Notebook metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotebookMetadata {
    pub kernel_spec: Option<KernelSpec>,
    pub language_info: Option<LanguageInfo>,
    pub title: Option<String>,
    pub authors: Vec<String>,
}

/// Kernel specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSpec {
    pub name: String,
    pub display_name: String,
    pub language: String,
}

/// Language info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
    pub version: String,
    pub file_extension: String,
}

/// Kernel controller trait
#[async_trait]
pub trait NotebookKernel: Send + Sync {
    fn info(&self) -> KernelInfo;
    fn state(&self) -> ExecutionState;
    async fn execute(&self, cell: &NotebookCell) -> Result<Vec<CellOutput>, KernelError>;
    async fn interrupt(&self) -> Result<(), KernelError>;
    async fn restart(&self) -> Result<(), KernelError>;
    async fn shutdown(&self) -> Result<(), KernelError>;
}

/// Kernel error
#[derive(Debug, Clone)]
pub enum KernelError {
    NotStarted,
    Busy,
    ExecutionError(String),
    Timeout,
    ConnectionLost,
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "Kernel not started"),
            Self::Busy => write!(f, "Kernel busy"),
            Self::ExecutionError(e) => write!(f, "Execution error: {}", e),
            Self::Timeout => write!(f, "Kernel timeout"),
            Self::ConnectionLost => write!(f, "Connection lost"),
        }
    }
}

impl std::error::Error for KernelError {}

/// Notebook service
pub struct NotebookService {
    notebooks: RwLock<HashMap<NotebookId, NotebookDocument>>,
    kernels: RwLock<HashMap<NotebookId, Arc<dyn NotebookKernel>>>,
    execution_counter: RwLock<u32>,
}

impl NotebookService {
    pub fn new() -> Self {
        Self {
            notebooks: RwLock::new(HashMap::new()),
            kernels: RwLock::new(HashMap::new()),
            execution_counter: RwLock::new(0),
        }
    }

    pub fn create_notebook(&self, uri: &str) -> NotebookId {
        let doc = NotebookDocument {
            id: NotebookId::new(),
            uri: uri.to_string(),
            cells: Vec::new(),
            metadata: NotebookMetadata::default(),
        };
        let id = doc.id;
        self.notebooks.write().insert(id, doc);
        id
    }

    pub fn add_cell(&self, notebook_id: NotebookId, kind: CellKind, language: &str) -> Option<CellId> {
        let cell = NotebookCell {
            id: CellId::new(),
            kind,
            language: language.to_string(),
            source: String::new(),
            outputs: Vec::new(),
            metadata: CellMetadata::default(),
            execution_order: None,
        };
        let cell_id = cell.id;
        self.notebooks.write().get_mut(&notebook_id)?.cells.push(cell);
        Some(cell_id)
    }

    pub fn get_notebook(&self, id: NotebookId) -> Option<NotebookDocument> {
        self.notebooks.read().get(&id).cloned()
    }

    pub fn attach_kernel(&self, notebook_id: NotebookId, kernel: Arc<dyn NotebookKernel>) {
        self.kernels.write().insert(notebook_id, kernel);
    }

    pub fn next_execution_order(&self) -> u32 {
        let mut counter = self.execution_counter.write();
        *counter += 1;
        *counter
    }
}

impl Default for NotebookService { fn default() -> Self { Self::new() } }
