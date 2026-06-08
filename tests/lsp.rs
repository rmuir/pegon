#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use std::thread;

use gen_lsp_types::{
    ClientCapabilities, DiagnosticClientCapabilities, DidChangeTextDocumentNotification,
    DidCloseTextDocumentNotification, DidOpenTextDocumentNotification, DocumentDiagnosticRequest,
    GeneralClientCapabilities, InitializeParams, Notification as _, PositionEncodingKind,
    Request as _, TextDocumentClientCapabilities, TextDocumentSyncClientCapabilities,
};
use lsp_server::Connection;
use pegon::lsp::run_server;

use lsp_client::LspClient;
pub mod lsp_client;

/// default to UTF-16 according to the spec
#[test]
fn encoding_default() {
    let client = LspClient::new(InitializeParams::default());
    assert_eq!(
        Some(PositionEncodingKind::UTF16),
        client.init_response().capabilities.position_encoding
    );
}

/// pick the client's first offered encoding.
/// most likely it is the most performant for that client
#[test]
fn encoding_preferred() {
    let client = LspClient::new(InitializeParams {
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
fn negotiate_encodings() {
    for encoding in [
        PositionEncodingKind::UTF8,
        PositionEncodingKind::UTF16,
        PositionEncodingKind::UTF32,
    ] {
        let client = LspClient::new(InitializeParams {
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

#[test]
fn dynamic_registration() {
    let client = LspClient::new(InitializeParams {
        capabilities: ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities {
                synchronization: Some(TextDocumentSyncClientCapabilities {
                    dynamic_registration: Some(true),
                    ..Default::default()
                }),
                diagnostic: Some(DiagnosticClientCapabilities {
                    dynamic_registration: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    });
    let result = client.registrations();
    assert!(result.is_some());
    let params = result.unwrap().registrations;
    assert_eq!(params.len(), 4);
    assert_eq!(
        params[0].method,
        DidOpenTextDocumentNotification::METHOD.to_string()
    );
    assert_eq!(
        params[1].method,
        DidChangeTextDocumentNotification::METHOD.to_string()
    );
    assert_eq!(
        params[2].method,
        DidCloseTextDocumentNotification::METHOD.to_string()
    );
    assert_eq!(
        params[3].method,
        DocumentDiagnosticRequest::METHOD.to_string()
    );
}

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
