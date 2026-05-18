#![expect(clippy::panic, reason = "tests")]

use core::cell::{Cell, RefCell};
use core::time::Duration;
use std::{
    io::ErrorKind,
    thread::{self, JoinHandle},
};

use anyhow::{Error, Result};
use crossbeam_channel::{Receiver, after, select};
use ls_types::request::{ExecuteCommand, RegisterCapability, Request as _};
use ls_types::{ExecuteCommandParams, RegistrationParams};
use ls_types::{
    InitializeParams, InitializeResult, InitializedParams,
    notification::{Exit, Initialized},
    request::{Initialize, Shutdown},
};
use lsp_server::{Connection, Message, Request, Response};
use pegon::lsp::start;
use serde::Serialize;
use serde_json::Value;

/// slimmed down and reworked from rust-analyzer test code
pub struct LspClient {
    /// counter for request IDs that we make
    req_id: Cell<i32>,
    /// response to `initialize()` with server capabilities
    init_response: RefCell<Option<InitializeResult>>,
    /// dynamic registrations from the server, if any
    registrations: RefCell<Option<RegistrationParams>>,
    /// Connection to Server
    conn: Connection,
    /// Server run in separate thread
    #[expect(dead_code, reason = "for the drop")]
    thread: JoinHandle<Result<(), Error>>,
}

impl LspClient {
    /// Creates a new language server [`LspClient`].
    #[must_use]
    pub fn new(params: InitializeParams) -> Self {
        let (client, server) = Connection::memory();
        let instance = Self {
            req_id: Cell::new(1),
            init_response: RefCell::default(),
            registrations: RefCell::default(),
            conn: client,
            thread: thread::spawn(move || start(server)),
        };
        // initialize with the server and save the results
        let response = instance.request::<Initialize>(params);
        *instance.init_response.borrow_mut() = Some(response);
        instance.notify::<Initialized>(InitializedParams {});
        // ensure dynamic registration is complete
        instance.request::<ExecuteCommand>(ExecuteCommandParams {
            command: "bogus".to_owned(),
            arguments: vec![],
            ..Default::default()
        });
        instance
    }

    /// Returns the init response from the server
    pub fn init_response(&self) -> InitializeResult {
        self.init_response
            .borrow()
            .clone()
            .expect("initialize occurred in new")
    }

    /// Returns dynamic registrations from the server
    pub fn registrations(&self) -> Option<RegistrationParams> {
        self.registrations.borrow().clone()
    }

    pub(crate) fn notify<N>(&self, params: N::Params)
    where
        N: ls_types::notification::Notification,
        N::Params: Serialize,
    {
        let notification = lsp_server::Notification::new(N::METHOD.to_owned(), params);
        self.conn
            .sender
            .send(Message::Notification(notification))
            .expect("able to send notification");
    }

    pub fn read_notify<N>(&self) -> N::Params
    where
        N: ls_types::notification::Notification,
        N::Params: Serialize,
    {
        let message = self
            .recv()
            .expect("able to read message")
            .expect("able to deserialize");
        let Message::Notification(msg) = message else {
            panic!();
        };
        serde_json::from_value(msg.params).expect("able to deserialize")
    }

    pub fn request<R>(&self, params: R::Params) -> R::Result
    where
        R: ls_types::request::Request,
        R::Params: Serialize,
    {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let req = Request::new(id.into(), R::METHOD.to_owned(), params);
        let value = self.send_request_(&req);
        serde_json::from_value(value).expect("able to deserialize")
    }

    fn send_request_(&self, req: &Request) -> Value {
        let id = req.id.clone();
        self.conn
            .sender
            .send(req.clone().into())
            .expect("able to send request");
        while let Some(msg) = self.recv().unwrap_or_else(|_| panic!("timeout: {req:?}")) {
            match msg {
                Message::Request(request) => self.process_request(request),
                Message::Notification(_) => (),
                Message::Response(res) => {
                    assert_eq!(res.id, id);
                    if let Some(err) = res.error {
                        return serde_json::to_value(err).expect("should serialize");
                    }
                    return res.result.expect("able to deserialize");
                }
            }
        }
        panic!("no response for {req:?}");
    }

    fn process_request(&self, request: Request) {
        match request.method.as_str() {
            RegisterCapability::METHOD => {
                let params: RegistrationParams =
                    serde_json::from_value(request.params).expect("could deserialize");
                *self.registrations.borrow_mut() = Some(params);
                self.conn
                    .sender
                    .send(Message::Response(Response::new_ok::<()>(request.id, ())))
                    .expect("able to send response");
            }
            _ => panic!("unexpected request: {request:?}"),
        }
    }

    fn recv(&self) -> Result<Option<Message>, ErrorKind> {
        let msg = recv_timeout(&self.conn.receiver)?;
        Ok(msg)
    }
}

fn recv_timeout(receiver: &Receiver<Message>) -> Result<Option<Message>, ErrorKind> {
    let timeout = Duration::from_secs(30);
    select! {
        recv(receiver) -> msg => Ok(msg.ok()),
        recv(after(timeout)) -> _ => Err(ErrorKind::TimedOut),
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        assert_eq!((), self.request::<Shutdown>(()));
        self.notify::<Exit>(());
    }
}
