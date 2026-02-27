use std::{
    str::FromStr,
    thread::{self},
};

use indoc::indoc;
use lsp_server::{Connection, Message};
use lsp_types::{
    ClientCapabilities, DidOpenTextDocumentParams, GeneralClientCapabilities, InitializeParams,
    PositionEncodingKind, TextDocumentItem, Uri, notification::DidOpenTextDocument,
};
use pegon::lsp::start;

use crate::lsp_client::Client;

mod lsp_client;

/// default to UTF-16 according to the spec
#[test]
fn test_encoding_default() {
    let client = Client::new(InitializeParams::default());
    assert_eq!(
        Some(PositionEncodingKind::UTF16),
        client.init_response().capabilities.position_encoding
    );
}

/// pick the client's first offered encoding.
/// most likely it is the most performant for that client
#[test]
fn test_encoding_preferred() {
    let client = Client::new(InitializeParams {
        capabilities: ClientCapabilities {
            general: Some(GeneralClientCapabilities {
                position_encodings: Some(vec![
                    PositionEncodingKind::UTF8,
                    PositionEncodingKind::UTF16,
                ]),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    });
    assert_eq!(
        Some(PositionEncodingKind::UTF8),
        client.init_response().capabilities.position_encoding
    );
}

/// check all encoding kinds can be negotiated
#[test]
fn test_encodings() {
    for encoding in [
        PositionEncodingKind::UTF8,
        PositionEncodingKind::UTF16,
        PositionEncodingKind::UTF32,
    ] {
        let client = Client::new(InitializeParams {
            capabilities: ClientCapabilities {
                general: Some(GeneralClientCapabilities {
                    position_encodings: Some(vec![encoding.clone()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(
            Some(encoding),
            client.init_response().capabilities.position_encoding
        );
    }
}

/// diagnose a simple document
#[test]
fn test_diagnose() {
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

/// make sure if the stream disconnects that the error makes it out
/// this ensure no leftover processes, which will annoy users!
#[test]
fn test_hard_disconnect() {
    let (client, server) = Connection::memory();
    let server_thread = thread::spawn(move || start(server));
    drop(client);
    let err = server_thread.join().unwrap().unwrap_err();
    assert_eq!(err.to_string(), "disconnected channel");
}
