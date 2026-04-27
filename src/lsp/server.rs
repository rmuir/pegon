use core::num::NonZero;
use std::{collections::HashMap, sync::Arc, thread};

use anyhow::{Context as _, Error, Result, bail};
use crossbeam_channel::Sender;
use line_index::LineIndex;
use ls_types::{
    CodeActionKind, CodeActionOptions, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, DiagnosticOptions, DiagnosticRegistrationOptions,
    DiagnosticServerCapabilities, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentDiagnosticParams, DocumentFilter, DocumentSymbolOptions,
    DocumentSymbolParams, FoldingRangeParams, FoldingRangeProviderCapability, InitializeResult,
    LogMessageParams, MessageType, OneOf, Registration, RegistrationParams, SelectionRangeOptions,
    SelectionRangeParams, SelectionRangeProviderCapability, SelectionRangeRegistrationOptions,
    ServerCapabilities, ServerInfo, StaticTextDocumentColorProviderOptions,
    StaticTextDocumentRegistrationOptions, TextDocumentChangeRegistrationOptions,
    TextDocumentRegistrationOptions, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, Uri, WorkDoneProgressOptions, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, LogMessage,
        Notification as _, PublishDiagnostics,
    },
    request::{
        CodeActionRequest, DocumentDiagnosticRequest, DocumentSymbolRequest, FoldingRangeRequest,
        RegisterCapability, Request as _, SelectionRangeRequest,
    },
};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use serde::{Deserialize, Serialize};
use tree_sitter::{Parser, Tree};

use crate::lsp::client::Client;

/// A Language Server Protocol Server
pub struct Server {
    connection: Connection,
    workers: ThreadPool,
}

/// A client-managed resource (file)
///
/// The client might notify us about files that aren't java. This can happen e.g. due to
/// wrong client configuration by the user. In such a case, an initial error is logged via
/// `window/logMessage`, but we track the URI resource to avoid spamming the logs with
/// subsequent false errors throughout the rest of the lifecycle.
pub enum Resource {
    /// A client-managed Java document.
    Java(Arc<Document>),
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

type Job = Box<dyn FnOnce() + Send + 'static>;

struct ThreadPool {
    sender: Sender<Job>,
}

impl ThreadPool {
    fn new(size: NonZero<usize>) -> Result<Self> {
        let (sender, receiver) = crossbeam_channel::unbounded::<Job>();
        for id in 0..size.get() {
            let receiver = receiver.clone();
            thread::Builder::new()
                .name(format!("lsp worker {id}").to_owned())
                .spawn(move || {
                    while let Ok(job) = receiver.recv() {
                        job();
                    }
                })?;
        }
        Ok(Self { sender })
    }

    fn execute<F>(&self, job: F) -> Result<()>
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .send(Box::new(job))
            .map_err(|err| Error::msg(format!("threadpool error: {err}")))
    }
}

impl Server {
    /// Initializes a new server
    pub fn new(connection: Connection, client: &Client, id: RequestId) -> Result<Self> {
        let document_selector = Some(vec![DocumentFilter {
            language: Some("java".into()),
            scheme: None,
            pattern: None,
        }]);
        let diagnostic_options = DiagnosticRegistrationOptions {
            diagnostic_options: DiagnosticOptions {
                identifier: Some(env!("CARGO_PKG_NAME").into()),
                ..DiagnosticOptions::default()
            },
            text_document_registration_options: TextDocumentRegistrationOptions {
                document_selector: document_selector.clone(),
            },
            ..DiagnosticRegistrationOptions::default()
        };
        let code_action_options = CodeActionOptions {
            code_action_kinds: Some(vec![
                CodeActionKind::QUICKFIX,
                CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
                CodeActionKind::new(concat!(env!("CARGO_PKG_NAME"), ".organizeImports")),
            ]),
            ..CodeActionOptions::default()
        };
        let document_symbol_options = DocumentSymbolOptions {
            label: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        };
        let folding_range_options = FoldingRangeRegistrationOptions {
            folding_range_options: WorkDoneProgressOptions::default(),
            registration_options: StaticTextDocumentRegistrationOptions {
                document_selector: document_selector.clone(),
                id: Some(FoldingRangeRequest::METHOD.to_owned()),
            },
        };
        let selection_range_options = SelectionRangeRegistrationOptions {
            selection_range_options: SelectionRangeOptions::default(),
            registration_options: StaticTextDocumentRegistrationOptions {
                document_selector: document_selector.clone(),
                id: Some(SelectionRangeRequest::METHOD.to_owned()),
            },
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
                code_action_provider: if client.registers_code_actions() {
                    None
                } else {
                    Some(CodeActionProviderCapability::Options(
                        code_action_options.clone(),
                    ))
                },
                diagnostic_provider: if client.registers_diagnostics() {
                    None
                } else {
                    Some(DiagnosticServerCapabilities::RegistrationOptions(
                        diagnostic_options.clone(),
                    ))
                },
                document_symbol_provider: if client.registers_document_symbols() {
                    None
                } else {
                    Some(OneOf::Right(document_symbol_options.clone()))
                },
                folding_range_provider: if client.registers_folding_range() {
                    None
                } else {
                    // TODO: lsp types are really broken here
                    Some(FoldingRangeProviderCapability::Options(
                        StaticTextDocumentColorProviderOptions {
                            document_selector: document_selector.clone(),
                            id: Some(FoldingRangeRequest::METHOD.to_owned()),
                        },
                    ))
                },
                position_encoding: Some(client.negotiated_encoding()),
                selection_range_provider: if client.registers_selection_range() {
                    None
                } else {
                    Some(SelectionRangeProviderCapability::RegistrationOptions(
                        selection_range_options.clone(),
                    ))
                },
                text_document_sync: if client.registers_sync() {
                    None
                } else {
                    Some(TextDocumentSyncCapability::Options(
                        TextDocumentSyncOptions {
                            open_close: Some(true),
                            change: Some(TextDocumentSyncKind::INCREMENTAL),
                            ..TextDocumentSyncOptions::default()
                        },
                    ))
                },
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
                register_options: Some(serde_json::to_value(diagnostic_options)?),
            });
        }
        if client.registers_document_symbols() {
            registrations.push(Registration {
                id: DocumentSymbolRequest::METHOD.to_owned(),
                method: DocumentSymbolRequest::METHOD.to_owned(),
                register_options: Some(serde_json::to_value(document_symbol_options)?),
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
        if client.registers_folding_range() {
            registrations.push(Registration {
                id: FoldingRangeRequest::METHOD.to_owned(),
                method: FoldingRangeRequest::METHOD.to_owned(),
                register_options: Some(serde_json::to_value(folding_range_options)?),
            });
        }
        if client.registers_selection_range() {
            registrations.push(Registration {
                id: SelectionRangeRequest::METHOD.to_owned(),
                method: SelectionRangeRequest::METHOD.to_owned(),
                register_options: Some(serde_json::to_value(selection_range_options)?),
            });
        }
        if !registrations.is_empty() {
            connection.sender.send(request::<RegisterCapability>(
                0.into(),
                RegistrationParams { registrations },
            ))?;
        }
        let default = NonZero::new(1).context("not zero")?;
        let limit = NonZero::new(8).context("not zero")?;
        let size = thread::available_parallelism().map_or(default, |val| val.min(limit));
        let workers = ThreadPool::new(size)?;
        Ok(Self {
            connection,
            workers,
        })
    }

    pub fn main_loop(&self, client: &Arc<Client>) -> Result<(), Error> {
        let mut state = State::new()?;

        for msg in &self.connection.receiver {
            match msg {
                Message::Request(req) => {
                    // try to go down gracefully, but always go down
                    if self.connection.handle_shutdown(&req)? {
                        break;
                    }
                    match self.handle_request(client, &req, &state.docs) {
                        Ok(()) => {}
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
                    match handle_notification(client.as_ref(), note, &mut state) {
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

    // handles an incoming request
    // every request must have an associated response
    fn handle_request(
        &self,
        client: &Arc<Client>,
        req: &Request,
        docs: &HashMap<String, Resource>,
    ) -> Result<()> {
        let id = req.id.clone();
        let client = Arc::clone(client);
        let sender = self.connection.sender.clone();
        match req.method.as_str() {
            CodeActionRequest::METHOD => {
                let params: CodeActionParams = serde_json::from_value(req.params.clone())?;
                let _doc = java_document(docs, &params.text_document.uri)?;
                let actions: Vec<CodeActionOrCommand> = vec![];
                sender.send(response::<CodeActionRequest>(id, Some(actions)))?;
                Ok(())
            }
            DocumentDiagnosticRequest::METHOD => {
                let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    let response =
                        match super::diagnostics::pull(client.as_ref(), doc.as_ref(), &params) {
                            Ok(result) => response::<DocumentDiagnosticRequest>(id, result),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        };
                    drop(sender.send(response));
                })
            }
            DocumentSymbolRequest::METHOD => {
                let params: DocumentSymbolParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    let response = match super::document_symbols::request(
                        client.as_ref(),
                        doc.as_ref(),
                        &params,
                    ) {
                        Ok(result) => response::<DocumentSymbolRequest>(id, Some(result)),
                        Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                    };
                    drop(sender.send(response));
                })
            }
            FoldingRangeRequest::METHOD => {
                let params: FoldingRangeParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    let response =
                        match super::folding_range::request(client.as_ref(), doc.as_ref()) {
                            Ok(result) => response::<FoldingRangeRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        };
                    drop(sender.send(response));
                })
            }
            SelectionRangeRequest::METHOD => {
                let params: SelectionRangeParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    let response = match super::selection_range::request(
                        client.as_ref(),
                        doc.as_ref(),
                        &params,
                    ) {
                        Ok(result) => response::<SelectionRangeRequest>(id, result),
                        Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                    };
                    drop(sender.send(response));
                })
            }
            _ => {
                sender.send(error(
                    id,
                    ErrorCode::MethodNotFound,
                    "unhandled request".to_owned(),
                ))?;
                Ok(())
            }
        }
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

/// returns open java document from the editor, or an error
fn java_document(docs: &HashMap<String, Resource>, uri: &Uri) -> Result<Arc<Document>> {
    match docs.get(&uri.to_string()) {
        Some(Resource::Java(doc)) => Ok(Arc::clone(doc)),
        Some(Resource::Other) => bail!("non-java document: {}", **uri),
        None => bail!("document not open: {}", **uri),
    }
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

/// Folding Range registration options.
///
/// @since 3.17.0
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoldingRangeRegistrationOptions {
    #[serde(flatten)]
    pub registration_options: StaticTextDocumentRegistrationOptions,

    #[serde(flatten)]
    pub folding_range_options: WorkDoneProgressOptions,
}
