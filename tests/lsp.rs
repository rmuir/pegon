use std::{
    str::FromStr,
    thread::{self},
};

use indoc::indoc;
use lsp_server::{Connection, Message};
use lsp_types::{
    DidOpenTextDocumentParams, InitializeParams, TextDocumentItem, Uri,
    notification::DidOpenTextDocument,
};
use pegon::lsp::start;

use crate::lsp_client::Client;

mod lsp_client;

#[test]
fn test_connect() {
    let client = Client::new(InitializeParams::default());
    client.notify::<DidOpenTextDocument>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Uri::from_str("file:///Foo.java").unwrap(),
            language_id: "java".into(),
            version: 0,
            text: indoc! {r#"
                public class foo {
                }
            "#}
            .into(),
        },
    });
    let notification = client.recv().unwrap();
    if let Some(Message::Notification(diagnostics)) = notification {
        assert_eq!("textDocument/publishDiagnostics", diagnostics.method);
    } else {
        panic!("wrong: {notification:?}");
    }
}

#[test]
fn test_hard_disconnect() {
    let (client, server) = Connection::memory();
    let server_thread = thread::spawn(move || start(server));
    drop(client);
    let err = server_thread.join().unwrap().unwrap_err();
    assert_eq!(err.to_string(), "disconnected channel");
}
