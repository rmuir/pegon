use anyhow::{Context as _, Result};
use line_index::LineIndex;
use lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DocumentDiagnosticParams, DocumentDiagnosticReportResult, FullDocumentDiagnosticReport,
    Location, NumberOrString, PublishDiagnosticsParams, UnchangedDocumentDiagnosticReport, Uri,
};
use lsp_types::{
    DocumentDiagnosticReport, RelatedFullDocumentDiagnosticReport,
    RelatedUnchangedDocumentDiagnosticReport,
};

use core::hash::Hash as _;
use core::hash::Hasher as _;
use core::str::FromStr as _;
use std::hash::DefaultHasher;

use crate::{
    lint::{Lint, Severity, lint, rule},
    lsp::{Client, server::Document},
};

impl From<Severity> for DiagnosticSeverity {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Error => Self::ERROR,
            Severity::Warn => Self::WARNING,
            Severity::Info => Self::INFORMATION,
            Severity::Hint => Self::HINT,
        }
    }
}

/// diagnostics request (pull)
pub fn pull(
    client: &Client,
    doc: &Document,
    params: &DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReportResult> {
    let bytes = doc.text.as_bytes();
    let results = lint(&doc.tree, bytes)?;
    let result_id = hash_items(&results);

    if let Some(previous_id) = &params.previous_result_id
        && *previous_id == result_id
    {
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Unchanged(RelatedUnchangedDocumentDiagnosticReport {
                unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport {
                    result_id,
                },
                related_documents: None,
            }),
        ))
    } else {
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    items: encode(client, &params.text_document.uri, &doc.line_index, &results)?,
                    result_id: Some(result_id),
                },
                related_documents: None,
            }),
        ))
    }
}

/// publish diagnostics (push)
pub fn push(client: &Client, doc: &Document, uri: &Uri) -> Result<PublishDiagnosticsParams> {
    let bytes = doc.text.as_bytes();
    let results = lint(&doc.tree, bytes)?;
    Ok(PublishDiagnosticsParams {
        diagnostics: encode(client, uri, &doc.line_index, &results)?,
        uri: uri.clone(),
        version: client.supports_version().then_some(doc.version),
    })
}

fn hash_items(items: &Vec<Lint>) -> String {
    let mut hasher = DefaultHasher::new();
    items.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// encode diagnostics into LSP structure
fn encode(
    client: &Client,
    uri: &Uri,
    line_index: &LineIndex,
    results: &[Lint],
) -> Result<Vec<Diagnostic>> {
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
            Ok(Diagnostic {
                range,
                severity: Some(lsp_severity),
                code: Some(NumberOrString::String(rule.name.clone())),
                code_description: client.supports_code_description().then(|| CodeDescription {
                    href: Uri::from_str(&rule.url).expect("rule url should exist"),
                }),
                source: Some("pegon".to_owned()),
                message: diagnostic.title.clone(),
                related_information: client
                    .supports_related_information()
                    .then_some(related_information),
                tags: None,
                data: None,
            })
        })
        .collect()
}
