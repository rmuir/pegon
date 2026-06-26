use core::num::NonZero;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{Context as _, Error, Result, anyhow, bail};
use crossbeam_channel::Sender;
use gen_lsp_types::{
    CancelParams, CodeAction, CodeActionParams, CodeActionRequest, CodeActionResolveRequest,
    DefinitionParams, DefinitionRequest, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentDiagnosticParams, DocumentDiagnosticRequest,
    DocumentHighlightParams, DocumentHighlightRequest, DocumentSymbolParams, DocumentSymbolRequest,
    FoldingRangeParams, FoldingRangeRequest, HoverParams, HoverRequest, Id, InlayHintParams,
    InlayHintRequest, LogMessageNotification, LogMessageParams, MessageType, Notification as _,
    PublishDiagnosticsNotification, RegistrationParams, RegistrationRequest, SelectionRangeParams,
    SelectionRangeRequest, Uri,
};
use line_index::LineIndex;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use serde::Serialize;
use tree_sitter::{Parser, Tree};

use super::client::Client;

/// A Language Server Protocol Server
///
/// The main thread handles notifications directly, and dispatches requests to a thread
/// pool of workers. Worker threads are backed by a queue, but dispatch guarantees the
/// worker thread always works the version of the document at the time the initial request
/// was received.
///
/// Request cancellation works at a coarse level by checking `in_flight` both before and
/// after doing the work, to save both client and server resources when possible.
pub struct Server {
    /// LSP connection to the client (editor)
    connection: Connection,
    /// Pool of workers for answering requests
    workers: ThreadPool,
    /// Current in-flight requests, either queued or being processed by workers
    in_flight: InFlight,
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
    pub text: String,
    /// Document's version
    pub version: i32,
    /// Parse tree
    pub tree: Tree,
    /// Index of newlines
    pub line_index: LineIndex,
}

/// LSP state, only accessed by the main thread
pub struct State {
    /// Map of documents currently opened by the editor, keyed by URI
    pub docs: HashMap<String, Resource>,
    /// Treesitter parser used for parsing opened/modified documents
    pub parser: Parser,
}

impl State {
    fn new() -> Result<Self> {
        let docs: HashMap<String, Resource> = HashMap::default();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&crate::support::LANGUAGE.into())?;
        Ok(Self { docs, parser })
    }
}

/// Map of in-flight requests to their cancellation status
type InFlight = Arc<Mutex<HashMap<RequestId, bool>>>;
/// Job handled by the request thread pool
type Job = Box<dyn FnOnce() + Send + 'static>;

/// Request thread pool
struct ThreadPool {
    sender: Sender<Job>,
}

impl ThreadPool {
    /// Create a new pool of the specified size
    fn new(size: NonZero<usize>) -> Result<Self> {
        let (sender, receiver) = crossbeam_channel::unbounded::<Job>();
        for id in 0..size.get() {
            let receiver = receiver.clone();
            thread::Builder::new()
                .name(format!("lsp worker {id}"))
                .spawn(move || {
                    while let Ok(job) = receiver.recv() {
                        job();
                    }
                })?;
        }
        Ok(Self { sender })
    }

    /// Enqueue a new job to be executed by the pool
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
        let (initialize_result, registrations) = super::initialize::init(client)?;
        connection.initialize_finish(id, serde_json::json!(initialize_result))?;
        if !registrations.is_empty() {
            connection.sender.send(request::<RegistrationRequest>(
                0.into(),
                RegistrationParams { registrations },
            ))?;
        }
        let default = NonZero::new(1).context("not zero")?;
        let size = thread::available_parallelism().unwrap_or(default);
        let workers = ThreadPool::new(size)?;
        Ok(Self {
            connection,
            workers,
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// main thread LSP server loop
    ///
    /// main thread pulls off new requests and notifications.
    /// notifications are handled by the main thread directly, since they cause a state change
    /// requests are dispatched to the thread pool
    pub fn main_loop(&self, client: &Arc<Client>) -> Result<(), Error> {
        let mut state = State::new()?;

        for msg in &self.connection.receiver {
            match msg {
                Message::Request(req) => {
                    // try to go down gracefully, but always go down
                    if self.connection.handle_shutdown(&req)? {
                        break;
                    }
                    self.in_flight
                        .lock()
                        .map_err(|err| anyhow!("poisoned: {err}"))?
                        .insert(req.id.clone(), false);
                    match self.handle_request(client, &req, &state.docs) {
                        Ok(()) => {}
                        Err(err) => {
                            self.connection.sender.send(finish_request(
                                &self.in_flight,
                                req.id.clone(),
                                error(req.id.clone(), ErrorCode::RequestFailed, format!("{err:#}")),
                            ))?;
                        }
                    }
                }
                Message::Notification(note) => {
                    let method = note.method.clone();
                    match handle_notification(client.as_ref(), note, &mut state, &self.in_flight) {
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

    /// Handles an incoming request
    fn handle_request(
        &self,
        client: &Arc<Client>,
        req: &Request,
        docs: &HashMap<String, Resource>,
    ) -> Result<()> {
        let id = req.id.clone();
        let client = Arc::clone(client);
        let sender = self.connection.sender.clone();
        let in_flight = Arc::clone(&self.in_flight);
        match req.method.as_str() {
            "textDocument/codeAction" => {
                let params: CodeActionParams = serde_json::from_value(req.params.clone())?;
                let _doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        response::<CodeActionRequest>(id, Some(vec![])),
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/definition" => {
                let params: DefinitionParams = serde_json::from_value(req.params.clone())?;
                let uri = &params.text_document_position_params.text_document.uri;
                let doc = java_document(docs, uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        match super::definition::request(client.as_ref(), doc.as_ref(), &params) {
                            Ok(result) => response::<DefinitionRequest>(id, result),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "codeAction/resolve" => {
                // TODO: deserialize 'data' and process
                let params: CodeAction = serde_json::from_value(req.params.clone())?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        response::<CodeActionResolveRequest>(id, params),
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/diagnostic" => {
                let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        match super::diagnostics::pull(client.as_ref(), doc.as_ref(), &params) {
                            Ok(result) => response::<DocumentDiagnosticRequest>(id, result),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/documentHighlight" => {
                let params: DocumentHighlightParams = serde_json::from_value(req.params.clone())?;
                let uri = &params.text_document_position_params.text_document.uri;
                let doc = java_document(docs, uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        match super::document_highlight::request(
                            client.as_ref(),
                            doc.as_ref(),
                            &params,
                        ) {
                            Ok(result) => response::<DocumentHighlightRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/documentSymbol" => {
                let params: DocumentSymbolParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        match super::document_symbols::request(
                            client.as_ref(),
                            doc.as_ref(),
                            &params,
                        ) {
                            Ok(result) => response::<DocumentSymbolRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/foldingRange" => {
                let params: FoldingRangeParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        match super::folding_range::request(client.as_ref(), doc.as_ref()) {
                            Ok(result) => response::<FoldingRangeRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/hover" => {
                let params: HoverParams = serde_json::from_value(req.params.clone())?;
                let uri = &params.text_document_position_params.text_document.uri;
                let doc = java_document(docs, uri)?;
                let position = params.text_document_position_params.position;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        match super::hover::request(client.as_ref(), doc.as_ref(), position) {
                            Ok(result) => response::<HoverRequest>(id, result),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/inlayHint" => {
                let params: InlayHintParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        match super::inlay_hints::request(client.as_ref(), doc.as_ref(), &params) {
                            Ok(result) => response::<InlayHintRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/selectionRange" => {
                let params: SelectionRangeParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &in_flight,
                        id.clone(),
                        match super::selection_range::request(
                            client.as_ref(),
                            doc.as_ref(),
                            &params,
                        ) {
                            Ok(result) => response::<SelectionRangeRequest>(id, result),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            _ => {
                sender.send(finish_request(
                    &self.in_flight,
                    id.clone(),
                    error(id, ErrorCode::MethodNotFound, "unhandled request".into()),
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
    in_flight: &InFlight,
) -> Result<Option<Message>> {
    let response = match note.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(note.params)?;
            let uri = params.text_document.uri.clone();
            super::sync::did_open(client, params, state).context(uri.to_string())?
        }
        "textDocument/didChange" => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(note.params)?;
            let uri = params.text_document.text_document_identifier.uri.clone();
            super::sync::did_change(client, params, state).context(uri.to_string())?
        }
        "textDocument/didClose" => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(note.params)?;
            let uri = params.text_document.uri.clone();
            super::sync::did_close(client, params, state).context(uri.to_string())?
        }
        "$/cancelRequest" => {
            let params: CancelParams = serde_json::from_value(note.params)?;
            let request_id: RequestId = match params.id {
                Id::Int(id) => id.into(),
                Id::String(id) => id.into(),
            };
            if let Some(cancelled) = in_flight
                .lock()
                .map_err(|err| anyhow!("poisoned: {err}"))?
                .get_mut(&request_id)
            {
                *cancelled = true;
            }
            None
        }
        // can be safely ignored according to specification
        method if method.starts_with("$/") => None,
        // log an error otherwise
        _ => bail!("unexpected notification"),
    }
    .map(notification::<PublishDiagnosticsNotification>);
    Ok(response)
}

/// returns a cancellation response when the request was already cancelled in the queue
fn start_request(in_flight: &InFlight, id: &RequestId) -> Option<Message> {
    let cancelled = { in_flight.lock().expect("poisoned").get(id).copied() };
    cancelled.unwrap_or_default().then(|| {
        finish_request(
            in_flight,
            id.clone(),
            error(id.clone(), ErrorCode::RequestCanceled, "cancelled".into()),
        )
    })
}

/// returns response, unless the request was cancelled
fn finish_request(in_flight: &InFlight, id: RequestId, response: Message) -> Message {
    let cancelled = { in_flight.lock().expect("poisoned").remove(&id) };
    if cancelled.unwrap_or_default() {
        error(id, ErrorCode::RequestCanceled, "cancelled".into())
    } else {
        response
    }
}

/// creates a notification message to the client
fn notification<N>(params: N::Params) -> Message
where
    N: gen_lsp_types::Notification,
    N::Params: Serialize,
{
    Message::Notification(Notification::new(N::METHOD.to_string(), params))
}

/// creates a request to the client
fn request<R>(id: RequestId, params: R::Params) -> Message
where
    R: gen_lsp_types::Request,
    R::Params: Serialize,
{
    Message::Request(Request::new(id, R::METHOD.to_string(), params))
}

/// creates a successful response to the client
fn response<R>(id: RequestId, result: R::Result) -> Message
where
    R: gen_lsp_types::Request,
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
        LogMessageNotification::METHOD.into(),
        LogMessageParams {
            kind: MessageType::Error,
            message: format!("pegon[{method}]: {message}"),
        },
    ))
}

/// returns open java document from the editor, or an error
fn java_document(docs: &HashMap<String, Resource>, uri: &Uri) -> Result<Arc<Document>> {
    match docs.get(&uri.to_string()) {
        Some(Resource::Java(doc)) => Ok(Arc::clone(doc)),
        Some(Resource::Other) => bail!("non-java document: {uri}"),
        None => bail!("document not open: {uri}"),
    }
}
