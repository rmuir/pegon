use std::{
    cell::{Cell, RefCell},
    str::FromStr,
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Error, Result};
use crossbeam_channel::{Receiver, after, select};
use lsp_server::{Connection, Message, Request as ServerRequest};
use lsp_types::{
    DidOpenTextDocumentParams, InitializeParams, InitializedParams, TextDocumentItem, Uri,
    notification::{DidOpenTextDocument, Exit, Initialized},
    request::{Initialize, Shutdown},
};
use pegon::lsp::start;
use serde::Serialize;
use serde_json::Value;

struct Client {
    req_id: Cell<i32>,
    messages: RefCell<Vec<Message>>,
    conn: Connection,
    #[allow(dead_code)]
    thread: JoinHandle<Result<(), Error>>,
}

impl Client {
    fn new() -> Self {
        let (client, server) = Connection::memory();
        let instance = Self {
            req_id: Cell::new(1),
            messages: RefCell::default(),
            conn: client,
            thread: thread::spawn(move || start(server)),
        };
        let response = instance.request::<Initialize>(InitializeParams::default());
        assert_ne!(response, Value::Null);
        instance.notify::<Initialized>(InitializedParams {});
        instance
    }

    pub(crate) fn notify<N>(&self, params: N::Params)
    where
        N: lsp_types::notification::Notification,
        N::Params: Serialize,
    {
        let r = lsp_server::Notification::new(N::METHOD.to_owned(), params);
        self.conn.sender.send(Message::Notification(r)).unwrap();
    }

    #[track_caller]
    pub(crate) fn request<R>(&self, params: R::Params) -> Value
    where
        R: lsp_types::request::Request,
        R::Params: Serialize,
    {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let r = ServerRequest::new(id.into(), R::METHOD.to_owned(), params);
        self.send_request_(&r)
    }

    #[track_caller]
    fn send_request_(&self, r: &ServerRequest) -> Value {
        let id = r.id.clone();
        self.conn.sender.send(r.clone().into()).unwrap();
        while let Some(msg) = self
            .recv()
            .unwrap_or_else(|Timeout| panic!("timeout: {r:?}"))
        {
            match msg {
                Message::Request(req) => {
                    panic!("unexpected request: {req:?}")
                }
                Message::Notification(_) => (),
                Message::Response(res) => {
                    assert_eq!(res.id, id);
                    if let Some(err) = res.error {
                        panic!("error response: {err:#?}");
                    }
                    return res.result.unwrap();
                }
            }
        }
        panic!("no response for {r:?}");
    }

    fn recv(&self) -> Result<Option<Message>, Timeout> {
        let msg = recv_timeout(&self.conn.receiver)?;
        let msg = msg.inspect(|msg| {
            self.messages.borrow_mut().push(msg.clone());
        });
        Ok(msg)
    }
}

struct Timeout;

fn recv_timeout(receiver: &Receiver<Message>) -> Result<Option<Message>, Timeout> {
    let timeout = Duration::from_secs(30);
    select! {
        recv(receiver) -> msg => Ok(msg.ok()),
        recv(after(timeout)) -> _ => Err(Timeout),
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        assert_eq!(Value::Null, self.request::<Shutdown>(()));
        self.notify::<Exit>(());
    }
}

#[test]
fn test_connect() {
    let client = Client::new();
    client.notify::<DidOpenTextDocument>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Uri::from_str("file:///Foo.java").unwrap(),
            language_id: "java".into(),
            version: 0,
            text: "public class foo {}".into(),
        },
    });
}

#[test]
fn test_hard_disconnect() {
    let (client, server) = Connection::memory();
    let server_thread = thread::spawn(move || start(server));
    drop(client);
    let err = server_thread.join().unwrap().unwrap_err();
    assert_eq!(err.to_string(), "disconnected channel");
}
