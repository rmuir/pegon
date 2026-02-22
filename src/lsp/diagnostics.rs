use anyhow::{Result, bail};
use line_index::LineIndex;
use lsp_server::Message;
use lsp_types::{
    CodeDescription, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DocumentDiagnosticReportKind, FullDocumentDiagnosticReport, Location, NumberOrString,
    PublishDiagnosticsParams, Range, Uri,
    notification::{Notification, PublishDiagnostics},
};
use rustc_hash::FxHashMap;

use std::str::FromStr;

use crate::{
    lint::{Linter, Severity, rule},
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
    linter: &mut Linter,
) -> Result<DocumentDiagnosticReportKind> {
    if docs.get(&uri.to_string()).is_none() {
        bail!("unknown doc: {uri:?}");
    }
    Ok(DocumentDiagnosticReportKind::Full(
        FullDocumentDiagnosticReport {
            items: diagnostics(client, uri, docs, linter),
            ..Default::default()
        },
    ))
}

/// publish diagnostics (push)
pub fn push_diagnostics(
    client: &Client,
    uri: &Uri,
    docs: &FxHashMap<String, OpenDocument>,
    linter: &mut Linter,
) -> Result<()> {
    let Some(doc) = docs.get(&uri.to_string()) else {
        bail!("unknown doc: {uri:?}");
    };
    client
        .connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            PublishDiagnostics::METHOD.to_owned(),
            PublishDiagnosticsParams {
                diagnostics: diagnostics(client, uri, docs, linter),
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

/// return diagnostics
fn diagnostics(
    client: &Client,
    uri: &Uri,
    docs: &FxHashMap<String, OpenDocument>,
    linter: &mut Linter,
) -> Vec<Diagnostic> {
    let doc = docs.get(&uri.to_string()).unwrap();

    let line_index = LineIndex::new(&doc.text);

    linter
        .lint(doc.text.as_bytes())
        .unwrap_or_default()
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
                        href: Uri::from_str(&rule.url).unwrap(),
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
        .collect::<Vec<_>>()
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
