use anyhow::{Context as _, Error, Result};
use line_index::LineIndex;
use lsp_server::{Connection, ErrorCode, Message, Request as ServerRequest};
use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, DiagnosticOptions, DiagnosticServerCapabilities,
    DocumentDiagnosticParams, DocumentDiagnosticReportResult, InitializeResult, OneOf,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
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

pub struct Server {
    pub(crate) connection: Connection,
}

pub struct Document {
    pub(crate) text: String,
    pub(crate) version: i32,
    pub(crate) line_index: LineIndex,
    pub(crate) tree: Tree,
}

impl Server {
    pub fn initialize(&self, client: &Client) -> InitializeResult {
        InitializeResult {
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
        }
    }

    pub fn main_loop(&self, client: &Client) -> Result<(), Error> {
        let mut docs: FxHashMap<String, Document> = FxHashMap::default();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&crate::LANGUAGE.into())?;

        for msg in &self.connection.receiver {
            let start_time = Instant::now();
            match msg {
                Message::Request(req) => {
                    // try to go down gracefully, but always go down
                    if self.connection.handle_shutdown(&req)? {
                        return Ok(());
                    }
                    if let Err(err) = self.handle_request(client, &req, & /*mut*/ docs) {
                        eprintln!("[lsp] request {} failed: {err}", req.method);
                        super::error(
                            &self.connection,
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
                    if let Err(err) = self.handle_notification(client, note, &mut docs, &mut parser)
                    {
                        eprintln!("[lsp] notification {method} failed: {err}");
                    }
                    eprintln!(
                        "[notify] {}: {} ms",
                        method,
                        start_time.elapsed().as_millis()
                    );
                }

                // we can request workspaceEdit, but we don't care about the response.
                Message::Response(_) => {}
            }
        }
        Ok(())
    }

    fn handle_notification(
        &self,
        client: &Client,
        note: lsp_server::Notification,
        docs: &mut FxHashMap<String, Document>,
        parser: &mut Parser,
    ) -> Result<()> {
        match note.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params = serde_json::from_value(note.params)?;
                super::sync::did_open(&self.connection, client, params, docs, parser)
            }
            DidChangeTextDocument::METHOD => {
                let params = serde_json::from_value(note.params)?;
                super::sync::did_change(&self.connection, client, params, docs, parser)
            }
            DidCloseTextDocument::METHOD => {
                let params = serde_json::from_value(note.params)?;
                super::sync::did_close(&self.connection, client, params, docs)
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
        &self,
        client: &Client,
        req: &ServerRequest,
        docs: &FxHashMap<String, Document>,
    ) -> Result<()> {
        match req.method.as_str() {
            CodeActionRequest::METHOD => {
                let params: CodeActionParams = serde_json::from_value(req.params.clone())?;
                let uri = &params.text_document.uri;
                let _doc = docs.get(&uri.to_string()).context("document not open")?;
                let response: Vec<CodeActionOrCommand> = vec![];
                super::respond::<CodeActionRequest>(
                    &self.connection,
                    req.id.clone(),
                    Some(response),
                )?;
            }

            Formatting::METHOD => {
                todo!()
            }
            DocumentDiagnosticRequest::METHOD => {
                let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
                let uri = &params.text_document.uri;
                let doc = docs.get(&uri.to_string()).context("document not open")?;
                let response = super::diagnostics::pull(client, doc, &params)?;
                super::respond::<DocumentDiagnosticRequest>(
                    &self.connection,
                    req.id.clone(),
                    DocumentDiagnosticReportResult::Report(response),
                )?;
            }
            _ => {
                eprintln!("[lsp] unhandled request {req:?}");
                super::error(
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
