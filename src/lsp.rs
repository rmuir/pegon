use std::str::FromStr;

use ropey::Rope;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, InitializeParams, InitializeResult, InitializedParams, Location,
    NumberOrString, OneOf, Position, SaveOptions, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Uri, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

use crate::lint::{Lint, Linter, Severity, rule};

struct Backend {
    client: Client,
}

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "pegon".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            offset_encoding: None,

            capabilities: ServerCapabilities {
                // TODO: negotiate more efficient UTF-8 encoding
                position_encoding: None,
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        // TODO: delta updates
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(TextDocumentChange {
            uri: params.text_document.uri,
            text: &params.text_document.text,
        })
        .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentChange {
            uri: params.text_document.uri,
            text: &params.content_changes[0].text,
        })
        .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.on_change(TextDocumentChange {
            uri: params.text_document.uri,
            text: &params.text.unwrap_or_default(),
        })
        .await;
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {}

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {}

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {}

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {}
}

pub(crate) async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend { client }).finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}

// TODO: so inefficient
fn diagnose(str: &str) -> std::result::Result<Vec<Lint>, anyhow::Error> {
    Linter::new().lint(str.as_bytes())
}

impl Backend {
    async fn on_change(&self, item: TextDocumentChange<'_>) {
        let rope = Rope::from_str(item.text);
        let diagnostics = diagnose(item.text)
            .unwrap_or_default()
            .iter()
            .filter_map(|diagnostic| {
                let rule = rule(diagnostic.rule_id);
                let start = offset_to_position(diagnostic.range.start, &rope)?;
                let end = offset_to_position(diagnostic.range.end, &rope)?;
                let lsp_severity = match rule.severity {
                    Severity::Warn => DiagnosticSeverity::WARNING,
                    Severity::Info => DiagnosticSeverity::INFORMATION,
                    Severity::Hint => DiagnosticSeverity::HINT,
                    Severity::Error => DiagnosticSeverity::ERROR,
                };
                // all the context ranges are related information
                let mut related_information = diagnostic
                    .context
                    .iter()
                    .filter_map(|context| {
                        let related_start = offset_to_position(context.start, &rope)?;
                        let related_end = offset_to_position(context.end, &rope)?;
                        let related = DiagnosticRelatedInformation {
                            location: Location {
                                uri: item.uri.clone(),
                                range: tower_lsp_server::ls_types::Range::new(
                                    related_start,
                                    related_end,
                                ),
                            },
                            message: rule.context_label.clone().unwrap_or_default(),
                        };
                        Some(related)
                    })
                    .collect::<Vec<_>>();
                // optional label maps to related information at node's position
                if let Some(label) = &diagnostic.label {
                    related_information.push(DiagnosticRelatedInformation {
                        location: Location {
                            uri: item.uri.clone(),
                            range: tower_lsp_server::ls_types::Range::new(start, end),
                        },
                        message: label.clone(),
                    });
                }
                // help text maps to related information at node's position
                related_information.push(DiagnosticRelatedInformation {
                    location: Location {
                        uri: item.uri.clone(),
                        range: tower_lsp_server::ls_types::Range::new(start, end),
                    },
                    message: diagnostic.help.clone(),
                });
                Some(Diagnostic {
                    range: tower_lsp_server::ls_types::Range::new(start, end),
                    severity: Some(lsp_severity),
                    code: Some(NumberOrString::String(rule.name.clone())),
                    code_description: Some(CodeDescription {
                        href: Uri::from_str(&rule.url).unwrap(),
                    }),
                    source: Some("pegon".to_string()),
                    message: diagnostic.title.clone(),
                    related_information: Some(related_information),
                    tags: None,
                    data: None,
                })
            })
            .collect::<Vec<_>>();

        self.client
            .publish_diagnostics(item.uri, diagnostics, None)
            .await;
    }
}

struct TextDocumentChange<'a> {
    uri: Uri,
    text: &'a str,
}

/// Convert a UTF-8 byte offset to a UTF-16 LSP position
fn offset_to_position(offset: usize, rope: &Rope) -> Option<Position> {
    let line = rope.try_byte_to_line(offset).ok()?;
    let first_char_of_line = rope.try_line_to_byte(line).ok()?;
    let line_data = rope.byte_slice(first_char_of_line..offset);
    let character = line_data.len_utf16_cu();
    Some(Position::new(line as u32, character as u32))
}
