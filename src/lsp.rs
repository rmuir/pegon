use anyhow::{Error, Result};
use lsp_server::{Connection, Message, Request as ServerRequest, RequestId, Response};
use lsp_types::{
    DiagnosticOptions, DiagnosticServerCapabilities, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentDiagnosticParams,
    InitializeParams, InitializeResult, OneOf, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    },
    request::{DocumentDiagnosticRequest, Formatting, Request},
};
use rustc_hash::FxHashMap;

use crate::{
    lint::Linter,
    lsp::{
        client::Client,
        diagnostics::{pull_diagnostics, push_clear, push_diagnostics},
        open_document::OpenDocument,
    },
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
                    // TODO: delta updates
                    change: Some(TextDocumentSyncKind::FULL),
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
    let mut linter = Linter::new();

    for msg in &client.connection.receiver {
        match msg {
            Message::Request(req) => {
                if client.connection.handle_shutdown(&req)? {
                    break;
                }
                if let Err(err) = handle_request(client, &req, & /*mut*/ docs, &mut linter) {
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

fn handle_notification(
    client: &Client,
    note: &lsp_server::Notification,
    docs: &mut FxHashMap<String, OpenDocument>,
    linter: &mut Linter,
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
            push_diagnostics(client, &uri, docs, linter)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(note.params.clone())?;
            // TODO: loop
            if let Some(change) = params.content_changes.into_iter().next() {
                let uri = params.text_document.uri;
                let version = params.text_document.version;
                docs.insert(
                    uri.to_string(),
                    OpenDocument {
                        text: change.text,
                        version,
                    },
                );
                push_diagnostics(client, &uri, docs, linter)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            docs.remove(&uri.to_string());
            push_clear(client, &uri)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_request(
    client: &Client,
    req: &ServerRequest,
    docs: & /*mut*/ FxHashMap<String, OpenDocument>,
    linter: &mut Linter,
) -> Result<()> {
    match req.method.as_str() {
        Formatting::METHOD => {
            todo!()
        }
        DocumentDiagnosticRequest::METHOD => {
            let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
            let uri = params.text_document.uri;
            let response = pull_diagnostics(client, &uri, docs, linter)?;
            send_ok(&client.connection, req.id.clone(), &response)?;
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
