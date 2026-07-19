use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use gen_lsp_types::LanguageKind;
use gen_lsp_types::TextDocumentContentChangeEvent::{
    TextDocumentContentChangePartial, TextDocumentContentChangeWholeDocument,
};
use gen_lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    PublishDiagnosticsParams,
};
use line_index::LineIndex;
use tree_sitter::InputEdit;

use super::diagnostics;
use super::server::{Resource, State};
use super::{client::Client, server::Document};

pub fn did_open(
    client: &Client,
    params: DidOpenTextDocumentParams,
    state: &mut State,
) -> Result<Option<PublishDiagnosticsParams>> {
    let uri = params.text_document.uri;
    let lang = params.text_document.language_id;
    if lang != LanguageKind::Java {
        state.docs.insert(uri.to_string(), Resource::Other);
        bail!("non-java language_id: {lang}");
    }

    state.parser.reset();
    let tree = state
        .parser
        .parse(&params.text_document.text, None)
        .context("broken parser setup")?;
    let line_index = LineIndex::new(&params.text_document.text);
    let doc = Document {
        text: params.text_document.text,
        version: params.text_document.version,
        tree,
        line_index,
        workspace: state.workspace(&uri),
    };
    let push = if client.supports_pull_diagnostics() {
        None
    } else {
        Some(diagnostics::push(client, &doc, &uri)?)
    };
    if state
        .docs
        .insert(uri.to_string(), Resource::Java(Arc::new(doc)))
        .is_some()
    {
        bail!("was previously already open");
    }
    Ok(push)
}

pub fn did_change(
    client: &Client,
    params: DidChangeTextDocumentParams,
    state: &mut State,
) -> Result<Option<PublishDiagnosticsParams>> {
    let uri = params.text_document.text_document_identifier.uri;
    let resource = state
        .docs
        .remove(&uri.to_string())
        .context("document not open")?;
    let Resource::Java(doc) = resource else {
        return Ok(None);
    };
    let mut text = doc.text.clone();
    let mut old_tree = doc.tree.clone();
    let mut line_index = LineIndex::new(&text);
    // process each change in order, updating the line index and tree
    // TODO: this is simple and safe, but rust-analyzer has some interesting optos
    for change in params.content_changes {
        let decoded = client
            .decode_change(&change, &line_index)
            .context("illegal range")?;
        // validate range is legal UTF-8
        text.get(decoded.start_byte..decoded.end_byte)
            .context("illegal slice")?;
        let change_text = match change {
            TextDocumentContentChangePartial(partial) => partial.text,
            TextDocumentContentChangeWholeDocument(whole) => whole.text,
        };
        // edit document
        text.replace_range(decoded.start_byte..decoded.end_byte, &change_text);
        // rebuild index
        line_index = LineIndex::new(&text);
        // edit parse tree
        let new_end_byte = decoded
            .start_byte
            .checked_add(change_text.len())
            .context("overflow")?;
        let new_end_position =
            Client::to_point(new_end_byte, &line_index).context("illegal range")?;
        old_tree.edit(&InputEdit {
            start_byte: decoded.start_byte,
            old_end_byte: decoded.end_byte,
            new_end_byte,
            start_position: decoded.start_point,
            old_end_position: decoded.end_point,
            new_end_position,
        });
    }
    state.parser.reset();
    let tree = state
        .parser
        .parse(&text, Some(&old_tree))
        .context("broken parser setup")?;
    let newdoc = Document {
        text,
        version: params.text_document.version,
        tree,
        line_index,
        workspace: doc.workspace.clone(),
    };

    let push = if client.supports_pull_diagnostics() {
        None
    } else {
        Some(diagnostics::push(client, &newdoc, &uri)?)
    };
    state
        .docs
        .insert(uri.to_string(), Resource::Java(Arc::new(newdoc)));
    Ok(push)
}

pub fn did_close(
    client: &Client,
    params: DidCloseTextDocumentParams,
    state: &mut State,
) -> Result<Option<PublishDiagnosticsParams>> {
    let uri = params.text_document.uri;
    if state.docs.remove(&uri.to_string()).is_none() {
        bail!("was not previously open");
    }
    // according to LSP spec, we should clear on close if we are pushing
    if client.supports_pull_diagnostics() {
        Ok(None)
    } else {
        Ok(Some(PublishDiagnosticsParams {
            diagnostics: vec![],
            uri,
            version: None,
        }))
    }
}
