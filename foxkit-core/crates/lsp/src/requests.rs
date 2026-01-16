//! LSP Request Builders
//!
//! Convenient builders for LSP requests.

use lsp_types::*;
use lsp_types::request::{GotoTypeDefinitionParams, GotoImplementationParams, GotoDeclarationParams};
use std::path::Path;

/// Request builder for LSP operations
pub struct LspRequestBuilder;

impl LspRequestBuilder {
    /// Build a goto definition request
    pub fn goto_definition(uri: Url, position: Position) -> GotoDefinitionParams {
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a hover request
    pub fn hover(uri: Url, position: Position) -> HoverParams {
        HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    /// Build a completion request
    pub fn completion(uri: Url, position: Position, trigger: Option<CompletionTriggerKind>) -> CompletionParams {
        CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: trigger.unwrap_or(CompletionTriggerKind::INVOKED),
                trigger_character: None,
            }),
        }
    }

    /// Build a completion request with trigger character
    pub fn completion_triggered(uri: Url, position: Position, trigger_char: char) -> CompletionParams {
        CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(trigger_char.to_string()),
            }),
        }
    }

    /// Build a references request
    pub fn references(uri: Url, position: Position, include_declaration: bool) -> ReferenceParams {
        ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration,
            },
        }
    }

    /// Build a document symbol request
    pub fn document_symbols(uri: Url) -> DocumentSymbolParams {
        DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a workspace symbol request
    pub fn workspace_symbols(query: &str) -> WorkspaceSymbolParams {
        WorkspaceSymbolParams {
            query: query.to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a code action request
    pub fn code_actions(uri: Url, range: Range, diagnostics: Vec<Diagnostic>) -> CodeActionParams {
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            context: CodeActionContext {
                diagnostics,
                only: None,
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a code action request for specific kinds
    pub fn code_actions_for_kinds(
        uri: Url,
        range: Range,
        diagnostics: Vec<Diagnostic>,
        kinds: Vec<CodeActionKind>,
    ) -> CodeActionParams {
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            context: CodeActionContext {
                diagnostics,
                only: Some(kinds),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a rename request
    pub fn rename(uri: Url, position: Position, new_name: &str) -> RenameParams {
        RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            new_name: new_name.to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    /// Build a prepare rename request
    pub fn prepare_rename(uri: Url, position: Position) -> TextDocumentPositionParams {
        TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position,
        }
    }

    /// Build a formatting request
    pub fn formatting(uri: Url, options: FormattingOptions) -> DocumentFormattingParams {
        DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri },
            options,
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    /// Build a range formatting request
    pub fn range_formatting(uri: Url, range: Range, options: FormattingOptions) -> DocumentRangeFormattingParams {
        DocumentRangeFormattingParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            options,
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    /// Build an on-type formatting request
    pub fn on_type_formatting(
        uri: Url,
        position: Position,
        ch: char,
        options: FormattingOptions,
    ) -> DocumentOnTypeFormattingParams {
        DocumentOnTypeFormattingParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            ch: ch.to_string(),
            options,
        }
    }

    /// Build a signature help request
    pub fn signature_help(uri: Url, position: Position) -> SignatureHelpParams {
        SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            context: None,
        }
    }

    /// Build a signature help request with context
    pub fn signature_help_retrigger(
        uri: Url,
        position: Position,
        active_signature_help: SignatureHelp,
        trigger_char: Option<char>,
    ) -> SignatureHelpParams {
        SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            context: Some(SignatureHelpContext {
                trigger_kind: if trigger_char.is_some() {
                    SignatureHelpTriggerKind::TRIGGER_CHARACTER
                } else {
                    SignatureHelpTriggerKind::CONTENT_CHANGE
                },
                trigger_character: trigger_char.map(|c| c.to_string()),
                is_retrigger: true,
                active_signature_help: Some(active_signature_help),
            }),
        }
    }

    /// Build a document highlight request
    pub fn document_highlight(uri: Url, position: Position) -> DocumentHighlightParams {
        DocumentHighlightParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a type definition request
    pub fn type_definition(uri: Url, position: Position) -> GotoTypeDefinitionParams {
        GotoTypeDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build an implementation request
    pub fn implementation(uri: Url, position: Position) -> GotoImplementationParams {
        GotoImplementationParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a declaration request
    pub fn declaration(uri: Url, position: Position) -> GotoDeclarationParams {
        GotoDeclarationParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a folding range request
    pub fn folding_range(uri: Url) -> FoldingRangeParams {
        FoldingRangeParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a selection range request
    pub fn selection_range(uri: Url, positions: Vec<Position>) -> SelectionRangeParams {
        SelectionRangeParams {
            text_document: TextDocumentIdentifier { uri },
            positions,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a document link request
    pub fn document_link(uri: Url) -> DocumentLinkParams {
        DocumentLinkParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a code lens request
    pub fn code_lens(uri: Url) -> CodeLensParams {
        CodeLensParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build an inlay hint request
    pub fn inlay_hints(uri: Url, range: Range) -> InlayHintParams {
        InlayHintParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    /// Build a semantic tokens full request
    pub fn semantic_tokens_full(uri: Url) -> SemanticTokensParams {
        SemanticTokensParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a semantic tokens range request
    pub fn semantic_tokens_range(uri: Url, range: Range) -> SemanticTokensRangeParams {
        SemanticTokensRangeParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a call hierarchy prepare request
    pub fn call_hierarchy_prepare(uri: Url, position: Position) -> CallHierarchyPrepareParams {
        CallHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    /// Build a call hierarchy incoming calls request
    pub fn call_hierarchy_incoming(item: CallHierarchyItem) -> CallHierarchyIncomingCallsParams {
        CallHierarchyIncomingCallsParams {
            item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a call hierarchy outgoing calls request
    pub fn call_hierarchy_outgoing(item: CallHierarchyItem) -> CallHierarchyOutgoingCallsParams {
        CallHierarchyOutgoingCallsParams {
            item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a type hierarchy prepare request
    pub fn type_hierarchy_prepare(uri: Url, position: Position) -> TypeHierarchyPrepareParams {
        TypeHierarchyPrepareParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    /// Build a type hierarchy supertypes request
    pub fn type_hierarchy_supertypes(item: TypeHierarchyItem) -> TypeHierarchySupertypesParams {
        TypeHierarchySupertypesParams {
            item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a type hierarchy subtypes request
    pub fn type_hierarchy_subtypes(item: TypeHierarchyItem) -> TypeHierarchySubtypesParams {
        TypeHierarchySubtypesParams {
            item,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        }
    }

    /// Build a linked editing range request
    pub fn linked_editing_range(uri: Url, position: Position) -> LinkedEditingRangeParams {
        LinkedEditingRangeParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    /// Build an execute command request
    pub fn execute_command(command: &str, arguments: Option<Vec<serde_json::Value>>) -> ExecuteCommandParams {
        ExecuteCommandParams {
            command: command.to_string(),
            arguments: arguments.unwrap_or_default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }
}

/// Notification builder for LSP operations
pub struct LspNotificationBuilder;

impl LspNotificationBuilder {
    /// Build a didOpen notification
    pub fn did_open(uri: Url, language_id: &str, version: i32, text: String) -> DidOpenTextDocumentParams {
        DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: language_id.to_string(),
                version,
                text,
            },
        }
    }

    /// Build a didClose notification
    pub fn did_close(uri: Url) -> DidCloseTextDocumentParams {
        DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        }
    }

    /// Build a didChange notification (full sync)
    pub fn did_change_full(uri: Url, version: i32, text: String) -> DidChangeTextDocumentParams {
        DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text,
            }],
        }
    }

    /// Build a didChange notification (incremental sync)
    pub fn did_change_incremental(
        uri: Url,
        version: i32,
        changes: Vec<(Range, String)>,
    ) -> DidChangeTextDocumentParams {
        DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes: changes
                .into_iter()
                .map(|(range, text)| TextDocumentContentChangeEvent {
                    range: Some(range),
                    range_length: None,
                    text,
                })
                .collect(),
        }
    }

    /// Build a didSave notification
    pub fn did_save(uri: Url, text: Option<String>) -> DidSaveTextDocumentParams {
        DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text,
        }
    }

    /// Build a willSave notification
    pub fn will_save(uri: Url, reason: TextDocumentSaveReason) -> WillSaveTextDocumentParams {
        WillSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            reason,
        }
    }

    /// Build a didChangeConfiguration notification
    pub fn did_change_configuration(settings: serde_json::Value) -> DidChangeConfigurationParams {
        DidChangeConfigurationParams { settings }
    }

    /// Build a didChangeWatchedFiles notification
    pub fn did_change_watched_files(changes: Vec<(Url, FileChangeType)>) -> DidChangeWatchedFilesParams {
        DidChangeWatchedFilesParams {
            changes: changes
                .into_iter()
                .map(|(uri, typ)| FileEvent { uri, typ })
                .collect(),
        }
    }
}

/// URI utilities
pub fn file_uri(path: impl AsRef<Path>) -> Url {
    Url::from_file_path(path.as_ref()).expect("Invalid file path")
}

/// Position utilities
pub fn pos(line: u32, character: u32) -> Position {
    Position { line, character }
}

/// Range utilities
pub fn range(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> Range {
    Range {
        start: pos(start_line, start_char),
        end: pos(end_line, end_char),
    }
}
