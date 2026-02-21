use std::str::FromStr;

use anyhow::{Error, Result};
use line_index::LineIndex;
use lsp_server::{Connection, Message, Request as ServerRequest, RequestId, Response};
use lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, InitializeParams, InitializeResult, Location, NumberOrString, OneOf,
    PublishDiagnosticsParams, Range, SaveOptions, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Uri, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Notification, PublishDiagnostics,
    },
    request::{Formatting, Request},
};
use rustc_hash::FxHashMap;

use crate::{
    lint::{Linter, Severity, rule},
    lsp::encoding::Encoding,
};

mod encoding;

// =====================================================================
// main
// =====================================================================

pub(crate) fn main() -> std::result::Result<(), Error> {
    // transport
    let (connection, io_thread) = Connection::stdio();

    // get the client capabilities
    let (id, params) = connection.initialize_start()?;
    let init_params: InitializeParams = serde_json::from_value(params)?;

    let encoding = Encoding::preferred(&init_params.capabilities);

    let result = serde_json::json!(InitializeResult {
        server_info: Some(ServerInfo {
            name: "pegon".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        }),
        offset_encoding: None,

        capabilities: ServerCapabilities {
            position_encoding: Some(encoding.into()),
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
    });

    connection.initialize_finish(id, result)?;
    let client = Client {
        connection,
        init_params,
        encoding,
    };
    main_loop(&client)?;
    io_thread.join()?;
    Ok(())
}

// =====================================================================
// event loop
// =====================================================================

fn main_loop(client: &Client) -> Result<(), Error> {
    let mut docs: FxHashMap<String, String> = FxHashMap::default();
    let mut linter = Linter::new();

    for msg in &client.connection.receiver {
        match msg {
            Message::Request(req) => {
                if client.connection.handle_shutdown(&req)? {
                    break;
                }
                if let Err(err) = handle_request(client, &req, &mut docs, &mut linter) {
                    eprintln!("[lsp] request {} failed: {err}", &req.method);
                }
            }
            Message::Notification(note) => {
                if let Err(err) = handle_notification(client, &note, &mut docs, &mut linter) {
                    eprintln!("[lsp] notification {} failed: {err}", note.method);
                }
            }
            Message::Response(resp) => {
                eprintln!("[lsp] response: {resp:?}");
            }
        }
    }
    Ok(())
}

// =====================================================================
// notifications
// =====================================================================

fn handle_notification(
    client: &Client,
    note: &lsp_server::Notification,
    docs: &mut FxHashMap<String, String>,
    linter: &mut Linter,
) -> Result<()> {
    match note.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            docs.insert(uri.to_string(), params.text_document.text);
            diagnostics(client, &uri, docs, linter)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(note.params.clone())?;
            if let Some(change) = params.content_changes.into_iter().next() {
                let uri = params.text_document.uri;
                docs.insert(uri.to_string(), change.text);
                diagnostics(client, &uri, docs, linter)?;
            }
        }
        DidSaveTextDocument::METHOD => {
            let params: DidSaveTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            if let Some(text) = params.text {
                docs.insert(uri.to_string(), text);
                diagnostics(client, &uri, docs, linter)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            docs.remove(&uri.to_string());
        }
        _ => {}
    }
    Ok(())
}

/// currently no requests are supported
fn handle_request(
    client: &Client,
    req: &ServerRequest,
    _docs: &mut FxHashMap<String, String>,
    _linter: &mut Linter,
) -> Result<()> {
    match req.method.as_str() {
        Formatting::METHOD => {
            todo!()
        }
        _ => send_err(
            &client.connection,
            req.id.clone(),
            lsp_server::ErrorCode::MethodNotFound,
            "unhandled method",
        )?,
    }
    Ok(())
}

/// publish diagnostics
fn diagnostics(
    client: &Client,
    uri: &Uri,
    docs: &FxHashMap<String, String>,
    linter: &mut Linter,
) -> Result<()> {
    let text = docs.get(&uri.to_string()).unwrap();
    let encoding = &client.encoding;

    let line_index = LineIndex::new(text);
    let diagnostics = linter
        .lint(text.as_bytes())
        .unwrap_or_default()
        .iter()
        .filter_map(|diagnostic| {
            let rule = rule(diagnostic.rule_id);
            let start = encoding.to_position(diagnostic.range.start, &line_index)?;
            let end = encoding.to_position(diagnostic.range.end, &line_index)?;
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
                    let related_start = encoding.to_position(context.start, &line_index)?;
                    let related_end = encoding.to_position(context.end, &line_index)?;
                    let related = DiagnosticRelatedInformation {
                        location: Location {
                            uri: uri.clone(),
                            range: Range::new(related_start, related_end),
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
                        uri: uri.clone(),
                        range: Range::new(start, end),
                    },
                    message: label.clone(),
                });
            }
            // help text maps to related information at node's position
            related_information.push(DiagnosticRelatedInformation {
                location: Location {
                    uri: uri.clone(),
                    range: Range::new(start, end),
                },
                message: diagnostic.help.clone(),
            });
            Some(Diagnostic {
                range: Range::new(start, end),
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

    let params = PublishDiagnosticsParams {
        diagnostics,
        uri: uri.clone(),
        version: None,
    };
    client
        .connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            PublishDiagnostics::METHOD.to_owned(),
            params,
        )))?;
    Ok(())
}

fn send_err(
    conn: &Connection,
    id: RequestId,
    code: lsp_server::ErrorCode,
    msg: &str,
) -> Result<()> {
    let resp = Response {
        id,
        result: None,
        error: Some(lsp_server::ResponseError {
            code: code as i32,
            message: msg.into(),
            data: None,
        }),
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}

struct Client {
    connection: Connection,
    // TODO!
    #[allow(dead_code)]
    init_params: InitializeParams,
    encoding: Encoding,
}
