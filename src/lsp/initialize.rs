use anyhow::Result;
use gen_lsp_types::{
    ChangeNotifications, CodeActionKind, CodeActionOptions, CodeActionProvider,
    CodeActionRegistrationOptions, CodeActionRequest, DiagnosticOptions, DiagnosticProvider,
    DiagnosticRegistrationOptions, DidChangeTextDocumentNotification,
    DidCloseTextDocumentNotification, DidOpenTextDocumentNotification, DocumentDiagnosticRequest,
    DocumentFilter, DocumentSymbolOptions, DocumentSymbolProvider, DocumentSymbolRequest,
    FoldingRangeOptions, FoldingRangeProvider, FoldingRangeRegistrationOptions,
    FoldingRangeRequest, HoverOptions, HoverProvider, HoverRegistrationOptions, HoverRequest,
    InitializeResult, Notification as _, Registration, Request as _, SelectionRangeOptions,
    SelectionRangeProvider, SelectionRangeRegistrationOptions, SelectionRangeRequest,
    ServerCapabilities, ServerInfo, StaticRegistrationOptions,
    TextDocumentChangeRegistrationOptions, TextDocumentFilter, TextDocumentFilterLanguage,
    TextDocumentRegistrationOptions, TextDocumentSync, TextDocumentSyncKind,
    TextDocumentSyncOptions, WorkDoneProgressOptions, WorkspaceFoldersServerCapabilities,
    WorkspaceOptions,
};

use crate::lsp::client::Client;

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
    let document_symbol_options = DocumentSymbolOptions {
        label: None,
        work_done_progress_options,
    };
    let folding_range_options = FoldingRangeRegistrationOptions {
        folding_range_options: FoldingRangeOptions {
            work_done_progress_options: WorkDoneProgressOptions::default(),
        },
        static_registration_options: StaticRegistrationOptions {
            id: Some(FoldingRangeRequest::METHOD.into()),
        },
        text_document_registration_options: text_document_registration_options.clone(),
    };
    let hover_options = HoverOptions {
        work_done_progress_options,
    };
    let selection_range_options = SelectionRangeRegistrationOptions {
        selection_range_options: SelectionRangeOptions::default(),
        static_registration_options: StaticRegistrationOptions {
            id: Some(SelectionRangeRequest::METHOD.into()),
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
            code_action_provider: if client.registers_code_actions() {
                None
            } else {
                Some(CodeActionProvider::CodeActionOptions(
                    code_action_options.clone(),
                ))
            },
            diagnostic_provider: if client.registers_diagnostics() {
                None
            } else {
                Some(DiagnosticProvider::DiagnosticRegistrationOptions(
                    diagnostic_options.clone(),
                ))
            },
            document_symbol_provider: if client.registers_document_symbols() {
                None
            } else {
                Some(DocumentSymbolProvider::DocumentSymbolOptions(
                    document_symbol_options.clone(),
                ))
            },
            folding_range_provider: if client.registers_folding_range() {
                None
            } else {
                Some(FoldingRangeProvider::FoldingRangeRegistrationOptions(
                    folding_range_options.clone(),
                ))
            },
            hover_provider: if client.registers_hover() {
                None
            } else {
                Some(HoverProvider::HoverOptions(hover_options))
            },
            position_encoding: Some(client.negotiated_encoding()),
            selection_range_provider: if client.registers_selection_range() {
                None
            } else {
                Some(SelectionRangeProvider::SelectionRangeRegistrationOptions(
                    selection_range_options.clone(),
                ))
            },
            text_document_sync: if client.registers_sync() {
                None
            } else {
                Some(TextDocumentSync::Options(TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::Incremental),
                    ..TextDocumentSyncOptions::default()
                }))
            },
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
    let mut registrations: Vec<Registration> = Vec::with_capacity(3);
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
    if client.registers_diagnostics() {
        registrations.push(Registration {
            id: DocumentDiagnosticRequest::METHOD.into(),
            method: DocumentDiagnosticRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(diagnostic_options)?),
        });
    }
    if client.registers_document_symbols() {
        registrations.push(Registration {
            id: DocumentSymbolRequest::METHOD.into(),
            method: DocumentSymbolRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(document_symbol_options)?),
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
    if client.registers_selection_range() {
        registrations.push(Registration {
            id: SelectionRangeRequest::METHOD.into(),
            method: SelectionRangeRequest::METHOD.into(),
            register_options: Some(serde_json::to_value(selection_range_options)?),
        });
    }
    Ok((result, registrations))
}
