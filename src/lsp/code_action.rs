use core::sync::atomic::AtomicBool;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use gen_lsp_types::{
    Code, CodeAction, CodeActionKind, CodeActionParams, CodeActionResponse, DocumentChange, Edit,
    OptionalVersionedTextDocumentIdentifier, TextDocumentEdit, TextDocumentIdentifier, TextEdit,
    Uri, WorkspaceEdit,
};
use serde::{Deserialize, Serialize};

use crate::support::diagnostics::rule_by_name;

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &CodeActionParams,
) -> Result<Vec<CodeActionResponse>> {
    if client.supports_code_action_data() && client.supports_code_action_resolve_edit() {
        let mut result = Vec::with_capacity(params.context.diagnostics.len().saturating_add(1));
        let only = params.context.only.as_ref();
        let data = Some(serde_json::to_value(CustomData {
            uri: params.text_document.uri.clone(),
            version: doc.version,
        })?);
        if only.is_none_or(|only| only.contains(&CodeActionKind::QuickFix)) {
            for diagnostic in &params.context.diagnostics {
                if let Some(value) = &diagnostic.data {
                    let diagnostics_data: super::diagnostics::CustomData =
                        serde_json::from_value(value.clone())?;
                    // try to form the title from "code: Title of Fix"
                    let title = match &diagnostic.code {
                        Some(Code::String(code)) => format!("{code}: {}", diagnostics_data.fix),
                        _ => diagnostics_data.fix,
                    };
                    result.push(CodeActionResponse::CodeAction(CodeAction {
                        title,
                        kind: Some(CodeActionKind::QuickFix),
                        diagnostics: Some(vec![diagnostic.clone()]),
                        is_preferred: Some(true),
                        disabled: None,
                        edit: None,
                        command: None,
                        data: data.clone(),
                        tags: None,
                    }));
                }
            }
        }
        if only.is_none_or(|only| {
            only.contains(&CodeActionKind::Source)
                || only.contains(&CodeActionKind::SourceOrganizeImports)
        }) {
            result.push(CodeActionResponse::CodeAction(CodeAction {
                title: "Organize Imports".into(),
                kind: Some(CodeActionKind::SourceOrganizeImports),
                diagnostics: None,
                is_preferred: None,
                disabled: None,
                edit: None,
                command: None,
                data,
                tags: None,
            }));
        }
        Ok(result)
    } else {
        // just return empty code actions if the client can't be efficient about it
        Ok(vec![])
    }
}

#[derive(Serialize, Deserialize)]
pub struct CustomData {
    pub uri: Uri,
    pub version: i32,
}

pub fn resolve(
    client: &Client,
    doc: &Document,
    params: &CodeAction,
    data: &CustomData,
    _cancel: &Arc<AtomicBool>,
) -> Result<CodeAction> {
    let mut result = params.clone();
    let edit = match params.kind {
        Some(CodeActionKind::QuickFix) => quickfix(client, doc, params)?,
        Some(CodeActionKind::SourceOrganizeImports) => bail!("not just yet"),
        _ => bail!("invalid or missing kind"),
    };
    if let Some(edit) = edit {
        result.edit = Some(if client.supports_document_changes() {
            WorkspaceEdit {
                changes: None,
                document_changes: Some(vec![DocumentChange::TextDocumentEdit(TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        version: Some(doc.version),
                        text_document_identifier: TextDocumentIdentifier::new(data.uri.clone()),
                    },
                    edits: vec![Edit::TextEdit(edit)],
                })]),
                change_annotations: None,
            }
        } else {
            WorkspaceEdit {
                changes: Some(HashMap::from([(data.uri.clone(), vec![edit])])),
                document_changes: None,
                change_annotations: None,
            }
        });
    }
    Ok(result)
}

fn quickfix(client: &Client, doc: &Document, params: &CodeAction) -> Result<Option<TextEdit>> {
    let diagnostics = params.diagnostics.as_ref().context("missing diagnostics")?;
    let diagnostic = diagnostics.first().context("missing diagnostics")?;
    let range = client
        .decode_range(&diagnostic.range, &doc.line_index)
        .context("valid range")?;
    let rule = match &diagnostic.code {
        Some(Code::String(name)) => rule_by_name(name).context("invalid code")?,
        _ => bail!("invalid or missing code"),
    };
    if let Some(fix) = &rule.fix
        && let Some(edit) = fix.generate(
            range.start_byte..range.end_byte,
            &doc.tree,
            doc.text.as_bytes(),
        )?
    {
        let ts_range = tree_sitter::Range {
            start_byte: edit.range.start,
            end_byte: edit.range.end,
            start_point: Client::to_point(edit.range.start, &doc.line_index)
                .context("valid offset")?,
            end_point: Client::to_point(edit.range.end, &doc.line_index).context("valid offset")?,
        };
        let encoded = client
            .encode_range(&ts_range, &doc.line_index)
            .context("valid range")?;
        return Ok(Some(TextEdit::new(encoded, edit.replacement)));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use gen_lsp_types::{
        ClientCapabilities, ClientCodeActionResolveOptions, Code, CodeAction,
        CodeActionClientCapabilities, CodeActionContext, CodeActionKind, CodeActionParams,
        CodeActionRequest, CodeActionResolveRequest, CodeActionResponse,
        DiagnosticClientCapabilities, DiagnosticSeverity, DiagnosticsCapabilities,
        DidOpenTextDocumentNotification, DidOpenTextDocumentParams, DocumentDiagnosticParams,
        DocumentDiagnosticReport, DocumentDiagnosticRequest, InitializeParams, Message,
        PartialResultParams, Position, Range, TextDocumentClientCapabilities,
        TextDocumentIdentifier, TextDocumentItem, TextEdit, Uri, WorkDoneProgressParams,
        WorkspaceEdit,
    };
    use indoc::indoc;
    use serde_json::json;

    use crate::lsp::test_client::TestClient;

    fn capabilities() -> ClientCapabilities {
        ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities {
                code_action: Some(CodeActionClientCapabilities {
                    data_support: Some(true),
                    resolve_support: Some(ClientCodeActionResolveOptions {
                        properties: vec!["edit".into(), "command".into()],
                    }),
                    ..Default::default()
                }),
                diagnostic: Some(DiagnosticClientCapabilities {
                    diagnostics_capabilities: DiagnosticsCapabilities {
                        data_support: Some(true),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Get an autofix for a swallowed exception
    #[test]
    fn basic() {
        let client = TestClient::new(InitializeParams {
            capabilities: capabilities(),
            ..Default::default()
        });
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {r#"
                public class Foo {
                    public void bar() {
                        try {
                            Integer.parseInt("foo");
                        } catch (Exception wtf) {
                        }
                    }
                }
            "#}
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

        let diagnostics = &full.full_document_diagnostic_report.items;

        // one problem
        assert_eq!(
            vec![gen_lsp_types::Diagnostic {
                range: Range::new(Position::new(4, 27), Position::new(4, 30)),
                severity: Some(DiagnosticSeverity::Hint),
                code: Some(Code::String("swallowed-exception".into())),
                source: Some(env!("CARGO_PKG_NAME").into()),
                message: Message::String("Unhandled caught exception: `wtf`".into()),
                data: Some(json!({ "fix": "Indicate ignored exception with unnamed variable `_`"})),
                ..Default::default()
            }],
            diagnostics.clone()
        );

        let action_list = client.request::<CodeActionRequest>(CodeActionParams {
            text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
            range: Range::new(Position::new(0, 0), Position::new(7, 0)),
            context: CodeActionContext {
                diagnostics: diagnostics.clone(),
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });

        assert_eq!(
            action_list,
            Some(vec![
                CodeActionResponse::CodeAction(CodeAction {
                    title:
                        "swallowed-exception: Indicate ignored exception with unnamed variable `_`"
                            .into(),
                    kind: Some(CodeActionKind::QuickFix),
                    diagnostics: Some(diagnostics.clone()),
                    is_preferred: Some(true),
                    disabled: None,
                    edit: None,
                    command: None,
                    data: Some(json!({
                        "uri": "file:///Foo.java",
                        "version": 0
                    })),
                    tags: None,
                }),
                CodeActionResponse::CodeAction(CodeAction {
                    title: "Organize Imports".into(),
                    kind: Some(CodeActionKind::SourceOrganizeImports),
                    diagnostics: None,
                    is_preferred: None,
                    disabled: None,
                    edit: None,
                    command: None,
                    data: Some(json!({
                        "uri": "file:///Foo.java",
                        "version": 0
                    })),
                    tags: None
                })
            ])
        );

        let resolved = client.request::<CodeActionResolveRequest>(CodeAction {
            title: "Indicate ignored exception with unnamed variable `_`".into(),
            kind: Some(CodeActionKind::QuickFix),
            diagnostics: Some(diagnostics.clone()),
            is_preferred: Some(true),
            disabled: None,
            edit: None,
            command: None,
            data: Some(json!({
                "uri": "file:///Foo.java",
                "version": 0
            })),
            tags: None,
        });

        assert_eq!(
            resolved.edit,
            Some(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    Uri("file:///Foo.java".into()),
                    vec![TextEdit {
                        range: Range::new(Position::new(4, 27), Position::new(4, 30)),
                        new_text: "_".into()
                    }]
                )])),
                document_changes: None,
                change_annotations: None,
            }),
        );
    }
}
