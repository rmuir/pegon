use anyhow::{Error, Result};
use lsp_server::{Connection, Message, Request as ServerRequest, RequestId, Response};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, InitializeParams, InitializeResult, ServerInfo,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Notification,
    },
    request::{Formatting, Request},
};
use rustc_hash::FxHashMap;

use crate::{lint::Linter, lsp::client::Client, lsp::diagnostics::push_diagnostics};

mod client;
mod diagnostics;

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
        capabilities: client.server_capabilities()
    });

    client.connection.initialize_finish(id, result)?;
    main_loop(&client)?;
    io_thread.join()?;
    Ok(())
}

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
            push_diagnostics(client, &uri, docs, linter)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(note.params.clone())?;
            if let Some(change) = params.content_changes.into_iter().next() {
                let uri = params.text_document.uri;
                docs.insert(uri.to_string(), change.text);
                push_diagnostics(client, &uri, docs, linter)?;
            }
        }
        DidSaveTextDocument::METHOD => {
            let params: DidSaveTextDocumentParams = serde_json::from_value(note.params.clone())?;
            let uri = params.text_document.uri;
            if let Some(text) = params.text {
                docs.insert(uri.to_string(), text);
                push_diagnostics(client, &uri, docs, linter)?;
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
