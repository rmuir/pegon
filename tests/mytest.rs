use std::{
    cell::{Cell, RefCell},
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Error, Result};
use crossbeam_channel::{Receiver, after, select};
use lsp_server::{Connection, Message, Request as ServerRequest};
use lsp_types::{
    InitializeParams, InitializedParams,
    notification::{Exit, Initialized, Notification},
    request::{Initialize, Request, Shutdown},
};
use pegon::lsp::start;
use serde::Serialize;
use serde_json::{Value, to_string_pretty, to_value};

struct Server {
    req_id: Cell<i32>,
    messages: RefCell<Vec<Message>>,
    thread: JoinHandle<Result<(), Error>>,
    client: Connection,
}

impl Server {
    pub(crate) fn notification<N>(&self, params: N::Params)
    where
        N: lsp_types::notification::Notification,
        N::Params: Serialize,
    {
        let r = lsp_server::Notification::new(N::METHOD.to_owned(), params);
        self.send_notification(r)
    }

    fn send_notification(&self, not: lsp_server::Notification) {
        self.client.sender.send(Message::Notification(not)).unwrap();
    }

    #[track_caller]
    pub(crate) fn request<R>(&self, params: R::Params, expected_resp: Value)
    where
        R: lsp_types::request::Request,
        R::Params: Serialize,
    {
        let actual = self.send_request::<R>(params);
        if let Some((expected_part, actual_part)) = find_mismatch(&expected_resp, &actual) {
            panic!(
                "JSON mismatch\nExpected:\n{}\nWas:\n{}\nExpected part:\n{}\nActual part:\n{}\n",
                to_string_pretty(&expected_resp).unwrap(),
                to_string_pretty(&actual).unwrap(),
                to_string_pretty(expected_part).unwrap(),
                to_string_pretty(actual_part).unwrap(),
            );
        }
    }

    #[track_caller]
    pub(crate) fn send_request<R>(&self, params: R::Params) -> Value
    where
        R: lsp_types::request::Request,
        R::Params: Serialize,
    {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let r = ServerRequest::new(id.into(), R::METHOD.to_owned(), params);
        self.send_request_(r)
    }

    #[track_caller]
    fn send_request_(&self, r: ServerRequest) -> Value {
        let id = r.id.clone();
        self.client.sender.send(r.clone().into()).unwrap();
        while let Some(msg) = self
            .recv()
            .unwrap_or_else(|Timeout| panic!("timeout: {r:?}"))
        {
            match msg {
                Message::Request(req) => {
                    if req.method == "client/registerCapability" {
                        let params = req.params.to_string();
                        if ["workspace/didChangeWatchedFiles", "textDocument/didSave"]
                            .into_iter()
                            .any(|it| params.contains(it))
                        {
                            continue;
                        }
                    }
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
        let msg = recv_timeout(&self.client.receiver)?;
        let msg = msg.inspect(|msg| {
            self.messages.borrow_mut().push(msg.clone());
        });
        Ok(msg)
    }
}

struct Timeout;

fn recv_timeout(receiver: &Receiver<Message>) -> Result<Option<Message>, Timeout> {
    let timeout = if cfg!(target_os = "macos") {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(120)
    };
    select! {
        recv(receiver) -> msg => Ok(msg.ok()),
        recv(after(timeout)) -> _ => Err(Timeout),
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.request::<Shutdown>((), Value::Null);
        self.notification::<Exit>(());
    }
}

/// Compares JSON object for approximate equality.
/// You can use `[..]` wildcard in strings (useful for OS dependent things such
/// as paths). You can use a `"{...}"` string literal as a wildcard for
/// arbitrary nested JSON. Arrays are sorted before comparison.
fn find_mismatch<'a>(expected: &'a Value, actual: &'a Value) -> Option<(&'a Value, &'a Value)> {
    match (expected, actual) {
        (Value::Number(l), Value::Number(r)) if l == r => None,
        (Value::Bool(l), Value::Bool(r)) if l == r => None,
        (Value::String(l), Value::String(r)) if lines_match(l, r) => None,
        (Value::Array(l), Value::Array(r)) => {
            if l.len() != r.len() {
                return Some((expected, actual));
            }

            let mut l = l.iter().collect::<Vec<_>>();
            let mut r = r.iter().collect::<Vec<_>>();

            l.retain(
                |l| match r.iter().position(|r| find_mismatch(l, r).is_none()) {
                    Some(i) => {
                        r.remove(i);
                        false
                    }
                    None => true,
                },
            );

            if !l.is_empty() {
                assert!(!r.is_empty());
                Some((l[0], r[0]))
            } else {
                assert_eq!(r.len(), 0);
                None
            }
        }
        (Value::Object(l), Value::Object(r)) => {
            fn sorted_values(obj: &serde_json::Map<String, Value>) -> Vec<&Value> {
                let mut entries = obj.iter().collect::<Vec<_>>();
                entries.sort_by_key(|it| it.0);
                entries.into_iter().map(|(_k, v)| v).collect::<Vec<_>>()
            }

            let same_keys = l.len() == r.len() && l.keys().all(|k| r.contains_key(k));
            if !same_keys {
                return Some((expected, actual));
            }

            let l = sorted_values(l);
            let r = sorted_values(r);

            l.into_iter().zip(r).find_map(|(l, r)| find_mismatch(l, r))
        }
        (Value::Null, Value::Null) => None,
        // magic string literal "{...}" acts as wildcard for any sub-JSON
        (Value::String(l), _) if l == "{...}" => None,
        _ => Some((expected, actual)),
    }
}

/// Compare a line with an expected pattern.
/// - Use `[..]` as a wildcard to match 0 or more characters on the same line
///   (similar to `.*` in a regex).
fn lines_match(expected: &str, actual: &str) -> bool {
    // Let's not deal with / vs \ (windows...)
    // First replace backslash-escaped backslashes with forward slashes
    // which can occur in, for example, JSON output
    let expected = expected.replace(r"\\", "/").replace('\\', "/");
    let mut actual: &str = &actual.replace(r"\\", "/").replace('\\', "/");
    for (i, part) in expected.split("[..]").enumerate() {
        match actual.find(part) {
            Some(j) => {
                if i == 0 && j != 0 {
                    return false;
                }
                actual = &actual[j + part.len()..];
            }
            None => return false,
        }
    }
    actual.is_empty() || expected.ends_with("[..]")
}

#[test]
fn test_connect() -> Result<(), Error> {
    let (client, server) = Connection::memory();
    let server_thread = thread::spawn(move || start(server));
    let request = lsp_server::Request::new(
        1.into(),
        Initialize::METHOD.to_owned(),
        to_value(InitializeParams {
            ..Default::default()
        })
        .unwrap(),
    );
    client.sender.send(Message::Request(request))?;
    client.receiver.recv()?;
    let notification = lsp_server::Notification {
        method: Initialized::METHOD.to_owned(),
        params: to_value(InitializedParams {}).unwrap(),
    };
    client.sender.send(Message::Notification(notification))?;
    let exit = lsp_server::Notification {
        method: Exit::METHOD.to_owned(),
        params: to_value(()).unwrap(),
    };
    client.sender.send(Message::Notification(exit))?;
    drop(client);
    server_thread.join().unwrap().unwrap();
    Ok(())
}

//#[test]
fn test_hard_disconnect() {
    let (client, server) = Connection::memory();
    let server_thread = thread::spawn(move || start(server));
    drop(client);
    let err = server_thread.join().unwrap().unwrap_err();
    assert_eq!(err.to_string(), "disconnected channel");
}
