use anyhow::{Context as _, Error, Result, bail};
use line_index::LineIndex;
use lsp_server::{
    Connection, ErrorCode, Message, Request as ServerRequest, RequestId, Response, ResponseError,
};
use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CodeActionResponse, DiagnosticOptions,
    DiagnosticServerCapabilities, DocumentDiagnosticParams, InitializeParams, InitializeResult,
    OneOf, ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
    notification::{
        Cancel, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    },
    request::{CodeActionRequest, DocumentDiagnosticRequest, Formatting, Request as _},
};
use rustc_hash::FxHashMap;
use std::time::Instant;
use tree_sitter::{Parser, Tree};

use crate::lsp::client::Client;

mod client;
mod diagnostics;
mod sync;

pub struct Document {
    pub(crate) text: String,
    pub(crate) version: i32,
    pub(crate) line_index: LineIndex,
    pub(crate) tree: Tree,
}

/// Start lsp server with provided connection
///
/// # Errors
///
/// This function will return an error if something bad happens
pub fn start(connection: Connection) -> Result<(), Error> {
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
            code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                ..Default::default()
            })),
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
                    change_notifications: Some(OneOf::Left(false)),
                }),
                file_operations: None,
            }),
            ..ServerCapabilities::default()
        }
    });

    connection.initialize_finish(id, result)?;
    main_loop(&connection, &client)?;
    drop(connection);
    Ok(())
}

fn main_loop(connection: &Connection, client: &Client) -> Result<(), Error> {
    let mut docs: FxHashMap<String, Document> = FxHashMap::default();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&crate::LANGUAGE.into())?;

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
                    error(
                        connection,
                        req.id.clone(),
                        ErrorCode::RequestFailed,
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
                let method = note.method.clone();
                if let Err(err) =
                    handle_notification(connection, client, note, &mut docs, &mut parser)
                {
                    eprintln!("[lsp] notification {method} failed: {err}");
                }
                eprintln!(
                    "[notify] {}: {} ms",
                    method,
                    start_time.elapsed().as_millis()
                );
            }
            // since we don't issue any requests, any response must result in connection close
            Message::Response(resp) => bail!("[lsp] unexpected response: {resp:?}"),
        }
    }
    Ok(())
}

fn handle_notification(
    connection: &Connection,
    client: &Client,
    note: lsp_server::Notification,
    docs: &mut FxHashMap<String, Document>,
    parser: &mut Parser,
) -> Result<()> {
    match note.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = serde_json::from_value(note.params)?;
            sync::did_open(connection, client, params, docs, parser)
        }
        DidChangeTextDocument::METHOD => {
            let params = serde_json::from_value(note.params)?;
            sync::did_change(connection, client, params, docs, parser)
        }
        DidCloseTextDocument::METHOD => {
            let params = serde_json::from_value(note.params)?;
            sync::did_close(connection, client, params, docs)
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
    docs: &FxHashMap<String, Document>,
) -> Result<()> {
    match req.method.as_str() {
        CodeActionRequest::METHOD => {
            let params: CodeActionParams = serde_json::from_value(req.params.clone())?;
            let uri = &params.text_document.uri;
            let doc = docs.get(&uri.to_string()).context("document not open")?;
            let response: Vec<CodeActionOrCommand> = vec![];
            respond(connection, req.id.clone(), &response);
        }

        Formatting::METHOD => {
            todo!()
        }
        DocumentDiagnosticRequest::METHOD => {
            let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
            let uri = &params.text_document.uri;
            let doc = docs.get(&uri.to_string()).context("document not open")?;
            let response = diagnostics::pull(client, doc, &params)?;
            respond(connection, req.id.clone(), &response)?;
        }
        _ => {
            eprintln!("[lsp] unhandled request {req:?}");
            error(
                connection,
                req.id.clone(),
                ErrorCode::MethodNotFound,
                "unhandled request",
            )?;
        }
    }
    Ok(())
}

pub(crate) fn notify<T: serde::Serialize>(
    conn: &Connection,
    method: &str,
    params: T,
) -> Result<()> {
    let note = lsp_server::Notification::new(method.to_owned(), params);
    conn.sender.send(Message::Notification(note))?;
    Ok(())
}

fn respond<T: serde::Serialize>(conn: &Connection, id: RequestId, result: &T) -> Result<()> {
    let resp = Response {
        id,
        result: Some(serde_json::to_value(result)?),
        error: None,
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}

fn error(conn: &Connection, id: RequestId, code: ErrorCode, msg: &str) -> Result<()> {
    let resp = Response {
        id,
        result: None,
        error: Some(ResponseError {
            code: code as i32,
            message: msg.into(),
            data: None,
        }),
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}
