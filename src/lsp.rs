use anyhow::{Context, Error, Result, bail};
use crossbeam_channel::RecvTimeoutError;
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
use std::{io::ErrorKind, time::Duration};
use tree_sitter::Parser;

use crate::lsp::{
    client::Client,
    diagnostics::{pull_diagnostics, push_clear, push_diagnostics},
    open_document::OpenDocument,
};

mod client;
mod diagnostics;
mod open_document;

pub(crate) fn main() -> Result<(), Error> {
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
    drop(client.connection); // needed for the join to really succeed
    io_thread.join()?;
    Ok(())
}

fn main_loop(client: &Client) -> Result<(), Error> {
    let mut docs: FxHashMap<String, OpenDocument> = FxHashMap::default();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

    let connection = &client.connection;
    loop {
        let msg = match connection.receiver.recv_timeout(Duration::from_secs(30)) {
            Ok(msg) => msg,
            Err(RecvTimeoutError::Timeout) => {
                // no activity, check that parent is alive, if we were provided the id
                if let Some(pid) = client.process_id {
                    check_parent(pid)?;
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => {
                bail!("disconnected");
            }
        };

        match msg {
            Message::Request(req) => {
                // try to go down gracefully, but always go down
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                if let Err(err) = handle_request(client, &req, & /*mut*/ docs, &mut parser) {
                    eprintln!("[lsp] request {} failed: {err}", &req.method);
                    send_err(
                        connection,
                        req.id.clone(),
                        lsp_server::ErrorCode::RequestFailed,
                        err.to_string().as_str(),
                    )?;
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
            let doc = docs.get(&uri.to_string()).context("document not open")?;
            let mut text = doc.text.clone();
            for change in params.content_changes {
                if let Some(range) = change.range {
                    let line_index = LineIndex::new(&text);
                    let offsets = client
                        .decode_range(range, &line_index)
                        .context("illegal range")?;
                    text.get(offsets.clone()).context("illegal slice")?;
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

#[cfg(unix)]
fn check_parent(process_id: u32) -> Result<(), std::io::Error> {
    if let Ok(pid) = i32::try_from(process_id)
        && libc::ESRCH == unsafe { libc::kill(pid, 0) }
    {
        return Err(std::io::Error::from(ErrorKind::BrokenPipe));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_parent(process_id: u32) -> Result<(), std::io::Error> {
    Ok(())
}
