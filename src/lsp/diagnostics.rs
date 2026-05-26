use anyhow::{Context as _, Result};
use gen_lsp_types::{
    Code, CodeDescription, DiagnosticRelatedInformation, DiagnosticSeverity,
    DocumentDiagnosticParams, DocumentDiagnosticReport, FullDocumentDiagnosticReport, Location,
    Message, PublishDiagnosticsParams, RelatedFullDocumentDiagnosticReport, Uri,
};
use line_index::LineIndex;

use crate::{
    diagnostics::{Diagnostic, Severity, lint, rule},
    lsp::{Client, server::Document},
};

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
) -> Result<DocumentDiagnosticReport> {
    let bytes = doc.text.as_bytes();
    let results = lint(&doc.tree, bytes)?;

    Ok(
        DocumentDiagnosticReport::RelatedFullDocumentDiagnosticReport(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    items: encode(
                        client,
                        &params.text_document.uri,
                        &doc.line_index,
                        false,
                        &results,
                    )?,
                    result_id: None, // don't attempt to cache, bugs such as neovim/neovim#32247
                },
            },
        ),
    )
}

/// publish diagnostics (push)
pub fn push(client: &Client, doc: &Document, uri: &Uri) -> Result<PublishDiagnosticsParams> {
    let bytes = doc.text.as_bytes();
    let results = lint(&doc.tree, bytes)?;
    Ok(PublishDiagnosticsParams {
        diagnostics: encode(client, uri, &doc.line_index, true, &results)?,
        uri: uri.clone(),
        version: client.supports_version().then_some(doc.version),
    })
}

/// encode diagnostics into LSP structure
fn encode(
    client: &Client,
    uri: &Uri,
    line_index: &LineIndex,
    push: bool,
    results: &[Diagnostic],
) -> Result<Vec<gen_lsp_types::Diagnostic>> {
    results
        .iter()
        .map(|diagnostic| {
            let rule = rule(diagnostic.rule_id);
            let range = client
                .encode_range(&diagnostic.range, line_index)
                .context("invalid range")?;
            let lsp_severity = rule.severity.into();
            let mut related_information: Vec<DiagnosticRelatedInformation> = Vec::with_capacity(3);
            // all the context ranges are related information
            if let Some(related) = &diagnostic.context {
                related_information.push(DiagnosticRelatedInformation {
                    location: Location {
                        uri: uri.clone(),
                        range: client
                            .encode_range(related, line_index)
                            .context("invalid range")?,
                    },
                    message: rule.context_label.clone().unwrap_or_default(),
                });
            }
            // optional label maps to related information at node's position
            if let Some(label) = &diagnostic.label {
                related_information.push(DiagnosticRelatedInformation {
                    location: Location {
                        uri: uri.clone(),
                        range,
                    },
                    message: label.clone(),
                });
            }
            // help text maps to related information at node's position
            related_information.push(DiagnosticRelatedInformation {
                location: Location {
                    uri: uri.clone(),
                    range,
                },
                message: diagnostic.help.clone(),
            });
            Ok(gen_lsp_types::Diagnostic {
                range,
                severity: Some(lsp_severity),
                code: Some(Code::String(rule.name.clone())),
                code_description: client.supports_code_description(push).then_some(
                    CodeDescription {
                        href: Uri(rule.url.clone()),
                    },
                ),
                source: Some("pegon".to_owned()),
                message: Message::String(diagnostic.title.clone()),
                related_information: client
                    .supports_related_information(push)
                    .then_some(related_information),
                tags: None,
                data: None,
            })
        })
        .collect()
}
