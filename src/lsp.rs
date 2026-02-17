use ropey::Rope;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, InitializeParams, InitializeResult, InitializedParams, Location,
    NumberOrString, OneOf, Position, SaveOptions, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Url,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::lint::{Lint, Linter};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            offset_encoding: None,

            capabilities: ServerCapabilities {
                // TODO: negotiate more efficient UTF-8 encoding
                position_encoding: None,
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
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
            uri: params.text_document.uri.to_string(),
            text: &params.text_document.text,
        })
        .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentChange {
            text: &params.content_changes[0].text,
            uri: params.text_document.uri.to_string(),
        })
        .await;
    }

    async fn did_save(&self, _params: DidSaveTextDocumentParams) {}

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
    Linter::new().lintnew(str.as_bytes().to_vec())
}

impl Backend {
    async fn on_change(&self, item: TextDocumentChange<'_>) {
        let uri =
            Url::parse(&item.uri).unwrap_or_else(|_| Url::from_directory_path(&item.uri).unwrap());

        let rope = Rope::from_str(item.text);
        let diagnostics = diagnose(item.text)
            .unwrap_or_default()
            .iter()
            .filter_map(|diagnostic| {
                let start = offset_to_position(diagnostic.range.start, &rope)?;
                let end = offset_to_position(diagnostic.range.end, &rope)?;
                let lsp_severity = match diagnostic.severity.as_str() {
                    "warn" => DiagnosticSeverity::WARNING,
                    "info" => DiagnosticSeverity::INFORMATION,
                    "hint" => DiagnosticSeverity::HINT,
                    _ => DiagnosticSeverity::ERROR,
                };
                let related_information = diagnostic
                    .context
                    .iter()
                    .filter_map(|context| {
                        let related_start = offset_to_position(context.start, &rope)?;
                        let related_end = offset_to_position(context.end, &rope)?;
                        let related = DiagnosticRelatedInformation {
                            location: Location {
                                uri: uri.clone(),
                                range: tower_lsp::lsp_types::Range::new(related_start, related_end),
                            },
                            message: diagnostic.context_label.clone().unwrap_or_default(),
                        };
                        Some(related)
                    })
                    .collect::<Vec<_>>();
                let diag = Diagnostic {
                    range: tower_lsp::lsp_types::Range::new(start, end),
                    severity: Some(lsp_severity),
                    code: Some(NumberOrString::String(diagnostic.name.clone())),
                    code_description: Some(CodeDescription {
                        href: Url::parse(&diagnostic.url).unwrap(),
                    }),
                    source: Some("pegon".to_string()),
                    message: diagnostic.title.clone(),
                    related_information: Some(related_information),
                    tags: None,
                    data: None,
                };
                Some(diag)
            })
            .collect::<Vec<_>>();

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

struct TextDocumentChange<'a> {
    uri: String,
    text: &'a str,
}

fn offset_to_position(offset: usize, rope: &Rope) -> Option<Position> {
    let line_number = rope.try_byte_to_line(offset).ok()?;
    let first_char_of_line = rope.try_line_to_byte(line_number).ok()?;
    let line = rope.byte_slice(first_char_of_line..offset);
    let column_number = line.len_utf16_cu();
    Some(Position::new(line_number as u32, column_number as u32))
}
