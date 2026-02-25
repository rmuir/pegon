use anyhow::{Context, Error, Result};
use line_index::LineIndex;
use lsp_server::Message;
use lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DocumentDiagnosticParams, DocumentDiagnosticReportKind, FullDocumentDiagnosticReport, Location,
    NumberOrString, PublishDiagnosticsParams, Range, UnchangedDocumentDiagnosticReport, Uri,
    notification::{Notification, PublishDiagnostics},
};

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
};

use crate::{
    lint::{Lint, Severity, lint, rule},
    lsp::{Client, open_document::OpenDocument},
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
pub fn pull_diagnostics(
    client: &Client,
    doc: &OpenDocument,
    params: &DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReportKind> {
    let line_index = LineIndex::new(&doc.text);
    let bytes = doc.text.as_bytes();
    let results = lint(&doc.tree, bytes)?;
    let result_id = hash_items(&results);

    if let Some(previous_id) = &params.previous_result_id
        && *previous_id == result_id
    {
        Ok(DocumentDiagnosticReportKind::Unchanged(
            UnchangedDocumentDiagnosticReport { result_id },
        ))
    } else {
        Ok(DocumentDiagnosticReportKind::Full(
            FullDocumentDiagnosticReport {
                items: encode(client, &params.text_document.uri, &line_index, &results)?,
                result_id: Some(result_id),
            },
        ))
    }
}

/// publish diagnostics (push)
pub fn push_diagnostics(client: &Client, doc: &OpenDocument, uri: &Uri) -> Result<()> {
    let line_index = LineIndex::new(&doc.text);
    let bytes = doc.text.as_bytes();
    let results = lint(&doc.tree, bytes)?;

    client
        .connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            PublishDiagnostics::METHOD.to_owned(),
            PublishDiagnosticsParams {
                diagnostics: encode(client, uri, &line_index, &results)?,
                uri: uri.clone(),
                version: if client.version_support() {
                    Some(doc.version)
                } else {
                    None
                },
            },
        )))?;
    Ok(())
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
            let start = client
                .encode_position(diagnostic.range.start, line_index)
                .context("invalid start offset")?;
            let end = client
                .encode_position(diagnostic.range.end, line_index)
                .context("invalid end offset")?;
            let lsp_severity = rule.severity.into();
            // all the context ranges are related information
            let mut related_information = diagnostic
                .context
                .iter()
                .map(|context| {
                    let related_start = client
                        .encode_position(context.start, line_index)
                        .context("invalid context start offset")?;
                    let related_end = client
                        .encode_position(context.end, line_index)
                        .context("invalid context end offset")?;
                    Ok(DiagnosticRelatedInformation {
                        location: Location {
                            uri: uri.clone(),
                            range: Range::new(related_start, related_end),
                        },
                        message: rule.context_label.clone().unwrap_or_default(),
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            // optional label maps to related information at node's position
            if let Some(label) = &diagnostic.label {
                related_information.push(DiagnosticRelatedInformation {
                    location: Location {
                        uri: uri.clone(),
                        range: Range::new(start, end),
                    },
                    message: label.clone(),
                });
            }
            // help text maps to related information at node's position
            related_information.push(DiagnosticRelatedInformation {
                location: Location {
                    uri: uri.clone(),
                    range: Range::new(start, end),
                },
                message: diagnostic.help.clone(),
            });
            Ok(Diagnostic {
                range: Range::new(start, end),
                severity: Some(lsp_severity),
                code: Some(NumberOrString::String(rule.name.clone())),
                code_description: if client.code_description_support() {
                    Some(CodeDescription {
                        href: Uri::from_str(&rule.url).expect("rule url should exist"),
                    })
                } else {
                    None
                },
                source: Some("pegon".to_string()),
                message: diagnostic.title.clone(),
                related_information: if client.related_information_support() {
                    Some(related_information)
                } else {
                    None
                },
                tags: None,
                data: None,
            })
        })
        .collect()
}

// for push clients, clear diagnostic space, e.g. on document close
pub fn push_clear(client: &Client, uri: &Uri) -> Result<()> {
    client
        .connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            PublishDiagnostics::METHOD.to_owned(),
            PublishDiagnosticsParams {
                diagnostics: vec![],
                uri: uri.clone(),
                version: None,
            },
        )))?;
    Ok(())
}
