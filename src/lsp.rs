use anyhow::{Context as _, Error, Result, bail};
use line_index::LineIndex;
use lsp_server::{
    Connection, ErrorCode, Message, Request as ServerRequest, RequestId, Response, ResponseError,
};
use lsp_types::{
    DiagnosticOptions, DiagnosticServerCapabilities, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentDiagnosticParams,
    InitializeParams, InitializeResult, OneOf, PublishDiagnosticsParams, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
    notification::{
        Cancel, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        Notification as _, PublishDiagnostics,
    },
    request::{DocumentDiagnosticRequest, Formatting, Request as _},
};
use rustc_hash::FxHashMap;
use std::time::Instant;
use tree_sitter::{InputEdit, Parser, Tree};

use crate::lsp::{
    client::Client,
    diagnostics::{pull_diagnostics, push_diagnostics},
};

mod client;
mod diagnostics;

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
                    send_error(
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
            // since we don't issue any requests, any response must result in connection close
            Message::Response(resp) => bail!("[lsp] unexpected response: {resp:?}"),
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
                send_notify(
                    connection,
                    PublishDiagnostics::METHOD,
                    push_diagnostics(client, &doc, &uri)?,
                )
            };
            docs.insert(uri.to_string(), doc);
            diagnosis
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            let doc = docs.remove(&uri.to_string()).context("document not open")?;
            let mut text = doc.text;
            let mut old_tree = doc.tree;
            let mut line_index = LineIndex::new(&text);
            for change in params.content_changes {
                let decoded = client
                    .decode_change(&change, &line_index)
                    .context("illegal range")?;
                // validate range is legal UTF-8
                text.get(decoded.start_byte..decoded.end_byte)
                    .context("illegal slice")?;
                // edit document
                text.replace_range(decoded.start_byte..decoded.end_byte, &change.text);
                // rebuild index
                line_index = LineIndex::new(&text);
                // edit parse tree
                let new_end_byte = decoded
                    .start_byte
                    .checked_add(change.text.len())
                    .context("overflow")?;
                let new_end_position =
                    Client::to_point(new_end_byte, &line_index).context("illegal range")?;
                old_tree.edit(&InputEdit {
                    start_byte: decoded.start_byte,
                    old_end_byte: decoded.end_byte,
                    new_end_byte,
                    start_position: decoded.start_point,
                    old_end_position: decoded.end_point,
                    new_end_position,
                });
            }
            parser.reset();
            let tree = parser
                .parse(&text, Some(&old_tree))
                .context("broken parser setup")?;
            let newdoc = Document {
                text,
                version: params.text_document.version,
                tree,
                line_index,
            };

            let diagnosis = if client.pull_diagnostics_support() {
                Ok(())
            } else {
                send_notify(
                    connection,
                    PublishDiagnostics::METHOD,
                    push_diagnostics(client, &newdoc, &uri)?,
                )
            };
            docs.insert(uri.to_string(), newdoc);
            diagnosis
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            docs.remove(&uri.to_string());
            // according to LSP spec, we should clear on close if we are pushing
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
    docs: &FxHashMap<String, Document>,
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
            send_response(connection, req.id.clone(), &response)?;
        }
        _ => {
            eprintln!("[lsp] unhandled request {req:?}");
            send_error(
                connection,
                req.id.clone(),
                ErrorCode::MethodNotFound,
                "unhandled request",
            )?;
        }
    }
    Ok(())
}

fn send_notify<T: serde::Serialize>(conn: &Connection, method: &str, params: T) -> Result<()> {
    let note = lsp_server::Notification::new(method.to_owned(), params);
    conn.sender.send(Message::Notification(note))?;
    Ok(())
}

fn send_response<T: serde::Serialize>(conn: &Connection, id: RequestId, result: &T) -> Result<()> {
    let resp = Response {
        id,
        result: Some(serde_json::to_value(result)?),
        error: None,
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}

fn send_error(conn: &Connection, id: RequestId, code: ErrorCode, msg: &str) -> Result<()> {
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
