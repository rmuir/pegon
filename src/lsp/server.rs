use std::collections::HashMap;

use anyhow::{Context as _, Error, Result};
use crossbeam_channel::SendError;
use line_index::LineIndex;
use lsp_server::{
    Connection, ErrorCode, Message, Notification, Request as ServerRequest, RequestId, Response,
};
use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, DiagnosticOptions, DiagnosticServerCapabilities,
    DocumentDiagnosticParams, DocumentDiagnosticReportResult, InitializeResult, OneOf,
    PublishDiagnosticsParams, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
    notification::{
        Cancel, DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument,
        Notification as _, PublishDiagnostics,
    },
    request::{CodeActionRequest, DocumentDiagnosticRequest, Formatting, Request as _},
};
use serde::Serialize;
use tree_sitter::{Parser, Tree};

use crate::lsp::client::Client;

pub struct Server {
    connection: Connection,
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
        let mut docs: HashMap<String, Document> = HashMap::default();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&crate::LANGUAGE.into())?;

        for msg in &self.connection.receiver {
            match msg {
                Message::Request(req) => {
                    // try to go down gracefully, but always go down
                    if self.connection.handle_shutdown(&req)? {
                        break;
                    }
                    if let Err(err) = self.handle_request(client, &req, &docs) {
                        eprintln!("[lsp] request {} failed: {err}", req.method);
                        error(
                            &self.connection,
                            req.id.clone(),
                            ErrorCode::RequestFailed,
                            err.to_string().as_str(),
                        )?;
                    }
                }
                Message::Notification(note) => {
                    let method = note.method.clone();
                    match handle_notification(client, note, &mut docs, &mut parser) {
                        Ok(Some(push)) => {
                            notify::<PublishDiagnostics>(&self.connection, push)?;
                        }
                        Err(err) => {
                            eprintln!("[lsp] notification {method} failed: {err}");
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

    fn handle_request(
        &self,
        client: &Client,
        req: &ServerRequest,
        docs: &HashMap<String, Document>,
    ) -> Result<()> {
        match req.method.as_str() {
            CodeActionRequest::METHOD => {
                let params: CodeActionParams = serde_json::from_value(req.params.clone())?;
                let uri = &params.text_document.uri;
                let _doc = docs.get(&uri.to_string()).context("document not open")?;
                let response: Vec<CodeActionOrCommand> = vec![];
                respond::<CodeActionRequest>(&self.connection, req.id.clone(), Some(response))?;
            }

            Formatting::METHOD => {
                todo!()
            }
            DocumentDiagnosticRequest::METHOD => {
                let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
                let uri = &params.text_document.uri;
                let doc = docs.get(&uri.to_string()).context("document not open")?;
                let response = super::diagnostics::pull(client, doc, &params)?;
                respond::<DocumentDiagnosticRequest>(
                    &self.connection,
                    req.id.clone(),
                    DocumentDiagnosticReportResult::Report(response),
                )?;
            }
            _ => {
                error(
                    &self.connection,
                    req.id.clone(),
                    ErrorCode::MethodNotFound,
                    "unhandled request",
                )?;
            }
        }
        Ok(())
    }
}

/// handles an incoming notification.
/// if the client doesn't support pull diagnostics then we've got
/// a push diagnostics "response" that we'll `notify()` back
fn handle_notification(
    client: &Client,
    note: lsp_server::Notification,
    docs: &mut HashMap<String, Document>,
    parser: &mut Parser,
) -> Result<Option<PublishDiagnosticsParams>> {
    match note.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params = serde_json::from_value(note.params)?;
            super::sync::did_open(client, params, docs, parser)
        }
        DidChangeTextDocument::METHOD => {
            let params = serde_json::from_value(note.params)?;
            super::sync::did_change(client, params, docs, parser)
        }
        DidCloseTextDocument::METHOD => {
            let params = serde_json::from_value(note.params)?;
            Ok(super::sync::did_close(client, params, docs))
        }
        // doesn't make sense for a single-threaded impl
        Cancel::METHOD => Ok(None),
        _ => {
            eprintln!("[lsp] unhandled notification {note:?}");
            Ok(None)
        }
    }
}

/// sends an LSP notification to the client
fn notify<N>(conn: &Connection, params: N::Params) -> Result<(), SendError<Message>>
where
    N: lsp_types::notification::Notification,
    N::Params: Serialize,
{
    conn.sender.send(Message::Notification(Notification::new(
        N::METHOD.to_owned(),
        params,
    )))
}

/// responds successfully to an LSP client request
fn respond<R>(conn: &Connection, id: RequestId, result: R::Result) -> Result<(), SendError<Message>>
where
    R: lsp_types::request::Request,
    R::Result: Serialize,
{
    conn.sender
        .send(Message::Response(Response::new_ok(id, result)))
}

/// responds unsuccessfully to an LSP client request
fn error(
    conn: &Connection,
    id: RequestId,
    code: ErrorCode,
    msg: &str,
) -> Result<(), SendError<Message>> {
    conn.sender.send(Message::Response(Response::new_err(
        id,
        code as i32,
        msg.into(),
    )))
}
