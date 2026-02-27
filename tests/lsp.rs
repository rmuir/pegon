use std::{
    str::FromStr,
    thread::{self},
};

use indoc::indoc;
use lsp_server::Connection;
use lsp_types::{
    ClientCapabilities, CodeDescription, Diagnostic, DiagnosticClientCapabilities,
    DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag, DidOpenTextDocumentParams,
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    GeneralClientCapabilities, InitializeParams, Location, NumberOrString, PartialResultParams,
    Position, PositionEncodingKind, PublishDiagnosticsClientCapabilities, Range, TagSupport,
    TextDocumentClientCapabilities, TextDocumentIdentifier, TextDocumentItem, Uri,
    WorkDoneProgressParams,
    notification::{DidOpenTextDocument, PublishDiagnostics},
    request::DocumentDiagnosticRequest,
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
fn test_negotiate_encodings() {
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

/// diagnose a document with no problems
#[test]
fn test_no_diagnostics() {
    let client = Client::new(InitializeParams::default());
    client.notify::<DidOpenTextDocument>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Uri::from_str("file:///Foo.java").unwrap(),
            language_id: "java".into(),
            version: 0,
            text: indoc! {r#"
                public class Foo {
                }
            "#}
            .into(),
        },
    });
    let diagnostics = client.read_notify::<PublishDiagnostics>();
    // we didn't sign up for this
    assert_eq!(None, diagnostics.version);
    // no problems
    assert!(diagnostics.diagnostics.is_empty());
}

/// diagnose a simple document (push diagnostics, zero fancy features)
#[test]
fn test_diagnostics() {
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
    let diagnostics = client.read_notify::<PublishDiagnostics>();
    // we didn't sign up for this
    assert_eq!(None, diagnostics.version);
    // one problem
    assert_eq!(
        vec![Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 13
                },
                end: Position {
                    line: 0,
                    character: 16
                }
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String("lowercase-class".into())),
            source: Some("pegon".into()),
            message: "Lowercase class: `foo`".into(),
            ..Default::default()
        }],
        diagnostics.diagnostics
    );
}

/// full-featured client for ease of testing
fn full_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        text_document: Some(TextDocumentClientCapabilities {
            diagnostic: Some(DiagnosticClientCapabilities {
                related_document_support: Some(true),
                dynamic_registration: Some(true),
            }),
            publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                related_information: Some(true),
                code_description_support: Some(true),
                version_support: Some(true),
                data_support: Some(true),
                tag_support: Some(TagSupport {
                    value_set: vec![DiagnosticTag::UNNECESSARY, DiagnosticTag::DEPRECATED],
                }),
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// test diagnostics pull approach, with all features
#[test]
fn test_pull_diagnostics() {
    let client = Client::new(InitializeParams {
        capabilities: full_capabilities(),
        ..Default::default()
    });
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
    let result = client.request::<DocumentDiagnosticRequest>(DocumentDiagnosticParams {
        text_document: TextDocumentIdentifier {
            uri: Uri::from_str("file:///Foo.java").unwrap(),
        },
        previous_result_id: None,
        identifier: None,
        work_done_progress_params: WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: PartialResultParams {
            partial_result_token: None,
        },
    });

    let DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) = result
    else {
        panic!();
    };

    let report = full.full_document_diagnostic_report;
    let result_id = report.result_id;
    assert_ne!(None, result_id);
    let diagnostics = report.items;

    // one problem
    assert_eq!(
        vec![Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 13
                },
                end: Position {
                    line: 0,
                    character: 16
                }
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String("lowercase-class".into())),
            source: Some("pegon".into()),
            message: "Lowercase class: `foo`".into(),
            code_description: Some(CodeDescription {
                href: Uri::from_str("https://github.com/rmuir/pegon/wiki/lints#lowercase-class")
                    .unwrap(),
            }),
            related_information: Some(vec![DiagnosticRelatedInformation {
                location: Location {
                    uri: Uri::from_str("file:///Foo.java").unwrap(),
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 13
                        },
                        end: Position {
                            line: 0,
                            character: 16
                        }
                    },
                },
                message: "Rename `foo` using UpperCamelCase".into(),
            },]),
            ..Default::default()
        }],
        diagnostics
    );
}

/// when the result is the same as the `previous_result_id`, emit unchanged
/// it can save some serialization and client processing
#[test]
fn test_diagnostics_unchanged() {
    let client = Client::new(InitializeParams {
        capabilities: full_capabilities(),
        ..Default::default()
    });
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
    let result = client.request::<DocumentDiagnosticRequest>(DocumentDiagnosticParams {
        text_document: TextDocumentIdentifier {
            uri: Uri::from_str("file:///Foo.java").unwrap(),
        },
        previous_result_id: None,
        identifier: None,
        work_done_progress_params: WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: PartialResultParams {
            partial_result_token: None,
        },
    });

    let DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) = result
    else {
        panic!();
    };

    let report = full.full_document_diagnostic_report;
    let result_id = report.result_id.clone();
    assert_ne!(None, result_id);
    assert_eq!(1, report.items.len());

    let result = client.request::<DocumentDiagnosticRequest>(DocumentDiagnosticParams {
        text_document: TextDocumentIdentifier {
            uri: Uri::from_str("file:///Foo.java").unwrap(),
        },
        previous_result_id: result_id.clone(),
        identifier: None,
        work_done_progress_params: WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: PartialResultParams {
            partial_result_token: None,
        },
    });

    let DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Unchanged(unchanged)) =
        result
    else {
        panic!();
    };

    assert_eq!(
        result_id.unwrap(),
        unchanged.unchanged_document_diagnostic_report.result_id
    );
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
