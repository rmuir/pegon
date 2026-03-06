#![expect(clippy::panic, reason = "tests")]

use core::cell::{Cell, RefCell};
use core::time::Duration;
use std::{
    io::ErrorKind,
    thread::{self, JoinHandle},
};

use anyhow::{Error, Result};
use crossbeam_channel::{Receiver, after, select};
use lsp_server::{Connection, Message, Request as ServerRequest};
use lsp_types::{
    InitializeParams, InitializeResult, InitializedParams,
    notification::{Exit, Initialized},
    request::{Initialize, Shutdown},
};
use pegon::lsp::start;
use serde::Serialize;
use serde_json::Value;

pub struct Client {
    req_id: Cell<i32>,
    messages: RefCell<Vec<Message>>,
    init_response: RefCell<Option<InitializeResult>>,
    conn: Connection,
    #[expect(dead_code, reason = "for the drop")]
    thread: JoinHandle<Result<(), Error>>,
}

impl Client {
    /// Creates a new language server [`Client`].
    pub fn new(params: InitializeParams) -> Self {
        let (client, server) = Connection::memory();
        let instance = Self {
            req_id: Cell::new(1),
            messages: RefCell::default(),
            init_response: RefCell::default(),
            conn: client,
            thread: thread::spawn(move || start(&server)),
        };
        let response = instance.request::<Initialize>(params);
        *instance.init_response.borrow_mut() = Some(response);
        instance.notify::<Initialized>(InitializedParams {});
        instance
    }

    /// Returns the init response from the server
    pub fn init_response(&self) -> InitializeResult {
        self.init_response
            .borrow()
            .clone()
            .expect("initialize occurred in new")
    }

    pub(crate) fn notify<N>(&self, params: N::Params)
    where
        N: lsp_types::notification::Notification,
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
        N: lsp_types::notification::Notification,
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
        R: lsp_types::request::Request,
        R::Params: Serialize,
    {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let req = ServerRequest::new(id.into(), R::METHOD.to_owned(), params);
        let value = self.send_request_(&req);
        serde_json::from_value(value).expect("able to deserialize")
    }

    fn send_request_(&self, req: &ServerRequest) -> Value {
        let id = req.id.clone();
        self.conn
            .sender
            .send(req.clone().into())
            .expect("able to send request");
        while let Some(msg) = self.recv().unwrap_or_else(|_| panic!("timeout: {req:?}")) {
            match msg {
                Message::Request(request) => {
                    panic!("unexpected request: {request:?}")
                }
                Message::Notification(_) => (),
                Message::Response(res) => {
                    assert_eq!(res.id, id);
                    if let Some(err) = res.error {
                        panic!("error response: {err:#?}");
                    }
                    return res.result.expect("able to deserialize");
                }
            }
        }
        panic!("no response for {req:?}");
    }

    fn recv(&self) -> Result<Option<Message>, ErrorKind> {
        let msg = recv_timeout(&self.conn.receiver)?;
        let msg = msg.inspect(|msg| {
            self.messages.borrow_mut().push(msg.clone());
        });
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

impl Drop for Client {
    fn drop(&mut self) {
        assert_eq!((), self.request::<Shutdown>(()));
        self.notify::<Exit>(());
    }
}
