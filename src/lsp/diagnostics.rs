use core::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use gen_lsp_types::{
    Code, CodeDescription, DiagnosticRelatedInformation, DiagnosticSeverity,
    DocumentDiagnosticParams, DocumentDiagnosticReport, FullDocumentDiagnosticReport, Location,
    MarkupContent, MarkupKind, Message, PublishDiagnosticsParams,
    RelatedFullDocumentDiagnosticReport, Uri,
};
use serde::{Deserialize, Serialize};

use crate::support::diagnostics::{Diagnostic, Severity, lint, rule};

use super::{Client, server::Document};

/// Allow the fastest light-bulb possible
#[derive(Serialize, Deserialize)]
pub struct CustomData {
    /// Title for a code-action fix, if available
    pub fix: String,
}

impl From<Severity> for DiagnosticSeverity {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Error => Self::Error,
            Severity::Warn => Self::Warning,
            Severity::Info => Self::Information,
            Severity::Hint => Self::Hint,
        }
    }
}

/// diagnostics request (pull)
pub fn pull(
    client: &Client,
    doc: &Document,
    params: &DocumentDiagnosticParams,
    cancel_token: &Arc<AtomicBool>,
) -> Result<DocumentDiagnosticReport> {
    let bytes = doc.text.as_bytes();
    let results = lint(&doc.tree, bytes, cancel_token, false)?;

    Ok(
        DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    items: encode(client, &params.text_document.uri, doc, false, &results)?,
                    result_id: None, // don't attempt to cache, bugs such as neovim/neovim#32247
                },
            },
        ),
    )
}

/// publish diagnostics (push)
pub fn push(client: &Client, doc: &Document, uri: &Uri) -> Result<PublishDiagnosticsParams> {
    let bytes = doc.text.as_bytes();
    let results = lint(&doc.tree, bytes, &Arc::new(AtomicBool::new(false)), false)?;
    Ok(PublishDiagnosticsParams {
        diagnostics: encode(client, uri, doc, true, &results)?,
        uri: uri.clone(),
        version: client.supports_version().then_some(doc.version),
    })
}

/// encode diagnostics into LSP structure
fn encode(
    client: &Client,
    uri: &Uri,
    doc: &Document,
    push: bool,
    results: &[Diagnostic],
) -> Result<Vec<gen_lsp_types::Diagnostic>> {
    results
        .iter()
        .map(|diagnostic| {
            let rule = rule(diagnostic.rule_id);
            let range = client
                .encode_range(&diagnostic.range, &doc.line_index)
                .context("invalid range")?;
            let lsp_severity = rule.severity.into();
            let mut related_information: Vec<DiagnosticRelatedInformation> = Vec::with_capacity(3);
            // all the context ranges are related information
            if let Some(related) = &diagnostic.context {
                related_information.push(DiagnosticRelatedInformation {
                    location: Location {
                        uri: uri.clone(),
                        range: client
                            .encode_range(related, &doc.line_index)
                            .context("invalid range")?,
                    },
                    message: rule.context_label.clone().unwrap_or_default(),
                });
            }
            // optional label maps to related information at node's position
            if let Some(label) = &diagnostic.label {
                related_information.push(DiagnosticRelatedInformation {
                    location: Location::new(uri.clone(), range),
                    message: label.clone(),
                });
            }
            // help text maps to related information at node's position
            related_information.push(DiagnosticRelatedInformation {
                location: Location::new(uri.clone(), range),
                message: diagnostic.help.clone(),
            });
            let message = if client.supports_markup_messages(push) {
                Message::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: diagnostic.title.clone(),
                })
            } else {
                Message::String(diagnostic.title.clone())
            };
            let data = (client.supports_data(push) && rule.fix.is_some())
                .then(|| {
                    serde_json::to_value(CustomData {
                        fix: diagnostic.help.clone(),
                    })
                })
                .transpose()?;
            Ok(gen_lsp_types::Diagnostic {
                range,
                severity: Some(lsp_severity),
                code: Some(Code::String(rule.name.clone())),
                code_description: client
                    .supports_code_description(push)
                    .then_some(CodeDescription::new(rule.url.clone().into())),
                source: Some("pegon".into()),
                message,
                related_information: client
                    .supports_related_information(push)
                    .then_some(related_information),
                tags: None,
                data,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        ClientCapabilities, ClientDiagnosticsTagOptions, Code, CodeDescription,
        DiagnosticClientCapabilities, DiagnosticRelatedInformation, DiagnosticSeverity,
        DiagnosticTag, DiagnosticsCapabilities, DidChangeTextDocumentNotification,
        DidChangeTextDocumentParams, DidCloseTextDocumentNotification, DidCloseTextDocumentParams,
        DidOpenTextDocumentNotification, DidOpenTextDocumentParams, DocumentDiagnosticParams,
        DocumentDiagnosticReport, DocumentDiagnosticRequest, InitializeParams, Location,
        MarkupContent, MarkupKind, Message, PartialResultParams, Position,
        PublishDiagnosticsClientCapabilities, PublishDiagnosticsNotification, Range,
        TextDocumentClientCapabilities, TextDocumentContentChangeEvent,
        TextDocumentContentChangePartial, TextDocumentIdentifier, TextDocumentItem,
        TextDocumentSyncClientCapabilities, VersionedTextDocumentIdentifier,
        WorkDoneProgressParams,
    };
    use indoc::indoc;

    use crate::lsp::test_client::TestClient;

    /// diagnose a simple document (push diagnostics, zero fancy features)
    #[test]
    fn diagnostics() {
        let client = TestClient::new(InitializeParams::default());
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                public class foo {
                }
            "}
                .into(),
            },
        });
        let diagnostics = client.read_notify::<PublishDiagnosticsNotification>();
        // we didn't sign up for this
        assert_eq!(None, diagnostics.version);
        // one problem
        assert_eq!(
            vec![gen_lsp_types::Diagnostic {
                range: Range::new(Position::new(0, 13), Position::new(0, 16)),
                severity: Some(DiagnosticSeverity::Warning),
                code: Some(Code::String("lowercase-class".into())),
                source: Some(env!("CARGO_PKG_NAME").into()),
                message: "Lowercase class: `foo`".into(),
                ..Default::default()
            }],
            diagnostics.diagnostics
        );
    }

    /// push diagnostics should be cleared by the server on close
    #[test]
    fn push_clear_on_close() {
        let client = TestClient::new(InitializeParams::default());
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                public class foo {
                }
            "}
                .into(),
            },
        });
        let diagnostics = client.read_notify::<PublishDiagnosticsNotification>();
        // one problem
        assert_eq!(1, diagnostics.diagnostics.len());
        // close the file
        client.notify::<DidCloseTextDocumentNotification>(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
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
                    markup_message_support: Some(true),
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
        let client = TestClient::new(InitializeParams {
            capabilities: full_capabilities(),
            ..Default::default()
        });
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                public class foo {
                }
            "}
                .into(),
            },
        });
        let result = client.request::<DocumentDiagnosticRequest>(DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
            previous_result_id: None,
            identifier: None,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });

        let DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(full) = result else {
            panic!();
        };

        let diagnostics = full.full_document_diagnostic_report.items;

        // one problem
        assert_eq!(
            vec![gen_lsp_types::Diagnostic {
                range: Range::new(Position::new(0, 13), Position::new(0, 16)),
                severity: Some(DiagnosticSeverity::Warning),
                code: Some(Code::String("lowercase-class".into())),
                source: Some(env!("CARGO_PKG_NAME").into()),
                message: Message::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Lowercase class: `foo`".into(),
                }),
                code_description: Some(CodeDescription {
                    href: "https://github.com/rmuir/pegon/wiki/diagnostics#lowercase-class".into()
                }),
                related_information: Some(vec![DiagnosticRelatedInformation {
                    location: Location {
                        uri: "file:///Foo.java".into(),
                        range: Range::new(Position::new(0, 13), Position::new(0, 16)),
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
        let client = TestClient::new(InitializeParams::default());
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                public class Foo {
                }
            "}
                .into(),
            },
        });
        let diagnostics = client.read_notify::<PublishDiagnosticsNotification>();
        // no problems
        assert!(diagnostics.diagnostics.is_empty());
        client.notify::<DidChangeTextDocumentNotification>(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                text_document_identifier: TextDocumentIdentifier::new("file:///Foo.java".into()),
                version: 1,
            },
            content_changes: vec![
                TextDocumentContentChangeEvent::TextDocumentContentChangePartial(
                    TextDocumentContentChangePartial {
                        range: Range::new(Position::new(0, 13), Position::new(0, 14)),
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
}
