#![expect(clippy::panic, reason = "tests")]
#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use core::str::FromStr as _;

use indoc::indoc;
use ls_types::{
    ClientCapabilities, CodeDescription, Diagnostic, DiagnosticClientCapabilities,
    DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentDiagnosticParams,
    DocumentDiagnosticReport, DocumentDiagnosticReportResult, InitializeParams, Location,
    NumberOrString, PartialResultParams, Position, PublishDiagnosticsClientCapabilities, Range,
    TagSupport, TextDocumentClientCapabilities, TextDocumentContentChangeEvent,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentSyncClientCapabilities, Uri,
    VersionedTextDocumentIdentifier, WorkDoneProgressParams,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, PublishDiagnostics,
    },
    request::DocumentDiagnosticRequest,
};
use lsp_client::LspClient;

pub mod lsp_client;

/// diagnose a simple document (push diagnostics, zero fancy features)
#[test]
fn diagnostics() {
    let client = LspClient::new(InitializeParams::default());
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

/// push diagnostics should be cleared by the server on close
#[test]
fn push_clear_on_close() {
    let client = LspClient::new(InitializeParams::default());
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
    // one problem
    assert_eq!(1, diagnostics.diagnostics.len());
    // close the file
    client.notify::<DidCloseTextDocument>(DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: Uri::from_str("file:///Foo.java").unwrap(),
        },
    });
    let cleared = client.read_notify::<PublishDiagnostics>();
    assert_eq!(0, cleared.diagnostics.len());
}

/// full-featured client for ease of testing
fn full_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        text_document: Some(TextDocumentClientCapabilities {
            synchronization: Some(TextDocumentSyncClientCapabilities {
                dynamic_registration: Some(true),
                will_save: Some(true),
                will_save_wait_until: Some(true),
                did_save: Some(true),
            }),
            diagnostic: Some(DiagnosticClientCapabilities {
                related_document_support: Some(true),
                dynamic_registration: Some(true),
                related_information: Some(true),
                code_description_support: Some(true),
                data_support: Some(true),
                tag_support: None, // bug in ls_types
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
fn pull_diagnostics() {
    let client = LspClient::new(InitializeParams {
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

    let diagnostics = full.full_document_diagnostic_report.items;

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

/// modify a document to become problematic
#[test]
fn diagnostics_on_change() {
    let client = LspClient::new(InitializeParams::default());
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
    // no problems
    assert!(diagnostics.diagnostics.is_empty());
    client.notify::<DidChangeTextDocument>(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: Uri::from_str("file:///Foo.java").unwrap(),
            version: 1,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 13,
                },
                end: Position {
                    line: 0,
                    character: 14,
                },
            }),
            range_length: None,
            text: "f".into(),
        }],
    });
    let changed = client.read_notify::<PublishDiagnostics>();
    assert_eq!(1, changed.diagnostics.len());
    let code = Some(NumberOrString::String("lowercase-class".into()));
    assert_eq!(code, changed.diagnostics[0].code);
}
