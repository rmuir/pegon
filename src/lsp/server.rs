use std::collections::HashMap;

use anyhow::{Context as _, Error, Result, bail};
use line_index::LineIndex;
use lsp_server::{
    Connection, ErrorCode, Message, Notification, Request as ServerRequest, RequestId, Response,
};
use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, DiagnosticOptions, DiagnosticServerCapabilities,
    DocumentDiagnosticParams, InitializeResult, LogMessageParams, MessageType, OneOf,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
    notification::{
        Cancel, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, LogMessage,
        Notification as _, PublishDiagnostics,
    },
    request::{CodeActionRequest, DocumentDiagnosticRequest, Formatting, Request as _},
};
use serde::Serialize;
use tree_sitter::{Parser, Tree};

use crate::lsp::client::Client;

/// A Language Server Protocol Server
pub struct Server {
    connection: Connection,
}

/// A client-managed resource (file)
///
///
pub enum Resource {
    Java(Document),
    Other,
}

pub struct Document {
    pub(crate) text: String,
    pub(crate) version: i32,
    pub(crate) line_index: LineIndex,
    pub(crate) tree: Tree,
}

impl Server {
    pub fn new(connection: Connection, client: &Client, id: RequestId) -> Result<Self> {
        let result = serde_json::json!(InitializeResult {
            server_info: Some(ServerInfo {
                name: "pegon".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                        ..Default::default()
                    },
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("pegon".into()),
                        ..Default::default()
                    },
                )),
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
            },
        });
        connection.initialize_finish(id, result)?;
        Ok(Self { connection })
    }

    pub fn main_loop(&self, client: &Client) -> Result<(), Error> {
        let mut docs: HashMap<String, Resource> = HashMap::default();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&crate::LANGUAGE.into())?;

        for msg in &self.connection.receiver {
            match msg {
                Message::Request(req) => {
                    // try to go down gracefully, but always go down
                    if self.connection.handle_shutdown(&req)? {
                        break;
                    }
                    match handle_request(client, &req, &docs) {
                        Ok(response) => {
                            self.connection.sender.send(response)?;
                        }
                        Err(err) => {
                            self.connection.sender.send(error(
                                req.id.clone(),
                                ErrorCode::RequestFailed,
                                err.to_string(),
                            ))?;
                        }
                    }
                }
                Message::Notification(note) => {
                    match handle_notification(client, note, &mut docs, &mut parser) {
                        Ok(Some(push)) => {
                            self.connection.sender.send(push)?;
                        }
                        Err(err) => {
                            self.connection.sender.send(log_error(&err.to_string()))?;
                        }
                        _ => {}
                    }
                }
                // we can request workspaceEdit, but we don't care about the response.
                Message::Response(_) => {}
            }
        }
        Ok(())
    }
}

// handles an incoming request
// every request must have an associated response
fn handle_request(
    client: &Client,
    req: &ServerRequest,
    docs: &HashMap<String, Resource>,
) -> Result<Message> {
    match req.method.as_str() {
        CodeActionRequest::METHOD => {
            let params: CodeActionParams = serde_json::from_value(req.params.clone())?;
            let uri = &params.text_document.uri;
            let _doc = docs.get(&uri.to_string()).context("document not open")?;
            let actions: Vec<CodeActionOrCommand> = vec![];
            Ok(response::<CodeActionRequest>(req.id.clone(), Some(actions)))
        }
        Formatting::METHOD => {
            todo!()
        }
        DocumentDiagnosticRequest::METHOD => {
            let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
            let uri = &params.text_document.uri;
            match docs.get(&uri.to_string()) {
                Some(Resource::Java(doc)) => Ok(response::<DocumentDiagnosticRequest>(
                    req.id.clone(),
                    super::diagnostics::pull(client, doc, &params)?,
                )),
                Some(Resource::Other) => bail!("non-java document: {}", **uri),
                None => bail!("document not open: {}", **uri),
            }
        }
        _ => Ok(error(
            req.id.clone(),
            ErrorCode::MethodNotFound,
            "unhandled request".to_owned(),
        )),
    }
}

/// handles an incoming notification.
/// in our case notification has an "optional response".
/// if the client doesn't support pull diagnostics then we've got
/// a push diagnostics "response" that we'll `notify()` back
fn handle_notification(
    client: &Client,
    note: lsp_server::Notification,
    docs: &mut HashMap<String, Resource>,
    parser: &mut Parser,
) -> Result<Option<Message>> {
    let response = match note.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = serde_json::from_value(note.params)?;
            super::sync::did_open(client, params, docs, parser)?
        }
        DidChangeTextDocument::METHOD => {
            let params = serde_json::from_value(note.params)?;
            super::sync::did_change(client, params, docs, parser)?
        }
        DidCloseTextDocument::METHOD => {
            let params = serde_json::from_value(note.params)?;
            super::sync::did_close(client, params, docs)
        }
        // doesn't make sense for a single-threaded impl
        Cancel::METHOD => None,
        _ => {
            eprintln!("[lsp] unhandled notification {note:?}");
            None
        }
    }
    .map(notification::<PublishDiagnostics>);
    Ok(response)
}

/// creates a notification message to the client
fn notification<N>(params: N::Params) -> Message
where
    N: lsp_types::notification::Notification,
    N::Params: Serialize,
{
    Message::Notification(Notification::new(N::METHOD.to_owned(), params))
}

/// creates a successful response to the client
fn response<R>(id: RequestId, result: R::Result) -> Message
where
    R: lsp_types::request::Request,
    R::Result: Serialize,
{
    Message::Response(Response::new_ok(id, result))
}

/// creates an unsuccessful response to the LSP client
fn error(id: RequestId, code: ErrorCode, message: String) -> Message {
    Message::Response(Response::new_err(id, code as i32, message))
}

/// logs via notification an error to the LSP client
fn log_error(message: &String) -> Message {
    Message::Notification(Notification::new(
        LogMessage::METHOD.to_owned(),
        LogMessageParams {
            typ: MessageType::ERROR,
            message: format!("pegon: {message}"),
        },
    ))
}
