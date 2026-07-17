use core::cell::{Cell, RefCell};
use core::time::Duration;
use std::io::ErrorKind;
use std::thread::{self, JoinHandle};

use anyhow::{Error, Result};
use crossbeam_channel::{Receiver, after, select};
use gen_lsp_types::ExecuteCommandRequest;
use gen_lsp_types::{ExecuteCommandParams, RegistrationParams};
use gen_lsp_types::{
    ExitNotification, InitializeParams, InitializeRequest, InitializeResult,
    InitializedNotification, InitializedParams, ShutdownRequest,
};
use lsp_server::{Connection, Message, Request, Response};
use serde::Serialize;
use serde_json::Value;

/// LSP client for test purposes
///
/// slimmed down and reworked from rust-analyzer test code
pub struct TestClient {
    /// counter for request IDs that we make
    req_id: Cell<i32>,
    /// response to `initialize()` with server capabilities
    init_response: RefCell<Option<InitializeResult>>,
    /// dynamic registrations from the server, if any
    registrations: RefCell<Option<RegistrationParams>>,
    /// Connection to Server
    conn: Connection,
    /// Server run in separate thread
    #[expect(unused, reason = "for the drop")]
    thread: JoinHandle<Result<(), Error>>,
}

impl TestClient {
    /// Creates a new language server [`TestClient`].
    #[must_use]
    pub fn new(params: InitializeParams) -> Self {
        let (client, server) = Connection::memory();
        let instance = Self {
            req_id: Cell::new(1),
            init_response: RefCell::default(),
            registrations: RefCell::default(),
            conn: client,
            thread: thread::spawn(move || super::run_server(server)),
        };
        // initialize with the server and save the results
        let response = instance.request::<InitializeRequest>(params);
        *instance.init_response.borrow_mut() = Some(response);
        instance.notify::<InitializedNotification>(InitializedParams {});
        // ensure dynamic registration is complete
        instance.request::<ExecuteCommandRequest>(ExecuteCommandParams {
            command: "bogus".into(),
            arguments: Some(vec![]),
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

    /// Send a notification to the server
    pub fn notify<N>(&self, params: N::Params)
    where
        N: gen_lsp_types::Notification,
        N::Params: Serialize,
    {
        let notification = lsp_server::Notification::new(N::METHOD.into(), params);
        self.conn
            .sender
            .send(Message::Notification(notification))
            .expect("able to send notification");
    }

    /// Read a pending notification from the server
    pub fn read_notify<N>(&self) -> N::Params
    where
        N: gen_lsp_types::Notification,
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

    /// Send a request to the server and return the response
    pub fn request<R>(&self, params: R::Params) -> R::Result
    where
        R: gen_lsp_types::Request,
        R::Params: Serialize,
    {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let req = Request::new(id.into(), R::METHOD.into(), params);
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
                    return match res.response_result {
                        Ok(result) => result,
                        Err(error) => serde_json::to_value(error).expect("should serialize"),
                    };
                }
            }
        }
        panic!("no response for {req:?}");
    }

    fn process_request(&self, request: Request) {
        match request.method.as_str() {
            "client/registerCapability" => {
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

impl Drop for TestClient {
    fn drop(&mut self) {
        assert_eq!((), self.request::<ShutdownRequest>(()));
        self.notify::<ExitNotification>(());
    }
}
