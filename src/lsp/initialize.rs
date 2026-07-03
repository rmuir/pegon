use core::ops::Not as _;

use anyhow::Result;
use gen_lsp_types::{
    ChangeNotifications, CodeActionKind, CodeActionOptions, CodeActionProvider,
    CodeActionRegistrationOptions, CodeActionRequest, DefinitionOptions, DefinitionProvider,
    DefinitionRegistrationOptions, DefinitionRequest, DiagnosticOptions, DiagnosticProvider,
    DiagnosticRegistrationOptions, DidChangeTextDocumentNotification,
    DidCloseTextDocumentNotification, DidOpenTextDocumentNotification, DocumentDiagnosticRequest,
    DocumentFilter, DocumentHighlightOptions, DocumentHighlightProvider,
    DocumentHighlightRegistrationOptions, DocumentHighlightRequest, DocumentSymbolOptions,
    DocumentSymbolProvider, DocumentSymbolRequest, FoldingRangeOptions, FoldingRangeProvider,
    FoldingRangeRegistrationOptions, FoldingRangeRequest, Full, HoverOptions, HoverProvider,
    HoverRegistrationOptions, HoverRequest, InitializeResult, InlayHintOptions, InlayHintProvider,
    InlayHintRegistrationOptions, InlayHintRequest, Notification as _, Registration, Request as _,
    SelectionRangeOptions, SelectionRangeProvider, SelectionRangeRegistrationOptions,
    SelectionRangeRequest, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensOptionsRange,
    SemanticTokensProvider, SemanticTokensRegistrationOptions, SemanticTokensRequest,
    ServerCapabilities, ServerInfo, StaticRegistrationOptions,
    TextDocumentChangeRegistrationOptions, TextDocumentFilter, TextDocumentFilterLanguage,
    TextDocumentRegistrationOptions, TextDocumentSync, TextDocumentSyncKind,
    TextDocumentSyncOptions, WorkDoneProgressOptions, WorkspaceFoldersServerCapabilities,
    WorkspaceOptions,
};

use super::client::Client;

/// Initializes a new server
pub fn init(client: &Client) -> Result<(InitializeResult, Vec<Registration>)> {
    let text_document_registration_options = TextDocumentRegistrationOptions {
        document_selector: Some(vec![DocumentFilter::TextDocumentFilter(
            TextDocumentFilter::Language(TextDocumentFilterLanguage {
                language: "java".into(),
                scheme: None,
                pattern: None,
            }),
        )]),
    };
    let work_done_progress_options = WorkDoneProgressOptions::default();
    let diagnostic_options = DiagnosticRegistrationOptions {
        diagnostic_options: DiagnosticOptions {
            identifier: Some(env!("CARGO_PKG_NAME").into()),
            ..DiagnosticOptions::default()
        },
        text_document_registration_options: text_document_registration_options.clone(),
        ..DiagnosticRegistrationOptions::default()
    };
    let code_action_options = CodeActionOptions {
        code_action_kinds: Some(vec![
            CodeActionKind::QuickFix,
            CodeActionKind::new("source.organizeImports"),
        ]),
        resolve_provider: Some(true),
        ..CodeActionOptions::default()
    };
    let definition_options = DefinitionOptions {
        work_done_progress_options,
    };
    let document_highlight_options = DocumentHighlightOptions {
        work_done_progress_options,
    };
    let document_symbol_options = DocumentSymbolOptions {
        label: None,
        work_done_progress_options,
    };
    let folding_range_options = FoldingRangeRegistrationOptions {
        folding_range_options: FoldingRangeOptions::new(WorkDoneProgressOptions::default()),
        static_registration_options: StaticRegistrationOptions {
            id: Some(FoldingRangeRequest::METHOD.into()),
        },
        text_document_registration_options: text_document_registration_options.clone(),
    };
    let hover_options = HoverOptions {
        work_done_progress_options,
    };
    let inlay_hint_options = InlayHintRegistrationOptions {
        inlay_hint_options: InlayHintOptions {
            resolve_provider: Some(true),
            work_done_progress_options,
        },
        static_registration_options: StaticRegistrationOptions {
            id: Some(InlayHintRequest::METHOD.into()),
        },
        text_document_registration_options: text_document_registration_options.clone(),
    };
    let selection_range_options = SelectionRangeRegistrationOptions {
        selection_range_options: SelectionRangeOptions::default(),
        static_registration_options: StaticRegistrationOptions {
            id: Some(SelectionRangeRequest::METHOD.into()),
        },
        text_document_registration_options: text_document_registration_options.clone(),
    };
    let semantic_tokens_options = SemanticTokensRegistrationOptions {
        semantic_tokens_options: SemanticTokensOptions {
            legend: SemanticTokensLegend {
                token_types: super::SEMANTIC_TOKEN_TYPES
                    .into_iter()
                    .map(String::from)
                    .collect(),
                token_modifiers: super::SEMANTIC_TOKEN_MODIFIERS
                    .into_iter()
                    .map(String::from)
                    .collect(),
            },
            range: Some(SemanticTokensOptionsRange::Bool(true)),
            full: Some(Full::Bool(true)), // TODO: delta?
            work_done_progress_options,
        },
        static_registration_options: StaticRegistrationOptions {
            id: Some(SemanticTokensRequest::METHOD.into()),
        },
        text_document_registration_options: text_document_registration_options.clone(),
    };
    // if the client supports dynamic registration of the capability, then we use that.
    // it makes the code very confusing, but this is just the pain of dynamic registration.
    // it allows pushing a filter for "java" language documents to the client, to avoid waste.
    // otherwise, advertise it statically, but not both! (see spec)
    let result = InitializeResult {
        server_info: Some(ServerInfo {
            name: env!("CARGO_PKG_NAME").into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        }),
        capabilities: ServerCapabilities {
            code_action_provider: client.registers_code_actions().not().then_some(
                CodeActionProvider::CodeActionOptions(code_action_options.clone()),
            ),
            definition_provider: client
                .registers_definition()
                .not()
                .then_some(DefinitionProvider::DefinitionOptions(definition_options)),
            diagnostic_provider: client.registers_diagnostics().not().then_some(
                DiagnosticProvider::DiagnosticRegistrationOptions(diagnostic_options.clone()),
            ),
            document_highlight_provider: client.registers_document_highlight().not().then_some(
                DocumentHighlightProvider::DocumentHighlightOptions(document_highlight_options),
            ),
            document_symbol_provider: client.registers_document_symbols().not().then_some(
                DocumentSymbolProvider::DocumentSymbolOptions(document_symbol_options.clone()),
            ),
            folding_range_provider: client.registers_folding_range().not().then_some(
                FoldingRangeProvider::FoldingRangeRegistrationOptions(
                    folding_range_options.clone(),
                ),
            ),
            hover_provider: client
                .registers_hover()
                .not()
                .then_some(HoverProvider::HoverOptions(hover_options)),
            inlay_hint_provider: client.registers_inlay_hints().not().then_some(
                InlayHintProvider::InlayHintRegistrationOptions(inlay_hint_options.clone()),
            ),
            position_encoding: Some(client.negotiated_encoding()),
            selection_range_provider: client.registers_selection_range().not().then_some(
                SelectionRangeProvider::SelectionRangeRegistrationOptions(
                    selection_range_options.clone(),
                ),
            ),
            semantic_tokens_provider: client.registers_semantic_tokens().not().then_some(
                SemanticTokensProvider::SemanticTokensRegistrationOptions(
                    semantic_tokens_options.clone(),
                ),
            ),
            text_document_sync: client
                .registers_sync()
                .not()
                .then_some(TextDocumentSync::Options(TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::Incremental),
                    ..TextDocumentSyncOptions::default()
                })),
            // we don't care about classpaths or anything on disk, so advertise
            // the workspace support for better client-side reuse of the server.
            workspace: Some(WorkspaceOptions {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(ChangeNotifications::Bool(false)),
                }),
                file_operations: None,
                text_document_content: None, // TODO!
            }),
            ..ServerCapabilities::default()
        },
    };
    let mut registrations: Vec<Registration> = Vec::with_capacity(12);
    if client.registers_sync() {
        registrations.push(Registration {
            id: DidOpenTextDocumentNotification::METHOD.into(),
            method: DidOpenTextDocumentNotification::METHOD.into(),
            register_options: Some(serde_json::to_value(
                text_document_registration_options.clone(),
            )?),
        });
        registrations.push(Registration {
            id: DidChangeTextDocumentNotification::METHOD.into(),
            method: DidChangeTextDocumentNotification::METHOD.into(),
            register_options: Some(serde_json::to_value(
                TextDocumentChangeRegistrationOptions {
                    text_document_registration_options: text_document_registration_options.clone(),
                    sync_kind: TextDocumentSyncKind::Incremental,
                },
            )?),
        });
        registrations.push(Registration {
            id: DidCloseTextDocumentNotification::METHOD.into(),
            method: DidCloseTextDocumentNotification::METHOD.into(),
            register_options: Some(serde_json::to_value(
                text_document_registration_options.clone(),
            )?),
        });
    }
    if client.registers_code_actions() {
        registrations.push(Registration {
            id: CodeActionRequest::METHOD.into(),
            method: CodeActionRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(CodeActionRegistrationOptions {
                text_document_registration_options: text_document_registration_options.clone(),
                code_action_options,
            })?),
        });
    }
    if client.registers_definition() {
        registrations.push(Registration {
            id: DefinitionRequest::METHOD.into(),
            method: DefinitionRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(DefinitionRegistrationOptions {
                text_document_registration_options: text_document_registration_options.clone(),
                definition_options,
            })?),
        });
    }
    if client.registers_diagnostics() {
        registrations.push(Registration {
            id: DocumentDiagnosticRequest::METHOD.into(),
            method: DocumentDiagnosticRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(diagnostic_options)?),
        });
    }
    if client.registers_document_highlight() {
        registrations.push(Registration {
            id: DocumentHighlightRequest::METHOD.into(),
            method: DocumentHighlightRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(
                DocumentHighlightRegistrationOptions {
                    text_document_registration_options: text_document_registration_options.clone(),
                    document_highlight_options,
                },
            )?),
        });
    }
    if client.registers_document_symbols() {
        registrations.push(Registration {
            id: DocumentSymbolRequest::METHOD.into(),
            method: DocumentSymbolRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(document_symbol_options)?),
        });
    }
    if client.registers_folding_range() {
        registrations.push(Registration {
            id: FoldingRangeRequest::METHOD.into(),
            method: FoldingRangeRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(folding_range_options)?),
        });
    }
    if client.registers_hover() {
        registrations.push(Registration {
            id: HoverRequest::METHOD.into(),
            method: HoverRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(HoverRegistrationOptions {
                text_document_registration_options,
                hover_options,
            })?),
        });
    }
    if client.registers_inlay_hints() {
        registrations.push(Registration {
            id: InlayHintRequest::METHOD.into(),
            method: InlayHintRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(inlay_hint_options)?),
        });
    }
    if client.registers_selection_range() {
        registrations.push(Registration {
            id: SelectionRangeRequest::METHOD.into(),
            method: SelectionRangeRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(selection_range_options)?),
        });
    }
    if client.registers_semantic_tokens() {
        registrations.push(Registration {
            id: SemanticTokensRequest::METHOD.into(),
            method: "textDocument/semanticTokens".into(), // must be this ID
            register_options: Some(serde_json::to_value(semantic_tokens_options)?),
        });
    }
    Ok((result, registrations))
}

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        ClientCapabilities, DiagnosticClientCapabilities, DidChangeTextDocumentNotification,
        DidCloseTextDocumentNotification, DidOpenTextDocumentNotification,
        DocumentDiagnosticRequest, GeneralClientCapabilities, InitializeParams, Notification as _,
        PositionEncodingKind, Request as _, TextDocumentClientCapabilities,
        TextDocumentSyncClientCapabilities,
    };

    use crate::lsp::test_client::TestClient;

    /// default to UTF-16 according to the spec
    #[test]
    fn encoding_default() {
        let client = TestClient::new(InitializeParams::default());
        assert_eq!(
            Some(PositionEncodingKind::UTF16),
            client.init_response().capabilities.position_encoding
        );
    }

    /// pick the client's first offered encoding.
    /// most likely it is the most performant for that client
    #[test]
    fn encoding_preferred() {
        let client = TestClient::new(InitializeParams {
            capabilities: ClientCapabilities {
                general: Some(GeneralClientCapabilities {
                    position_encodings: Some(vec![
                        PositionEncodingKind::UTF8,
                        PositionEncodingKind::UTF16,
                    ]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(
            Some(PositionEncodingKind::UTF8),
            client.init_response().capabilities.position_encoding
        );
    }

    /// check all encoding kinds can be negotiated
    #[test]
    fn negotiate_encodings() {
        for encoding in [
            PositionEncodingKind::UTF8,
            PositionEncodingKind::UTF16,
            PositionEncodingKind::UTF32,
        ] {
            let client = TestClient::new(InitializeParams {
                capabilities: ClientCapabilities {
                    general: Some(GeneralClientCapabilities {
                        position_encodings: Some(vec![encoding.clone()]),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            });
            assert_eq!(
                Some(encoding),
                client.init_response().capabilities.position_encoding
            );
        }
    }

    #[test]
    fn dynamic_registration() {
        let client = TestClient::new(InitializeParams {
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    synchronization: Some(TextDocumentSyncClientCapabilities {
                        dynamic_registration: Some(true),
                        ..Default::default()
                    }),
                    diagnostic: Some(DiagnosticClientCapabilities {
                        dynamic_registration: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        });
        let result = client.registrations();
        assert!(result.is_some());
        let params = result.unwrap().registrations;
        assert_eq!(params.len(), 4);
        assert_eq!(
            params[0].method,
            DidOpenTextDocumentNotification::METHOD.to_string()
        );
        assert_eq!(
            params[1].method,
            DidChangeTextDocumentNotification::METHOD.to_string()
        );
        assert_eq!(
            params[2].method,
            DidCloseTextDocumentNotification::METHOD.to_string()
        );
        assert_eq!(
            params[3].method,
            DocumentDiagnosticRequest::METHOD.to_string()
        );
    }
}
