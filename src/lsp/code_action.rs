use core::sync::atomic::AtomicBool;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use gen_lsp_types::{
    Code, CodeAction, CodeActionKind, CodeActionParams, CodeActionResponse, TextEdit, Uri,
    WorkspaceEdit,
};
use serde::{Deserialize, Serialize};

use crate::support::diagnostics::{Fix, rule_by_name};

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &CodeActionParams,
    _cancel_token: &Arc<AtomicBool>,
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
                    result.push(CodeActionResponse::CodeAction(CodeAction {
                        title: diagnostics_data.fix,
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
        // TODO: make this a quick fix, and if returned already, don't return here
        if only.is_none_or(|only| only.contains(&CodeActionKind::SourceOrganizeImports)) {
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
    _cancel_token: &Arc<AtomicBool>,
) -> Result<CodeAction> {
    let mut result = params.clone();
    let edit = match params.kind {
        Some(CodeActionKind::QuickFix) => quickfix(client, doc, params),
        Some(CodeActionKind::SourceOrganizeImports) => bail!("not just yet"),
        _ => bail!("invalid or missing kind"),
    }?;
    result.edit = Some(WorkspaceEdit {
        changes: Some(HashMap::from([(data.uri.clone(), vec![edit])])),
        document_changes: None, // TODO! check capability and use this way to send version
        change_annotations: None,
    });
    Ok(result)
}

fn quickfix(_client: &Client, _doc: &Document, params: &CodeAction) -> Result<TextEdit> {
    let diagnostics = params.diagnostics.as_ref().context("missing diagnostics")?;
    let diagnostic = diagnostics.first().context("missing diagnostics")?;
    let range = &diagnostic.range;
    let rule = match &diagnostic.code {
        Some(Code::String(name)) => rule_by_name(name).context("invalid code")?,
        _ => bail!("invalid or missing code"),
    };
    Ok(match &rule.fix {
        Some(Fix::Static(replacement)) => TextEdit::new(*range, replacement.clone()),
        None => bail!("invalid code"),
    })
}
