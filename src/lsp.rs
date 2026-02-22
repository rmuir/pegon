use std::process::exit;

use anyhow::{Error, Result, bail};
use line_index::LineIndex;
use lsp_server::{Connection, Message, Request as ServerRequest, RequestId, Response};
use lsp_types::{
    DiagnosticOptions, DiagnosticServerCapabilities, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentDiagnosticParams,
    InitializeParams, InitializeResult, OneOf, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
    notification::{
        Cancel, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    },
    request::{DocumentDiagnosticRequest, Formatting, Request},
};
use rustc_hash::FxHashMap;
use tree_sitter::Parser;

use crate::lsp::{
    client::Client,
    diagnostics::{pull_diagnostics, push_clear, push_diagnostics},
    open_document::OpenDocument,
};

mod client;
mod diagnostics;
mod open_document;

pub(crate) fn main() -> std::result::Result<(), Error> {
    // transport
    let (connection, io_thread) = Connection::stdio();

    // get the client capabilities
    let (id, params) = connection.initialize_start()?;
    let init_params: InitializeParams = serde_json::from_value(params)?;

    let client = Client::new(connection, init_params);

    let result = serde_json::json!(InitializeResult {
        server_info: Some(ServerInfo {
            name: "pegon".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        }),
        offset_encoding: None,
        capabilities: ServerCapabilities {
            diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                identifier: Some("pegon".into()),
                ..Default::default()
            })),
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
            ..ServerCapabilities::default()
        }
    });

    client.connection.initialize_finish(id, result)?;
    main_loop(&client)?;
    io_thread.join()?;
    Ok(())
}

fn main_loop(client: &Client) -> Result<(), Error> {
    let mut docs: FxHashMap<String, OpenDocument> = FxHashMap::default();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

    for msg in &client.connection.receiver {
        match msg {
            Message::Request(req) => {
                // try to go down gracefully, but always go down
                let Ok(false) = client.connection.handle_shutdown(&req) else {
                    exit(0)
                };
                if let Err(err) = handle_request(client, &req, & /*mut*/ docs, &mut parser) {
                    eprintln!("[lsp] request {} failed: {err}", &req.method);
                    if let Err(err) = send_err(
                        &client.connection,
                        req.id.clone(),
                        lsp_server::ErrorCode::RequestFailed,
                        err.to_string().as_str(),
                    ) {
                        bail!("[lsp] request {} error delivery failed {err}", &req.method);
                    }
                }
            }
            Message::Notification(note) => {
                if let Err(err) = handle_notification(client, &note, &mut docs, &mut parser) {
                    eprintln!("[lsp] notification {} failed: {err}", note.method);
                }
            }
            Message::Response(resp) => {
                eprintln!("[lsp] unexpected response: {resp:?}");
            }
        }
    }
    Ok(())
}

fn handle_notification(
    client: &Client,
    note: &lsp_server::Notification,
    docs: &mut FxHashMap<String, OpenDocument>,
    parser: &mut Parser,
) -> Result<()> {
    match note.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            let version = params.text_document.version;
            docs.insert(
                uri.to_string(),
                OpenDocument {
                    text: params.text_document.text,
                    version,
                },
            );
            if !client.pull_diagnostics_support() {
                push_diagnostics(client, &uri, docs, parser)?;
            }
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            let version = params.text_document.version;
            let Some(doc) = docs.get(&uri.to_string()) else {
                bail!("[lsp] document not open: {uri:?}, version: {version}");
            };
            let mut text = doc.text.clone();
            for change in params.content_changes {
                if let Some(range) = change.range {
                    let line_index = LineIndex::new(&text);
                    let Some(offsets) = client.decode_range(range, &line_index) else {
                        bail!("[lsp] illegal range: {range:?}, uri: {uri:?}, version: {version}");
                    };
                    if text.get(offsets.clone()).is_none() {
                        bail!("[lsp] illegal slice: {range:?}, uri: {uri:?}, version: {version}");
                    }
                    text.replace_range(offsets, &change.text);
                } else {
                    text = change.text;
                }
            }

            docs.insert(uri.to_string(), OpenDocument { text, version });
            if !client.pull_diagnostics_support() {
                push_diagnostics(client, &uri, docs, parser)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            docs.remove(&uri.to_string());
            if !client.pull_diagnostics_support() {
                push_clear(client, &uri)?;
            }
        }
        // doesn't make sense for a single-threaded impl
        Cancel::METHOD => {}
        _ => {
            eprintln!("[lsp] unhandled notification {note:?}");
        }
    }
    Ok(())
}

fn handle_request(
    client: &Client,
    req: &ServerRequest,
    docs: & /*mut*/ FxHashMap<String, OpenDocument>,
    parser: &mut Parser,
) -> Result<()> {
    match req.method.as_str() {
        Formatting::METHOD => {
            todo!()
        }
        DocumentDiagnosticRequest::METHOD => {
            let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
            let uri = params.text_document.uri;
            let response = pull_diagnostics(client, &uri, docs, parser)?;
            send_ok(&client.connection, req.id.clone(), &response)?;
        }
        _ => {
            eprintln!("[lsp] unhandled request {req:?}");
            send_err(
                &client.connection,
                req.id.clone(),
                lsp_server::ErrorCode::MethodNotFound,
                "unhandled request",
            )?;
        }
    }
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
