use anyhow::{Context, Error, Result};
use line_index::LineIndex;
use lsp_server::{Connection, Message, Request as ServerRequest, RequestId, Response};
use lsp_types::{
    DiagnosticOptions, DiagnosticServerCapabilities, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentDiagnosticParams,
    InitializeParams, InitializeResult, OneOf, PublishDiagnosticsParams, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
    notification::{
        Cancel, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
        PublishDiagnostics,
    },
    request::{DocumentDiagnosticRequest, Formatting, Request},
};
use rustc_hash::FxHashMap;
use std::time::Instant;
use tree_sitter::Parser;

use crate::lsp::{
    client::Client,
    diagnostics::{pull_diagnostics, push_diagnostics},
    document::Document,
};

mod client;
mod diagnostics;
mod document;

pub(crate) fn main() -> Result<(), Error> {
    // transport
    let (connection, io_thread) = Connection::stdio();

    // get the client capabilities
    let (id, params) = connection.initialize_start()?;
    let init_params: InitializeParams = serde_json::from_value(params)?;

    let client = Client::new(init_params);

    let result = serde_json::json!(InitializeResult {
        server_info: Some(ServerInfo {
            name: "pegon".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        }),
        capabilities: ServerCapabilities {
            position_encoding: Some(client.negotiated_encoding()),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
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
            diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                identifier: Some("pegon".into()),
                ..Default::default()
            })),
            ..ServerCapabilities::default()
        }
    });

    connection.initialize_finish(id, result)?;
    main_loop(&connection, &client)?;
    drop(connection);
    io_thread.join()?;
    Ok(())
}

fn main_loop(connection: &Connection, client: &Client) -> Result<(), Error> {
    let mut docs: FxHashMap<String, Document> = FxHashMap::default();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

    for msg in &connection.receiver {
        let start_time = Instant::now();
        match msg {
            Message::Request(req) => {
                // try to go down gracefully, but always go down
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                if let Err(err) = handle_request(connection, client, &req, & /*mut*/ docs) {
                    eprintln!("[lsp] request {} failed: {err}", req.method);
                    send_err(
                        connection,
                        req.id.clone(),
                        lsp_server::ErrorCode::RequestFailed,
                        err.to_string().as_str(),
                    )?;
                }
                eprintln!(
                    "[request] {}: {} ms",
                    req.method,
                    start_time.elapsed().as_millis()
                );
            }
            Message::Notification(note) => {
                if let Err(err) =
                    handle_notification(connection, client, &note, &mut docs, &mut parser)
                {
                    eprintln!("[lsp] notification {} failed: {err}", note.method);
                }
                eprintln!(
                    "[notify] {}: {} ms",
                    note.method,
                    start_time.elapsed().as_millis()
                );
            }
            Message::Response(resp) => {
                eprintln!("[lsp] unexpected response: {resp:?}");
            }
        }
    }
    Ok(())
}

fn handle_notification(
    connection: &Connection,
    client: &Client,
    note: &lsp_server::Notification,
    docs: &mut FxHashMap<String, Document>,
    parser: &mut Parser,
) -> Result<()> {
    match note.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            parser.reset();
            let tree = parser
                .parse(&params.text_document.text, None)
                .context("broken parser setup")?;
            let line_index = LineIndex::new(&params.text_document.text);
            let doc = Document {
                text: params.text_document.text,
                version: params.text_document.version,
                tree,
                line_index,
            };
            let diagnosis = if client.pull_diagnostics_support() {
                Ok(())
            } else {
                let push = push_diagnostics(client, &doc, &uri)?;
                send_notify(connection, PublishDiagnostics::METHOD, push)
            };
            docs.insert(uri.to_string(), doc);
            diagnosis
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            let doc = docs.get(&uri.to_string()).context("document not open")?;
            let mut text = doc.text.clone();
            let mut line_index = LineIndex::new(&text);
            for change in params.content_changes {
                if let Some(range) = change.range {
                    let offsets = client
                        .decode_range(range, &line_index)
                        .context("illegal range")?;
                    text.get(offsets.clone()).context("illegal slice")?;
                    text.replace_range(offsets, &change.text);
                    line_index = LineIndex::new(&text);
                } else {
                    text = change.text;
                }
            }
            // TODO: still not incremental
            parser.reset();
            let tree = parser.parse(&text, None).context("broken parser setup")?;
            let doc = Document {
                text,
                version: params.text_document.version,
                tree,
                line_index,
            };

            let diagnosis = if client.pull_diagnostics_support() {
                Ok(())
            } else {
                let push = push_diagnostics(client, &doc, &uri)?;
                send_notify(connection, PublishDiagnostics::METHOD, push)
            };
            docs.insert(uri.to_string(), doc);
            diagnosis
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            docs.remove(&uri.to_string());
            if !client.pull_diagnostics_support() {
                send_notify(
                    connection,
                    PublishDiagnostics::METHOD,
                    PublishDiagnosticsParams {
                        diagnostics: vec![],
                        uri,
                        version: None,
                    },
                )?;
            }
            Ok(())
        }
        // doesn't make sense for a single-threaded impl
        Cancel::METHOD => Ok(()),
        _ => {
            eprintln!("[lsp] unhandled notification {note:?}");
            Ok(())
        }
    }
}

fn handle_request(
    connection: &Connection,
    client: &Client,
    req: &ServerRequest,
    docs: & /*mut*/ FxHashMap<String, Document>,
) -> Result<()> {
    match req.method.as_str() {
        Formatting::METHOD => {
            todo!()
        }
        DocumentDiagnosticRequest::METHOD => {
            let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
            let uri = &params.text_document.uri;
            let doc = docs.get(&uri.to_string()).context("document not open")?;
            let response = pull_diagnostics(client, doc, &params)?;
            send_ok(connection, req.id.clone(), &response)?;
        }
        _ => {
            eprintln!("[lsp] unhandled request {req:?}");
            send_err(
                connection,
                req.id.clone(),
                lsp_server::ErrorCode::MethodNotFound,
                "unhandled request",
            )?;
        }
    }
    Ok(())
}

fn send_notify<T: serde::Serialize>(conn: &Connection, method: &str, params: T) -> Result<()> {
    let note = lsp_server::Notification::new(method.to_string(), params);
    conn.sender.send(Message::Notification(note))?;
    Ok(())
}

fn send_ok<T: serde::Serialize>(conn: &Connection, id: RequestId, result: &T) -> Result<()> {
    let resp = Response {
        id,
        result: Some(serde_json::to_value(result)?),
        error: None,
    };
    conn.sender.send(Message::Response(resp))?;
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
