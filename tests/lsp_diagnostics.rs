#![expect(clippy::panic, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use gen_lsp_types::{
    ClientCapabilities, ClientDiagnosticsTagOptions, Code, CodeDescription, Diagnostic,
    DiagnosticClientCapabilities, DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag,
    DiagnosticsCapabilities, DidChangeTextDocumentNotification, DidChangeTextDocumentParams,
    DidCloseTextDocumentNotification, DidCloseTextDocumentParams, DidOpenTextDocumentNotification,
    DidOpenTextDocumentParams, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticRequest, InitializeParams, Location, PartialResultParams, Position,
    PublishDiagnosticsClientCapabilities, PublishDiagnosticsNotification, Range,
    TextDocumentClientCapabilities, TextDocumentContentChangeEvent,
    TextDocumentContentChangePartial, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentSyncClientCapabilities, VersionedTextDocumentIdentifier, WorkDoneProgressParams,
};
use indoc::indoc;
use lsp_client::LspClient;

pub mod lsp_client;

/// diagnose a simple document (push diagnostics, zero fancy features)
#[test]
fn diagnostics() {
    let client = LspClient::new(InitializeParams::default());
    client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: "file:///Foo.java".into(),
            language_id: "java".into(),
            version: 0,
            text: indoc! {r#"
                public class foo {
                }
            "#}
            .into(),
        },
    });
    let diagnostics = client.read_notify::<PublishDiagnosticsNotification>();
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
            severity: Some(DiagnosticSeverity::Warning),
            code: Some(Code::String("lowercase-class".into())),
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
    client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: "file:///Foo.java".into(),
            language_id: "java".into(),
            version: 0,
            text: indoc! {r#"
                public class foo {
                }
            "#}
            .into(),
        },
    });
    let diagnostics = client.read_notify::<PublishDiagnosticsNotification>();
    // one problem
    assert_eq!(1, diagnostics.diagnostics.len());
    // close the file
    client.notify::<DidCloseTextDocumentNotification>(DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: "file:///Foo.java".into(),
        },
    });
    let cleared = client.read_notify::<PublishDiagnosticsNotification>();
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
                diagnostics_capabilities: DiagnosticsCapabilities {
                    related_information: Some(true),
                    code_description_support: Some(true),
                    data_support: Some(true),
                    tag_support: Some(ClientDiagnosticsTagOptions {
                        value_set: vec![DiagnosticTag::Unnecessary, DiagnosticTag::Deprecated],
                    }),
                },
            }),
            publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                version_support: Some(true),
                diagnostics_capabilities: DiagnosticsCapabilities {
                    related_information: Some(true),
                    code_description_support: Some(true),
                    data_support: Some(true),
                    tag_support: Some(ClientDiagnosticsTagOptions {
                        value_set: vec![DiagnosticTag::Unnecessary, DiagnosticTag::Deprecated],
                    }),
                },
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
    client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: "file:///Foo.java".into(),
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
            uri: "file:///Foo.java".into(),
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

    let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(full) = result else {
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
            severity: Some(DiagnosticSeverity::Warning),
            code: Some(Code::String("lowercase-class".into())),
            source: Some("pegon".into()),
            message: "Lowercase class: `foo`".into(),
            code_description: Some(CodeDescription {
                href: "https://github.com/rmuir/pegon/wiki/lints#lowercase-class".into()
            }),
            related_information: Some(vec![DiagnosticRelatedInformation {
                location: Location {
                    uri: "file:///Foo.java".into(),
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
    client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: "file:///Foo.java".into(),
            language_id: "java".into(),
            version: 0,
            text: indoc! {r#"
                public class Foo {
                }
            "#}
            .into(),
        },
    });
    let diagnostics = client.read_notify::<PublishDiagnosticsNotification>();
    // no problems
    assert!(diagnostics.diagnostics.is_empty());
    client.notify::<DidChangeTextDocumentNotification>(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            text_document_identifier: TextDocumentIdentifier {
                uri: "file:///Foo.java".into(),
            },
            version: 1,
        },
        content_changes: vec![
            TextDocumentContentChangeEvent::TextDocumentContentChangePartial(
                TextDocumentContentChangePartial {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 13,
                        },
                        end: Position {
                            line: 0,
                            character: 14,
                        },
                    },
                    #[expect(deprecated, reason = "unavoidable")]
                    range_length: None,
                    text: "f".into(),
                },
            ),
        ],
    });
    let changed = client.read_notify::<PublishDiagnosticsNotification>();
    assert_eq!(1, changed.diagnostics.len());
    let code = Some(Code::String("lowercase-class".into()));
    assert_eq!(code, changed.diagnostics[0].code);
}
