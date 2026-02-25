use anyhow::{Context, Result};
use line_index::LineIndex;
use lsp_server::Message;
use lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DocumentDiagnosticReportKind, FullDocumentDiagnosticReport, Location, NumberOrString,
    PublishDiagnosticsParams, Range, UnchangedDocumentDiagnosticReport, Uri,
    notification::{Notification, PublishDiagnostics},
};
use rustc_hash::FxHashMap;

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
    uri: &Uri,
    docs: &FxHashMap<String, OpenDocument>,
    previous_result_id: Option<String>,
) -> Result<DocumentDiagnosticReportKind> {
    docs.get(&uri.to_string()).context("unknown doc")?;
    let (result_id, items) = diagnostics(client, uri, docs)?;
    if let Some(previous_id) = previous_result_id
        && previous_id == result_id
    {
        Ok(DocumentDiagnosticReportKind::Unchanged(
            UnchangedDocumentDiagnosticReport { result_id },
        ))
    } else {
        Ok(DocumentDiagnosticReportKind::Full(
            FullDocumentDiagnosticReport {
                items,
                result_id: Some(result_id),
            },
        ))
    }
}

/// publish diagnostics (push)
pub fn push_diagnostics(
    client: &Client,
    uri: &Uri,
    docs: &FxHashMap<String, OpenDocument>,
) -> Result<()> {
    let doc = docs.get(&uri.to_string()).context("unknown doc")?;
    let (_, items) = diagnostics(client, uri, docs)?;
    client
        .connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            PublishDiagnostics::METHOD.to_owned(),
            PublishDiagnosticsParams {
                diagnostics: items,
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

/// return diagnostics
fn diagnostics(
    client: &Client,
    uri: &Uri,
    docs: &FxHashMap<String, OpenDocument>,
) -> Result<(String, Vec<Diagnostic>)> {
    let doc = docs
        .get(&uri.to_string())
        .context("document should exist")?;

    let line_index = LineIndex::new(&doc.text);
    let bytes = doc.text.as_bytes();

    let results = lint(&doc.tree, bytes)?;
    let result_id = hash_items(&results);

    Ok((
        result_id,
        results
            .iter()
            .filter_map(|diagnostic| {
                let rule = rule(diagnostic.rule_id);
                let start = client.encode_position(diagnostic.range.start, &line_index)?;
                let end = client.encode_position(diagnostic.range.end, &line_index)?;
                let lsp_severity = rule.severity.into();
                // all the context ranges are related information
                let mut related_information = diagnostic
                    .context
                    .iter()
                    .filter_map(|context| {
                        let related_start = client.encode_position(context.start, &line_index)?;
                        let related_end = client.encode_position(context.end, &line_index)?;
                        let related = DiagnosticRelatedInformation {
                            location: Location {
                                uri: uri.clone(),
                                range: Range::new(related_start, related_end),
                            },
                            message: rule.context_label.clone().unwrap_or_default(),
                        };
                        Some(related)
                    })
                    .collect::<Vec<_>>();
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
                Some(Diagnostic {
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
            .collect::<Vec<_>>(),
    ))
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
