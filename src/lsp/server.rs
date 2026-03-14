use std::collections::HashMap;

use anyhow::{Context as _, Error, Result, bail};
use line_index::LineIndex;
use ls_types::{
    CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, DiagnosticOptions, DiagnosticRegistrationOptions,
    DiagnosticServerCapabilities, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentDiagnosticParams, DocumentFilter, InitializeResult,
    LogMessageParams, MessageType, OneOf, Registration, RegistrationParams, ServerCapabilities,
    ServerInfo, StaticRegistrationOptions, TextDocumentChangeRegistrationOptions,
    TextDocumentRegistrationOptions, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, LogMessage,
        Notification as _, PublishDiagnostics,
    },
    request::{
        CodeActionRequest, DocumentDiagnosticRequest, Formatting, RegisterCapability, Request as _,
    },
};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use serde::{Deserialize, Serialize};
use tree_sitter::{Parser, Tree};

use crate::lsp::client::Client;

/// A Language Server Protocol Server
pub struct Server {
    connection: Connection,
}

/// A client-managed resource (file)
///
/// The client might notify us about files that aren't java. This can happen e.g. due to
/// wrong client configuration by the user. In such a case, an initial error is logged via
/// `window/logMessage`, but we track the URI resource to avoid spamming the logs with
/// subsequent false errors throughout the rest of the lifecycle.
pub enum Resource {
    /// A client-managed Java document.
    Java(Document),
    /// A client-managed document in some other language.
    Other,
}

/// A client-managed java document
pub struct Document {
    /// Full text of document
    pub(crate) text: String,
    /// Document's version
    pub(crate) version: i32,
    /// Parse tree
    pub(crate) tree: Tree,
    /// Index of newlines
    pub(crate) line_index: LineIndex,
}

pub struct State {
    pub(crate) docs: HashMap<String, Resource>,
    pub(crate) parser: Parser,
}

impl State {
    fn new() -> Result<Self> {
        let docs: HashMap<String, Resource> = HashMap::default();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&crate::LANGUAGE.into())?;
        Ok(Self { docs, parser })
    }
}

impl Server {
    /// Initializes a new server
    pub fn new(connection: Connection, client: &Client, id: RequestId) -> Result<Self> {
        let diagnostic_options = DiagnosticOptions {
            identifier: Some(env!("CARGO_PKG_NAME").into()),
            ..Default::default()
        };
        let code_action_options = CodeActionOptions {
            code_action_kinds: Some(vec![
                CodeActionKind::QUICKFIX,
                CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
                CodeActionKind::new(concat!(env!("CARGO_PKG_NAME"), ".organizeImports")),
            ]),
            ..Default::default()
        };
        // if the client supports dynamic registration of the capability, then we use that.
        // it makes the code very confusing, but this is just the pain of dynamic registration.
        // it allows pushing a filter for "java" language documents to the client, to avoid waste.
        // otherwise, advertise it statically, but not both! (see spec)
        let result = serde_json::json!(InitializeResult {
            offset_encoding: None,
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: if client.registers_sync() {
                    None
                } else {
                    Some(TextDocumentSyncCapability::Options(
                        TextDocumentSyncOptions {
                            open_close: Some(true),
                            change: Some(TextDocumentSyncKind::INCREMENTAL),
                            ..Default::default()
                        },
                    ))
                },
                diagnostic_provider: if client.registers_diagnostics() {
                    None
                } else {
                    Some(DiagnosticServerCapabilities::Options(
                        diagnostic_options.clone(),
                    ))
                },
                code_action_provider: if client.registers_code_actions() {
                    None
                } else {
                    Some(CodeActionProviderCapability::Options(
                        code_action_options.clone(),
                    ))
                },
                // use client's preferred encoding
                position_encoding: Some(client.negotiated_encoding()),
                // we don't care about classpaths or anything on disk, so advertise
                // the workspace support for better client-side reuse of the server.
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
        let mut registrations: Vec<Registration> = Vec::with_capacity(3);
        let document_selector = Some(vec![DocumentFilter {
            language: Some("java".into()),
            scheme: None,
            pattern: None,
        }]);
        if client.registers_sync() {
            registrations.push(Registration {
                id: DidOpenTextDocument::METHOD.to_owned(),
                method: DidOpenTextDocument::METHOD.to_owned(),
                register_options: Some(serde_json::to_value(TextDocumentRegistrationOptions {
                    document_selector: document_selector.clone(),
                })?),
            });
            registrations.push(Registration {
                id: DidChangeTextDocument::METHOD.to_owned(),
                method: DidChangeTextDocument::METHOD.to_owned(),
                register_options: Some(serde_json::to_value(
                    TextDocumentChangeRegistrationOptions {
                        document_selector: document_selector.clone(),
                        sync_kind: TextDocumentSyncKind::INCREMENTAL,
                    },
                )?),
            });
            registrations.push(Registration {
                id: DidCloseTextDocument::METHOD.to_owned(),
                method: DidCloseTextDocument::METHOD.to_owned(),
                register_options: Some(serde_json::to_value(TextDocumentRegistrationOptions {
                    document_selector: document_selector.clone(),
                })?),
            });
        }
        if client.registers_diagnostics() {
            registrations.push(Registration {
                id: DocumentDiagnosticRequest::METHOD.to_owned(),
                method: DocumentDiagnosticRequest::METHOD.to_owned(),
                register_options: Some(serde_json::to_value(DiagnosticRegistrationOptions {
                    text_document_registration_options: TextDocumentRegistrationOptions {
                        document_selector: document_selector.clone(),
                    },
                    diagnostic_options,
                    static_registration_options: StaticRegistrationOptions::default(),
                })?),
            });
        }
        if client.registers_code_actions() {
            registrations.push(Registration {
                id: CodeActionRequest::METHOD.to_owned(),
                method: CodeActionRequest::METHOD.to_owned(),
                register_options: Some(serde_json::to_value(CodeActionRegistrationOptions {
                    text_document_registration_options: TextDocumentRegistrationOptions {
                        document_selector,
                    },
                    code_action_options,
                })?),
            });
        }
        if !registrations.is_empty() {
            connection.sender.send(request::<RegisterCapability>(
                0.into(),
                RegistrationParams { registrations },
            ))?;
        }
        Ok(Self { connection })
    }

    pub fn main_loop(&self, client: &Client) -> Result<(), Error> {
        let mut state = State::new()?;

        for msg in &self.connection.receiver {
            match msg {
                Message::Request(req) => {
                    // try to go down gracefully, but always go down
                    if self.connection.handle_shutdown(&req)? {
                        break;
                    }
                    match handle_request(client, &req, &state.docs) {
                        Ok(response) => {
                            self.connection.sender.send(response)?;
                        }
                        Err(err) => {
                            self.connection.sender.send(error(
                                req.id.clone(),
                                ErrorCode::RequestFailed,
                                format!("{err:#}"),
                            ))?;
                        }
                    }
                }
                Message::Notification(note) => {
                    let method = note.method.clone();
                    match handle_notification(client, note, &mut state) {
                        Ok(Some(push)) => {
                            self.connection.sender.send(push)?;
                        }
                        Err(err) => {
                            self.connection
                                .sender
                                .send(log_error(&method, &format!("{err:#}")))?;
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
    req: &Request,
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
    state: &mut State,
) -> Result<Option<Message>> {
    let response = match note.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(note.params)?;
            let uri = params.text_document.uri.clone();
            super::sync::did_open(client, params, state).context(uri.to_string())?
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(note.params)?;
            let uri = params.text_document.uri.clone();
            super::sync::did_change(client, params, state).context(uri.to_string())?
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(note.params)?;
            let uri = params.text_document.uri.clone();
            super::sync::did_close(client, params, state).context(uri.to_string())?
        }
        // can be safely ignored according to specification
        method if method.starts_with("$/") => None,
        // log an error otherwise
        _ => bail!("unexpected notification"),
    }
    .map(notification::<PublishDiagnostics>);
    Ok(response)
}

/// creates a notification message to the client
fn notification<N>(params: N::Params) -> Message
where
    N: ls_types::notification::Notification,
    N::Params: Serialize,
{
    Message::Notification(Notification::new(N::METHOD.to_owned(), params))
}

// creates a request to the client
fn request<R>(id: RequestId, params: R::Params) -> Message
where
    R: ls_types::request::Request,
    R::Params: Serialize,
{
    Message::Request(Request::new(id, R::METHOD.to_owned(), params))
}

/// creates a successful response to the client
fn response<R>(id: RequestId, result: R::Result) -> Message
where
    R: ls_types::request::Request,
    R::Result: Serialize,
{
    Message::Response(Response::new_ok(id, result))
}

/// creates an unsuccessful response to the LSP client
fn error(id: RequestId, code: ErrorCode, message: String) -> Message {
    Message::Response(Response::new_err(id, code as i32, message))
}

/// logs via notification an error to the LSP client
fn log_error(method: &String, message: &String) -> Message {
    Message::Notification(Notification::new(
        LogMessage::METHOD.to_owned(),
        LogMessageParams {
            typ: MessageType::ERROR,
            message: format!("pegon[{method}]: {message}"),
        },
    ))
}

/// Code Action registration options.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeActionRegistrationOptions {
    #[serde(flatten)]
    pub text_document_registration_options: TextDocumentRegistrationOptions,

    #[serde(flatten)]
    pub code_action_options: CodeActionOptions,
}
