//! LSP client - communicates with a language server

use std::sync::Arc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::{mpsc, oneshot, Mutex};
use parking_lot::RwLock;
use anyhow::Result;
use lsp_types::*;

use crate::{ServerConfig, ServerState, LspEvent};
use crate::transport::Transport;
use crate::process::ServerProcess;

/// LSP client for a single language server
pub struct LspClient {
    /// Configuration
    config: ServerConfig,
    /// Server process
    process: Option<ServerProcess>,
    /// Transport layer
    transport: Option<Arc<Transport>>,
    /// Current state
    state: RwLock<ServerState>,
    /// Request ID counter
    next_id: AtomicI64,
    /// Pending requests
    pending: Mutex<HashMap<i64, oneshot::Sender<serde_json::Value>>>,
    /// Server capabilities
    capabilities: RwLock<Option<ServerCapabilities>>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<LspEvent>,
    /// Root URI
    root_uri: RwLock<Option<Url>>,
}

impl LspClient {
    /// Create new client
    pub fn new(config: ServerConfig, event_tx: mpsc::UnboundedSender<LspEvent>) -> Self {
        Self {
            config,
            process: None,
            transport: None,
            state: RwLock::new(ServerState::Stopped),
            next_id: AtomicI64::new(1),
            pending: Mutex::new(HashMap::new()),
            capabilities: RwLock::new(None),
            event_tx,
            root_uri: RwLock::new(None),
        }
    }

    /// Start the language server
    pub async fn start(&mut self, root_path: &std::path::Path) -> Result<()> {
        *self.state.write() = ServerState::Starting;

        // Start process
        let process = ServerProcess::spawn(&self.config)?;
        let transport = Transport::new(process.stdin(), process.stdout());

        self.process = Some(process);
        self.transport = Some(Arc::new(transport));

        // Set root URI
        let root_uri = Url::from_file_path(root_path).ok();
        *self.root_uri.write() = root_uri.clone();

        // Initialize
        self.initialize(root_uri).await?;

        *self.state.write() = ServerState::Running;

        self.event_tx.send(LspEvent::ServerStarted {
            language_id: self.config.language_id.clone(),
        }).ok();

        Ok(())
    }

    /// Stop the language server
    pub async fn stop(&mut self) -> Result<()> {
        *self.state.write() = ServerState::ShuttingDown;

        // Send shutdown request
        self.request::<request::Shutdown>(()).await.ok();

        // Send exit notification
        self.notify::<notification::Exit>(()).ok();

        // Kill process
        if let Some(mut process) = self.process.take() {
            process.kill().await.ok();
        }

        *self.state.write() = ServerState::Stopped;

        self.event_tx.send(LspEvent::ServerStopped {
            language_id: self.config.language_id.clone(),
        }).ok();

        Ok(())
    }

    /// Initialize the server
    async fn initialize(&self, root_uri: Option<Url>) -> Result<()> {
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri,
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    completion: Some(CompletionClientCapabilities {
                        completion_item: Some(CompletionItemCapability {
                            snippet_support: Some(true),
                            documentation_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    hover: Some(HoverClientCapabilities {
                        content_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
                        ..Default::default()
                    }),
                    definition: Some(GotoCapability::default()),
                    references: Some(DynamicRegistrationClientCapabilities::default()),
                    document_symbol: Some(DocumentSymbolClientCapabilities::default()),
                    code_action: Some(CodeActionClientCapabilities::default()),
                    publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                        related_information: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            initialization_options: self.config.initialization_options.clone(),
            ..Default::default()
        };

        let result: InitializeResult = self.request::<request::Initialize>(params).await?;
        *self.capabilities.write() = Some(result.capabilities);

        // Send initialized notification
        self.notify::<notification::Initialized>(InitializedParams {})?;

        Ok(())
    }

    /// Send a request
    pub async fn request<R: request::Request>(&self, params: R::Params) -> Result<R::Result>
    where
        R::Params: serde::Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        self.pending.lock().await.insert(id, tx);

        let transport = self.transport.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        transport.send_request(id, R::METHOD, serde_json::to_value(params)?).await?;

        let response = rx.await?;
        let result: R::Result = serde_json::from_value(response)?;

        Ok(result)
    }

    /// Send a notification
    pub fn notify<N: notification::Notification>(&self, params: N::Params) -> Result<()>
    where
        N::Params: serde::Serialize,
    {
        let transport = self.transport.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        transport.send_notification(N::METHOD, serde_json::to_value(params)?)?;

        Ok(())
    }

    // Convenience methods

    /// Notify document opened
    pub fn did_open(&self, uri: Url, language_id: &str, version: i32, text: String) -> Result<()> {
        self.notify::<notification::DidOpenTextDocument>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: language_id.to_string(),
                version,
                text,
            },
        })
    }

    /// Notify document changed
    pub fn did_change(&self, uri: Url, version: i32, changes: Vec<TextDocumentContentChangeEvent>) -> Result<()> {
        self.notify::<notification::DidChangeTextDocument>(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes: changes,
        })
    }

    /// Notify document saved
    pub fn did_save(&self, uri: Url, text: Option<String>) -> Result<()> {
        self.notify::<notification::DidSaveTextDocument>(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text,
        })
    }

    /// Notify document closed
    pub fn did_close(&self, uri: Url) -> Result<()> {
        self.notify::<notification::DidCloseTextDocument>(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        })
    }

    /// Request completion
    pub async fn completion(&self, uri: Url, position: Position) -> Result<Option<CompletionResponse>> {
        self.request::<request::Completion>(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        }).await
    }

    /// Request hover
    pub async fn hover(&self, uri: Url, position: Position) -> Result<Option<Hover>> {
        self.request::<request::HoverRequest>(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
        }).await
    }

    /// Request definition
    pub async fn definition(&self, uri: Url, position: Position) -> Result<Option<GotoDefinitionResponse>> {
        self.request::<request::GotoDefinition>(GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }).await
    }

    /// Request references
    pub async fn references(&self, uri: Url, position: Position) -> Result<Option<Vec<Location>>> {
        self.request::<request::References>(ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        }).await
    }

    /// Request code actions
    pub async fn code_actions(&self, uri: Url, range: Range, diagnostics: Vec<Diagnostic>) -> Result<Option<CodeActionResponse>> {
        self.request::<request::CodeActionRequest>(CodeActionParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            context: CodeActionContext {
                diagnostics,
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }).await
    }

    /// Format document
    pub async fn format(&self, uri: Url, options: FormattingOptions) -> Result<Option<Vec<TextEdit>>> {
        self.request::<request::Formatting>(DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri },
            options,
            work_done_progress_params: Default::default(),
        }).await
    }

    /// Get server capabilities
    pub fn capabilities(&self) -> Option<ServerCapabilities> {
        self.capabilities.read().clone()
    }

    /// Get current state
    pub fn state(&self) -> ServerState {
        *self.state.read()
    }

    /// Is server running?
    pub fn is_running(&self) -> bool {
        self.state() == ServerState::Running
    }
}
