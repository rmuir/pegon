use anyhow::{Error, Result};
use line_index::{LineIndex, TextSize, WideEncoding};
use lsp_server::{Connection, Message, Request as ServerRequest, RequestId, Response};
use lsp_types::{
    CodeDescription,
    Diagnostic,
    DiagnosticRelatedInformation,
    DiagnosticSeverity,
    DidChangeTextDocumentParams,
    DidCloseTextDocumentParams,
    DidOpenTextDocumentParams,
    DidSaveTextDocumentParams,
    // core
    InitializeParams,
    InitializeResult,
    Location,
    NumberOrString,
    OneOf,
    Position,
    PublishDiagnosticsParams,
    Range,
    SaveOptions,
    ServerCapabilities,
    ServerInfo,
    TextDocumentSyncCapability,
    TextDocumentSyncKind,
    TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions,
    Url,
    WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
    // notifications
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Notification, PublishDiagnostics,
    },
    request::{Formatting, Request},
};
use rustc_hash::FxHashMap;

use crate::lint::{Lint, Linter, Severity, rule}; // for METHOD consts

// =====================================================================
// main
// =====================================================================

pub(crate) fn main() -> std::result::Result<(), Error> {
    // transport
    let (connection, io_thread) = Connection::stdio();

    // get the client capabilities
    let (id, params) = connection.initialize_start()?;
    let init_params: InitializeParams = serde_json::from_value(params)?;

    let result = serde_json::json!(InitializeResult {
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
    });

    connection.initialize_finish(id, result)?;
    main_loop(&connection, &init_params)?;
    io_thread.join()?;
    Ok(())
}

// =====================================================================
// event loop
// =====================================================================

fn main_loop(
    connection: &Connection,
    _params: &InitializeParams,
) -> std::result::Result<(), Error> {
    let mut docs: FxHashMap<Url, String> = FxHashMap::default();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                if let Err(_err) = handle_request(connection, &req, &mut docs) {
                    //log::error!("[lsp] request {} failed: {err}", &req.method);
                }
            }
            Message::Notification(note) => {
                if let Err(_err) = handle_notification(connection, &note, &mut docs) {
                    //log::error!("[lsp] notification {} failed: {err}", note.method);
                }
            }
            Message::Response(_resp) => {} //log::error!("[lsp] response: {resp:?}"),
        }
    }
    Ok(())
}

// =====================================================================
// notifications
// =====================================================================

fn handle_notification(
    conn: &Connection,
    note: &lsp_server::Notification,
    docs: &mut FxHashMap<Url, String>,
) -> Result<()> {
    match note.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let p: DidOpenTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = p.text_document.uri;
            docs.insert(uri.clone(), p.text_document.text);
            diagnostics(conn, docs, &uri)?;
        }
        DidChangeTextDocument::METHOD => {
            let p: DidChangeTextDocumentParams = serde_json::from_value(note.params.clone())?;
            if let Some(change) = p.content_changes.into_iter().next() {
                let uri = p.text_document.uri;
                docs.insert(uri.clone(), change.text);
                diagnostics(conn, docs, &uri)?;
            }
        }
        DidSaveTextDocument::METHOD => {
            let p: DidSaveTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = p.text_document.uri;
            if let Some(text) = p.text {
                docs.insert(uri.clone(), text);
                diagnostics(conn, docs, &uri)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let p: DidCloseTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = p.text_document.uri;
            docs.remove(&uri);
        }
        _ => {}
    }
    Ok(())
}

/// currently no requests are supported
fn handle_request(
    conn: &Connection,
    req: &ServerRequest,
    _docs: &mut FxHashMap<Url, String>,
) -> Result<()> {
    match req.method.as_str() {
        Formatting::METHOD => {}
        _ => send_err(
            conn,
            req.id.clone(),
            lsp_server::ErrorCode::MethodNotFound,
            "unhandled method",
        )?,
    }
    Ok(())
}

/// publish diagnostics
fn diagnostics(conn: &Connection, docs: &FxHashMap<Url, String>, uri: &Url) -> Result<()> {
    let text = docs.get(uri).unwrap();

    let line_index = LineIndex::new(text);
    let diagnostics = diagnose(text)
        .unwrap_or_default()
        .iter()
        .filter_map(|diagnostic| {
            let rule = rule(diagnostic.rule_id);
            let start = offset_to_position(diagnostic.range.start, &line_index)?;
            let end = offset_to_position(diagnostic.range.end, &line_index)?;
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
                    let related_start = offset_to_position(context.start, &line_index)?;
                    let related_end = offset_to_position(context.end, &line_index)?;
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
                    href: Url::parse(&rule.url).unwrap(),
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
    conn.sender
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

// TODO: so inefficient
fn diagnose(str: &str) -> std::result::Result<Vec<Lint>, anyhow::Error> {
    Linter::new().lint(str.as_bytes())
}

/// Convert a UTF-8 byte offset to a UTF-16 LSP position
fn offset_to_position(offset: usize, line_index: &LineIndex) -> Option<Position> {
    let position = line_index.try_line_col(TextSize::from(offset as u32))?;
    let wide = line_index.to_wide(WideEncoding::Utf16, position)?;
    Some(Position::new(wide.line, wide.col))
}
