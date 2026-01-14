//! LSP Client Capabilities
//!
//! Comprehensive LSP client capability negotiation.

use lsp_types::*;
use std::collections::HashMap;

/// Build comprehensive client capabilities
pub fn build_client_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        workspace: Some(build_workspace_capabilities()),
        text_document: Some(build_text_document_capabilities()),
        window: Some(build_window_capabilities()),
        general: Some(build_general_capabilities()),
        experimental: None,
    }
}

fn build_workspace_capabilities() -> WorkspaceClientCapabilities {
    WorkspaceClientCapabilities {
        apply_edit: Some(true),
        workspace_edit: Some(WorkspaceEditClientCapabilities {
            document_changes: Some(true),
            resource_operations: Some(vec![
                ResourceOperationKind::Create,
                ResourceOperationKind::Rename,
                ResourceOperationKind::Delete,
            ]),
            failure_handling: Some(FailureHandlingKind::Undo),
            normalizes_line_endings: Some(true),
            change_annotation_support: Some(ChangeAnnotationWorkspaceEditClientCapabilities {
                groups_on_label: Some(true),
            }),
        }),
        did_change_configuration: Some(DynamicRegistrationClientCapabilities {
            dynamic_registration: Some(true),
        }),
        did_change_watched_files: Some(DidChangeWatchedFilesClientCapabilities {
            dynamic_registration: Some(true),
            relative_pattern_support: Some(true),
        }),
        symbol: Some(WorkspaceSymbolClientCapabilities {
            dynamic_registration: Some(true),
            symbol_kind: Some(SymbolKindCapability {
                value_set: Some(all_symbol_kinds()),
            }),
            tag_support: Some(TagSupport {
                value_set: vec![SymbolTag::DEPRECATED],
            }),
            resolve_support: Some(WorkspaceSymbolResolveSupportCapability {
                properties: vec!["location.range".to_string()],
            }),
        }),
        execute_command: Some(DynamicRegistrationClientCapabilities {
            dynamic_registration: Some(true),
        }),
        workspace_folders: Some(true),
        configuration: Some(true),
        semantic_tokens: Some(SemanticTokensWorkspaceClientCapabilities {
            refresh_support: Some(true),
        }),
        code_lens: Some(CodeLensWorkspaceClientCapabilities {
            refresh_support: Some(true),
        }),
        file_operations: Some(FileOperationClientCapabilities {
            dynamic_registration: Some(true),
            did_create: Some(true),
            will_create: Some(true),
            did_rename: Some(true),
            will_rename: Some(true),
            did_delete: Some(true),
            will_delete: Some(true),
        }),
        inline_value: Some(InlineValueWorkspaceClientCapabilities {
            refresh_support: Some(true),
        }),
        inlay_hint: Some(InlayHintWorkspaceClientCapabilities {
            refresh_support: Some(true),
        }),
        diagnostics: Some(DiagnosticWorkspaceClientCapabilities {
            refresh_support: Some(true),
        }),
    }
}

fn build_text_document_capabilities() -> TextDocumentClientCapabilities {
    TextDocumentClientCapabilities {
        synchronization: Some(TextDocumentSyncClientCapabilities {
            dynamic_registration: Some(true),
            will_save: Some(true),
            will_save_wait_until: Some(true),
            did_save: Some(true),
        }),
        completion: Some(CompletionClientCapabilities {
            dynamic_registration: Some(true),
            completion_item: Some(CompletionItemCapability {
                snippet_support: Some(true),
                commit_characters_support: Some(true),
                documentation_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
                deprecated_support: Some(true),
                preselect_support: Some(true),
                tag_support: Some(TagSupport {
                    value_set: vec![CompletionItemTag::DEPRECATED],
                }),
                insert_replace_support: Some(true),
                resolve_support: Some(CompletionItemCapabilityResolveSupport {
                    properties: vec![
                        "documentation".to_string(),
                        "detail".to_string(),
                        "additionalTextEdits".to_string(),
                    ],
                }),
                insert_text_mode_support: Some(InsertTextModeSupport {
                    value_set: vec![InsertTextMode::AS_IS, InsertTextMode::ADJUST_INDENTATION],
                }),
                label_details_support: Some(true),
            }),
            completion_item_kind: Some(CompletionItemKindCapability {
                value_set: Some(all_completion_item_kinds()),
            }),
            context_support: Some(true),
            insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
            completion_list: Some(CompletionListCapability {
                item_defaults: Some(vec![
                    "commitCharacters".to_string(),
                    "editRange".to_string(),
                    "insertTextFormat".to_string(),
                    "insertTextMode".to_string(),
                    "data".to_string(),
                ]),
            }),
        }),
        hover: Some(HoverClientCapabilities {
            dynamic_registration: Some(true),
            content_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
        }),
        signature_help: Some(SignatureHelpClientCapabilities {
            dynamic_registration: Some(true),
            signature_information: Some(SignatureInformationSettings {
                documentation_format: Some(vec![MarkupKind::Markdown, MarkupKind::PlainText]),
                parameter_information: Some(ParameterInformationSettings {
                    label_offset_support: Some(true),
                }),
                active_parameter_support: Some(true),
            }),
            context_support: Some(true),
        }),
        references: Some(DynamicRegistrationClientCapabilities {
            dynamic_registration: Some(true),
        }),
        document_highlight: Some(DynamicRegistrationClientCapabilities {
            dynamic_registration: Some(true),
        }),
        document_symbol: Some(DocumentSymbolClientCapabilities {
            dynamic_registration: Some(true),
            symbol_kind: Some(SymbolKindCapability {
                value_set: Some(all_symbol_kinds()),
            }),
            hierarchical_document_symbol_support: Some(true),
            tag_support: Some(TagSupport {
                value_set: vec![SymbolTag::DEPRECATED],
            }),
            label_support: Some(true),
        }),
        formatting: Some(DynamicRegistrationClientCapabilities {
            dynamic_registration: Some(true),
        }),
        range_formatting: Some(DynamicRegistrationClientCapabilities {
            dynamic_registration: Some(true),
        }),
        on_type_formatting: Some(DynamicRegistrationClientCapabilities {
            dynamic_registration: Some(true),
        }),
        declaration: Some(GotoCapability {
            dynamic_registration: Some(true),
            link_support: Some(true),
        }),
        definition: Some(GotoCapability {
            dynamic_registration: Some(true),
            link_support: Some(true),
        }),
        type_definition: Some(GotoCapability {
            dynamic_registration: Some(true),
            link_support: Some(true),
        }),
        implementation: Some(GotoCapability {
            dynamic_registration: Some(true),
            link_support: Some(true),
        }),
        code_action: Some(CodeActionClientCapabilities {
            dynamic_registration: Some(true),
            code_action_literal_support: Some(CodeActionLiteralSupport {
                code_action_kind: CodeActionKindLiteralSupport {
                    value_set: all_code_action_kinds(),
                },
            }),
            is_preferred_support: Some(true),
            disabled_support: Some(true),
            data_support: Some(true),
            resolve_support: Some(CodeActionCapabilityResolveSupport {
                properties: vec!["edit".to_string(), "command".to_string()],
            }),
            honors_change_annotations: Some(true),
        }),
        code_lens: Some(CodeLensClientCapabilities {
            dynamic_registration: Some(true),
        }),
        document_link: Some(DocumentLinkClientCapabilities {
            dynamic_registration: Some(true),
            tooltip_support: Some(true),
        }),
        color_provider: Some(DynamicRegistrationClientCapabilities {
            dynamic_registration: Some(true),
        }),
        rename: Some(RenameClientCapabilities {
            dynamic_registration: Some(true),
            prepare_support: Some(true),
            prepare_support_default_behavior: Some(PrepareSupportDefaultBehavior::IDENTIFIER),
            honors_change_annotations: Some(true),
        }),
        publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
            related_information: Some(true),
            tag_support: Some(TagSupport {
                value_set: vec![DiagnosticTag::UNNECESSARY, DiagnosticTag::DEPRECATED],
            }),
            version_support: Some(true),
            code_description_support: Some(true),
            data_support: Some(true),
        }),
        folding_range: Some(FoldingRangeClientCapabilities {
            dynamic_registration: Some(true),
            range_limit: Some(5000),
            line_folding_only: Some(false),
            folding_range_kind: Some(FoldingRangeKindCapability {
                value_set: Some(vec![
                    FoldingRangeKind::Comment,
                    FoldingRangeKind::Imports,
                    FoldingRangeKind::Region,
                ]),
            }),
            folding_range: Some(FoldingRangeCapability {
                collapsed_text: Some(true),
            }),
        }),
        selection_range: Some(SelectionRangeClientCapabilities {
            dynamic_registration: Some(true),
        }),
        linked_editing_range: Some(LinkedEditingRangeClientCapabilities {
            dynamic_registration: Some(true),
        }),
        call_hierarchy: Some(CallHierarchyClientCapabilities {
            dynamic_registration: Some(true),
        }),
        semantic_tokens: Some(SemanticTokensClientCapabilities {
            dynamic_registration: Some(true),
            requests: SemanticTokensClientCapabilitiesRequests {
                range: Some(true),
                full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
            },
            token_types: semantic_token_types(),
            token_modifiers: semantic_token_modifiers(),
            formats: vec![TokenFormat::RELATIVE],
            overlapping_token_support: Some(true),
            multiline_token_support: Some(true),
            server_cancel_support: Some(true),
            augments_syntax_tokens: Some(true),
        }),
        moniker: Some(MonikerClientCapabilities {
            dynamic_registration: Some(true),
        }),
        type_hierarchy: Some(TypeHierarchyClientCapabilities {
            dynamic_registration: Some(true),
        }),
        inline_value: Some(InlineValueClientCapabilities {
            dynamic_registration: Some(true),
        }),
        inlay_hint: Some(InlayHintClientCapabilities {
            dynamic_registration: Some(true),
            resolve_support: Some(InlayHintResolveClientCapabilities {
                properties: vec![
                    "tooltip".to_string(),
                    "textEdits".to_string(),
                    "label.tooltip".to_string(),
                    "label.location".to_string(),
                    "label.command".to_string(),
                ],
            }),
        }),
        diagnostic: Some(DiagnosticClientCapabilities {
            dynamic_registration: Some(true),
            related_document_support: Some(true),
        }),
    }
}

fn build_window_capabilities() -> WindowClientCapabilities {
    WindowClientCapabilities {
        work_done_progress: Some(true),
        show_message: Some(ShowMessageRequestClientCapabilities {
            message_action_item: Some(MessageActionItemCapabilities {
                additional_properties_support: Some(true),
            }),
        }),
        show_document: Some(ShowDocumentClientCapabilities {
            support: true,
        }),
    }
}

fn build_general_capabilities() -> GeneralClientCapabilities {
    GeneralClientCapabilities {
        stale_request_support: Some(StaleRequestSupportClientCapabilities {
            cancel: true,
            retry_on_content_modified: vec![
                "textDocument/semanticTokens/full".to_string(),
                "textDocument/semanticTokens/range".to_string(),
                "textDocument/semanticTokens/full/delta".to_string(),
            ],
        }),
        regular_expressions: Some(RegularExpressionsClientCapabilities {
            engine: "ECMAScript".to_string(),
            version: Some("ES2020".to_string()),
        }),
        markdown: Some(MarkdownClientCapabilities {
            parser: "marked".to_string(),
            version: Some("1.1.0".to_string()),
            allowed_tags: None,
        }),
        position_encodings: Some(vec![
            PositionEncodingKind::UTF32,
            PositionEncodingKind::UTF16,
        ]),
    }
}

fn all_symbol_kinds() -> Vec<SymbolKind> {
    vec![
        SymbolKind::FILE,
        SymbolKind::MODULE,
        SymbolKind::NAMESPACE,
        SymbolKind::PACKAGE,
        SymbolKind::CLASS,
        SymbolKind::METHOD,
        SymbolKind::PROPERTY,
        SymbolKind::FIELD,
        SymbolKind::CONSTRUCTOR,
        SymbolKind::ENUM,
        SymbolKind::INTERFACE,
        SymbolKind::FUNCTION,
        SymbolKind::VARIABLE,
        SymbolKind::CONSTANT,
        SymbolKind::STRING,
        SymbolKind::NUMBER,
        SymbolKind::BOOLEAN,
        SymbolKind::ARRAY,
        SymbolKind::OBJECT,
        SymbolKind::KEY,
        SymbolKind::NULL,
        SymbolKind::ENUM_MEMBER,
        SymbolKind::STRUCT,
        SymbolKind::EVENT,
        SymbolKind::OPERATOR,
        SymbolKind::TYPE_PARAMETER,
    ]
}

fn all_completion_item_kinds() -> Vec<CompletionItemKind> {
    vec![
        CompletionItemKind::TEXT,
        CompletionItemKind::METHOD,
        CompletionItemKind::FUNCTION,
        CompletionItemKind::CONSTRUCTOR,
        CompletionItemKind::FIELD,
        CompletionItemKind::VARIABLE,
        CompletionItemKind::CLASS,
        CompletionItemKind::INTERFACE,
        CompletionItemKind::MODULE,
        CompletionItemKind::PROPERTY,
        CompletionItemKind::UNIT,
        CompletionItemKind::VALUE,
        CompletionItemKind::ENUM,
        CompletionItemKind::KEYWORD,
        CompletionItemKind::SNIPPET,
        CompletionItemKind::COLOR,
        CompletionItemKind::FILE,
        CompletionItemKind::REFERENCE,
        CompletionItemKind::FOLDER,
        CompletionItemKind::ENUM_MEMBER,
        CompletionItemKind::CONSTANT,
        CompletionItemKind::STRUCT,
        CompletionItemKind::EVENT,
        CompletionItemKind::OPERATOR,
        CompletionItemKind::TYPE_PARAMETER,
    ]
}

fn all_code_action_kinds() -> Vec<String> {
    vec![
        "quickfix".to_string(),
        "refactor".to_string(),
        "refactor.extract".to_string(),
        "refactor.inline".to_string(),
        "refactor.rewrite".to_string(),
        "source".to_string(),
        "source.organizeImports".to_string(),
        "source.fixAll".to_string(),
    ]
}

fn semantic_token_types() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::NAMESPACE,
        SemanticTokenType::TYPE,
        SemanticTokenType::CLASS,
        SemanticTokenType::ENUM,
        SemanticTokenType::INTERFACE,
        SemanticTokenType::STRUCT,
        SemanticTokenType::TYPE_PARAMETER,
        SemanticTokenType::PARAMETER,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::PROPERTY,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::EVENT,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::METHOD,
        SemanticTokenType::MACRO,
        SemanticTokenType::KEYWORD,
        SemanticTokenType::MODIFIER,
        SemanticTokenType::COMMENT,
        SemanticTokenType::STRING,
        SemanticTokenType::NUMBER,
        SemanticTokenType::REGEXP,
        SemanticTokenType::OPERATOR,
        SemanticTokenType::DECORATOR,
    ]
}

fn semantic_token_modifiers() -> Vec<SemanticTokenModifier> {
    vec![
        SemanticTokenModifier::DECLARATION,
        SemanticTokenModifier::DEFINITION,
        SemanticTokenModifier::READONLY,
        SemanticTokenModifier::STATIC,
        SemanticTokenModifier::DEPRECATED,
        SemanticTokenModifier::ABSTRACT,
        SemanticTokenModifier::ASYNC,
        SemanticTokenModifier::MODIFICATION,
        SemanticTokenModifier::DOCUMENTATION,
        SemanticTokenModifier::DEFAULT_LIBRARY,
    ]
}

/// Server capability analyzer
pub struct ServerCapabilityAnalyzer {
    capabilities: ServerCapabilities,
}

impl ServerCapabilityAnalyzer {
    pub fn new(capabilities: ServerCapabilities) -> Self {
        Self { capabilities }
    }

    pub fn supports_completion(&self) -> bool {
        self.capabilities.completion_provider.is_some()
    }

    pub fn supports_hover(&self) -> bool {
        matches!(
            &self.capabilities.hover_provider,
            Some(HoverProviderCapability::Simple(true) | HoverProviderCapability::Options(_))
        )
    }

    pub fn supports_definition(&self) -> bool {
        matches!(
            &self.capabilities.definition_provider,
            Some(OneOf::Left(true) | OneOf::Right(_))
        )
    }

    pub fn supports_references(&self) -> bool {
        matches!(
            &self.capabilities.references_provider,
            Some(OneOf::Left(true) | OneOf::Right(_))
        )
    }

    pub fn supports_document_symbol(&self) -> bool {
        matches!(
            &self.capabilities.document_symbol_provider,
            Some(OneOf::Left(true) | OneOf::Right(_))
        )
    }

    pub fn supports_workspace_symbol(&self) -> bool {
        matches!(
            &self.capabilities.workspace_symbol_provider,
            Some(OneOf::Left(true) | OneOf::Right(_))
        )
    }

    pub fn supports_code_action(&self) -> bool {
        matches!(
            &self.capabilities.code_action_provider,
            Some(CodeActionProviderCapability::Simple(true) | CodeActionProviderCapability::Options(_))
        )
    }

    pub fn supports_rename(&self) -> bool {
        matches!(
            &self.capabilities.rename_provider,
            Some(OneOf::Left(true) | OneOf::Right(_))
        )
    }

    pub fn supports_formatting(&self) -> bool {
        matches!(
            &self.capabilities.document_formatting_provider,
            Some(OneOf::Left(true) | OneOf::Right(_))
        )
    }

    pub fn supports_inlay_hints(&self) -> bool {
        matches!(
            &self.capabilities.inlay_hint_provider,
            Some(OneOf::Left(InlayHintServerCapabilities::Options(_)) | OneOf::Right(_))
        )
    }

    pub fn supports_semantic_tokens(&self) -> bool {
        self.capabilities.semantic_tokens_provider.is_some()
    }

    pub fn supports_call_hierarchy(&self) -> bool {
        matches!(
            &self.capabilities.call_hierarchy_provider,
            Some(CallHierarchyServerCapability::Simple(true) | CallHierarchyServerCapability::Options(_))
        )
    }

    pub fn supports_type_hierarchy(&self) -> bool {
        matches!(
            &self.capabilities.type_hierarchy_provider,
            Some(TypeHierarchyServerCapability::Simple(true) | TypeHierarchyServerCapability::Options(_))
        )
    }

    pub fn get_completion_trigger_characters(&self) -> Vec<String> {
        self.capabilities
            .completion_provider
            .as_ref()
            .and_then(|p| p.trigger_characters.clone())
            .unwrap_or_default()
    }

    pub fn get_signature_help_triggers(&self) -> Vec<String> {
        self.capabilities
            .signature_help_provider
            .as_ref()
            .and_then(|p| p.trigger_characters.clone())
            .unwrap_or_default()
    }

    /// Get a summary of supported features
    pub fn feature_summary(&self) -> HashMap<&'static str, bool> {
        let mut features = HashMap::new();
        features.insert("completion", self.supports_completion());
        features.insert("hover", self.supports_hover());
        features.insert("definition", self.supports_definition());
        features.insert("references", self.supports_references());
        features.insert("documentSymbol", self.supports_document_symbol());
        features.insert("workspaceSymbol", self.supports_workspace_symbol());
        features.insert("codeAction", self.supports_code_action());
        features.insert("rename", self.supports_rename());
        features.insert("formatting", self.supports_formatting());
        features.insert("inlayHints", self.supports_inlay_hints());
        features.insert("semanticTokens", self.supports_semantic_tokens());
        features.insert("callHierarchy", self.supports_call_hierarchy());
        features.insert("typeHierarchy", self.supports_type_hierarchy());
        features
    }
}
