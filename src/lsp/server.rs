//! Language Server Protocol Server
//!
//! The main thread handles notifications directly, and dispatches requests to a thread
//! pool of workers. Worker threads are backed by a queue, but dispatch guarantees the
//! worker thread always works the version of the document at the time the initial request
//! was received.
//!
//! Request cancellation works at a coarse level by checking state of in-flight requests
//! both before and after doing the work, to save both client and server resources when
//! possible.

use core::num::NonZero;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use std::{
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{Context as _, Error, Result, anyhow, bail};
use crossbeam_channel::Sender;
use gen_lsp_types::DidChangeWorkspaceFoldersParams;
use gen_lsp_types::SemanticTokensDeltaParams;
use gen_lsp_types::SemanticTokensDeltaRequest;
use gen_lsp_types::WorkspaceFolder;
use gen_lsp_types::{
    CancelParams, CodeAction, CodeActionParams, CodeActionRequest, CodeActionResolveRequest,
    DefinitionParams, DefinitionRequest, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentDiagnosticParams, DocumentDiagnosticRequest,
    DocumentHighlightParams, DocumentHighlightRequest, DocumentSymbolParams, DocumentSymbolRequest,
    FoldingRangeParams, FoldingRangeRequest, HoverParams, HoverRequest, Id, InlayHint,
    InlayHintParams, InlayHintRequest, InlayHintResolveRequest, LogMessageNotification,
    LogMessageParams, MessageType, Notification as _, PublishDiagnosticsNotification,
    RegistrationParams, RegistrationRequest, SelectionRangeParams, SelectionRangeRequest,
    SemanticTokensParams, SemanticTokensRangeParams, SemanticTokensRangeRequest,
    SemanticTokensRequest, Uri,
};
use line_index::LineIndex;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use rustc_hash::FxHashMap;
use serde::Serialize;
use tree_sitter::{Parser, Tree};

use super::client::Client;

/// A Language Server Protocol Server
pub struct Server {
    /// LSP connection to the client (editor)
    connection: Connection,
    /// Pool of workers for answering requests
    workers: ThreadPool,
    /// Current in-flight requests, either queued or being processed by workers
    in_flight: InFlight,
    /// Cache of recent semantic tokens responses (for delta purposes)
    semantic_cache: Arc<super::semantic_cache::Cache>,
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
///
/// An immutable snapshot
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

/// A workspace folder
pub struct Workspace {
    #[expect(dead_code, reason = "not yet")]
    pub root: Uri,
}

/// LSP state, only accessed by the main thread
pub struct State {
    /// Map of documents currently opened by the editor, keyed by URI
    pub docs: FxHashMap<String, Resource>,
    /// Treesitter parser used for parsing opened/modified documents
    pub parser: Parser,
    /// List of workspace folders, keyed by name
    pub workspaces: FxHashMap<String, Workspace>,
}

impl State {
    fn new(folders: &[WorkspaceFolder]) -> Result<Self> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&crate::support::language())?;
        Ok(Self {
            parser,
            docs: FxHashMap::default(),
            workspaces: folders
                .iter()
                .map(|folder| {
                    (
                        folder.name.clone(),
                        Workspace {
                            root: folder.uri.clone(),
                        },
                    )
                })
                .collect(),
        })
    }
}

/// Map of in-flight requests to their cancellation status
type InFlight = Arc<Mutex<FxHashMap<RequestId, Arc<AtomicBool>>>>;
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
            in_flight: Arc::new(Mutex::new(FxHashMap::default())),
            semantic_cache: Arc::new(super::semantic_cache::Cache::default()),
        })
    }

    /// main thread LSP server loop
    ///
    /// main thread pulls off new requests and notifications.
    /// notifications are handled by the main thread directly, since they cause a state change.
    /// requests are dispatched to the thread pool.
    pub fn main_loop(&self, client: &Arc<Client>) -> Result<(), Error> {
        let mut state = State::new(&client.workspace_folders())?;

        for msg in &self.connection.receiver {
            match msg {
                Message::Request(req) => {
                    // try to go down gracefully, but always go down
                    if self.connection.handle_shutdown(&req)? {
                        break;
                    }
                    let cancel = Arc::new(AtomicBool::new(false));
                    self.in_flight
                        .lock()
                        .map_err(|err| anyhow!("poisoned: {err}"))?
                        .insert(req.id.clone(), Arc::clone(&cancel));
                    match self.handle_request(client, &req, &state.docs, &cancel) {
                        Ok(()) => {}
                        Err(err) => {
                            // error during dispatch (e.g. parsing params or something)
                            // finalize the request since it didn't make it to threadpool.
                            self.connection.sender.send(finish_request(
                                &cancel,
                                &self.in_flight,
                                req.id.clone(),
                                error(req.id.clone(), ErrorCode::RequestFailed, format!("{err:#}")),
                            ))?;
                        }
                    }
                }
                Message::Notification(note) => {
                    let method = note.method.clone();
                    match self.handle_notification(client, note, &mut state) {
                        Ok(Some(push)) => {
                            self.connection.sender.send(push)?;
                        }
                        Err(err) => {
                            // error processing notification! communicate it to the editor...
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

    /// Handle an incoming request
    ///
    /// This function returns quickly, it queues the processing to happen on the threadpool.
    /// Each handler has a certain structure, which only differs slightly for annoying reasons
    /// and special cases in the protocol:
    ///
    /// 1. deserialize parameters. at least enough to know document's URI.
    /// 2. retrieve document, handling errors such as not-open document, non-java document, etc.
    /// 3. enqueue job on threadpool
    ///
    /// Once on the threadpool, worker "owns" the request and does these basic steps:
    ///
    /// 1. check if request has been cancelled: it could have been sitting on the queue for a bit.
    /// 2. invoke handler, passing cancellation token for periodic checks / early termination.
    /// 3. finalize request: send response, `RequestFailed`, or `RequestCancelled`.
    fn handle_request(
        &self,
        client: &Arc<Client>,
        req: &Request,
        docs: &FxHashMap<String, Resource>,
        cancel: &Arc<AtomicBool>,
    ) -> Result<()> {
        let id = req.id.clone();
        let client = Arc::clone(client);
        let sender = self.connection.sender.clone();
        let in_flight = Arc::clone(&self.in_flight);
        let cancel = Arc::clone(cancel);
        match req.method.as_str() {
            "textDocument/codeAction" => {
                let params: CodeActionParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::code_action::request(&client, &doc, &params) {
                            Ok(result) => response::<CodeActionRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "codeAction/resolve" => {
                let params: CodeAction = serde_json::from_value(req.params.clone())?;
                let data: super::code_action::CustomData = serde_json::from_value(
                    params
                        .data
                        .as_ref()
                        .context("data should be preserved")?
                        .clone(),
                )?;
                let doc = java_document(docs, &data.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::code_action::resolve(&client, &doc, &params, &data, &cancel) {
                            Ok(result) => response::<CodeActionResolveRequest>(id, result),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/definition" => {
                let params: DefinitionParams = serde_json::from_value(req.params.clone())?;
                let uri = &params.text_document_position_params.text_document.uri;
                let doc = java_document(docs, uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::definition::request(&client, &doc, &params, &cancel) {
                            Ok(result) => response::<DefinitionRequest>(id, result),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/diagnostic" => {
                let params: DocumentDiagnosticParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::diagnostics::pull(&client, &doc, &params, &cancel) {
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
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::document_highlight::request(&client, &doc, &params, &cancel) {
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
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::document_symbols::request(&client, &doc, &params, &cancel) {
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
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::folding_range::request(&client, &doc, &cancel) {
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
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::hover::request(&client, &doc, position, &cancel) {
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
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::inlay_hints::request(&client, &doc, &params, &cancel) {
                            Ok(result) => response::<InlayHintRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "inlayHint/resolve" => {
                let params: InlayHint = serde_json::from_value(req.params.clone())?;
                let data: super::inlay_hints::CustomData = serde_json::from_value(
                    params
                        .data
                        .as_ref()
                        .context("data should be preserved")?
                        .clone(),
                )?;
                let doc = java_document(docs, &data.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::inlay_hints::resolve(&client, &doc, &params, &data, &cancel) {
                            Ok(result) => response::<InlayHintResolveRequest>(id, result),
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
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::selection_range::request(&client, &doc, &params) {
                            Ok(result) => response::<SelectionRangeRequest>(id, result),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/semanticTokens/full" => {
                let params: SemanticTokensParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                let cache = Arc::clone(&self.semantic_cache);
                self.workers.execute(move || {
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::semantic_tokens::full(&client, &doc, &params, &cancel, &cache)
                        {
                            Ok(result) => response::<SemanticTokensRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/semanticTokens/full/delta" => {
                let params: SemanticTokensDeltaParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                let cache = Arc::clone(&self.semantic_cache);
                self.workers.execute(move || {
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::semantic_tokens::delta(&client, &doc, &params, &cancel, &cache)
                        {
                            Ok(result) => response::<SemanticTokensDeltaRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            "textDocument/semanticTokens/range" => {
                let params: SemanticTokensRangeParams = serde_json::from_value(req.params.clone())?;
                let doc = java_document(docs, &params.text_document.uri)?;
                self.workers.execute(move || {
                    if let Some(response) = start_request(&cancel, &in_flight, &id) {
                        drop(sender.send(response));
                        return;
                    }
                    let response = finish_request(
                        &cancel,
                        &in_flight,
                        id.clone(),
                        match super::semantic_tokens::range(&client, &doc, &params, &cancel) {
                            Ok(result) => response::<SemanticTokensRangeRequest>(id, Some(result)),
                            Err(err) => error(id, ErrorCode::RequestFailed, format!("{err:#}")),
                        },
                    );
                    drop(sender.send(response));
                })
            }
            _ => {
                sender.send(finish_request(
                    &cancel,
                    &self.in_flight,
                    id.clone(),
                    error(id, ErrorCode::MethodNotFound, "unhandled request".into()),
                ))?;
                Ok(())
            }
        }
    }

    /// Handle an incoming notification.
    ///
    /// These notifications are worked on the main thread directly. This
    /// ensures requests see the correct versions of documents.
    ///
    /// In our case notification has an "optional response".
    /// if the client doesn't support pull diagnostics then we've got
    /// a push diagnostics "response" that we'll `notify()` back.
    fn handle_notification(
        &self,
        client: &Client,
        note: lsp_server::Notification,
        state: &mut State,
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
            "workspace/didChangeWorkspaceFolders" => {
                let params: DidChangeWorkspaceFoldersParams = serde_json::from_value(note.params)?;
                for folder in params.event.removed {
                    if state.workspaces.remove(&folder.name).is_none() {
                        bail!("removed nonexistent workspace folder");
                    }
                }
                for folder in params.event.added {
                    if state
                        .workspaces
                        .insert(folder.name, Workspace { root: folder.uri })
                        .is_some()
                    {
                        bail!("added existing workspace folder");
                    }
                }
                None
            }
            "$/cancelRequest" => {
                let params: CancelParams = serde_json::from_value(note.params)?;
                let request_id: RequestId = match params.id {
                    Id::Int(id) => id.into(),
                    Id::String(id) => id.into(),
                };
                if let Some(cancelled) = self
                    .in_flight
                    .lock()
                    .map_err(|err| anyhow!("poisoned: {err}"))?
                    .get(&request_id)
                {
                    cancelled.store(true, Ordering::Relaxed);
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
}

/// returns a cancellation response when the request was already cancelled in the queue
fn start_request(
    cancel: &Arc<AtomicBool>,
    in_flight: &InFlight,
    id: &RequestId,
) -> Option<Message> {
    cancel.load(Ordering::Relaxed).then(|| {
        finish_request(
            cancel,
            in_flight,
            id.clone(),
            error(id.clone(), ErrorCode::RequestCanceled, "cancelled".into()),
        )
    })
}

/// returns response, unless the request was cancelled
fn finish_request(
    cancel: &Arc<AtomicBool>,
    in_flight: &InFlight,
    id: RequestId,
    response: Message,
) -> Message {
    in_flight.lock().expect("poisoned").remove(&id);
    if cancel.load(Ordering::Relaxed) {
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
fn java_document(docs: &FxHashMap<String, Resource>, uri: &Uri) -> Result<Arc<Document>> {
    match docs.get(&uri.to_string()) {
        Some(Resource::Java(doc)) => Ok(Arc::clone(doc)),
        Some(Resource::Other) => bail!("non-java document: {uri}"),
        None => bail!("document not open: {uri}"),
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use crate::lsp::run_server;
    use lsp_server::Connection;

    /// make sure if the stream disconnects that the error makes it out
    /// this ensure no leftover processes, which will annoy users!
    #[test]
    fn hard_disconnect() {
        let (client, server) = Connection::memory();
        let server_thread = thread::spawn(move || run_server(server));
        drop(client);
        let err = server_thread.join().unwrap().unwrap_err();
        assert_eq!(err.to_string(), "disconnected channel");
    }
}
