use std::collections::HashMap;

use anyhow::Context as _;
use anyhow::Result;
use line_index::LineIndex;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    PublishDiagnosticsParams,
};
use tree_sitter::{InputEdit, Parser};

use crate::lsp::diagnostics;
use crate::lsp::{client::Client, server::Document};

pub fn did_open(
    client: &Client,
    params: DidOpenTextDocumentParams,
    docs: &mut HashMap<String, Document>,
    parser: &mut Parser,
) -> Result<Option<PublishDiagnosticsParams>> {
    let uri = params.text_document.uri;
    parser.reset();
    let tree = parser
        .parse(&params.text_document.text, None)
        .context("broken parser setup")?;
    let line_index = LineIndex::new(&params.text_document.text);
    let doc = Document {
        text: params.text_document.text,
        version: params.text_document.version,
        tree,
        line_index,
    };
    let push = if client.supports_pull_diagnostics() {
        None
    } else {
        Some(diagnostics::push(client, &doc, &uri)?)
    };
    docs.insert(uri.to_string(), doc);
    Ok(push)
}

pub fn did_change(
    client: &Client,
    params: DidChangeTextDocumentParams,
    docs: &mut HashMap<String, Document>,
    parser: &mut Parser,
) -> Result<Option<PublishDiagnosticsParams>> {
    let uri = params.text_document.uri;
    let doc = docs.remove(&uri.to_string()).context("document not open")?;
    let mut text = doc.text;
    let mut old_tree = doc.tree;
    let mut line_index = LineIndex::new(&text);
    for change in params.content_changes {
        let decoded = client
            .decode_change(&change, &line_index)
            .context("illegal range")?;
        // validate range is legal UTF-8
        text.get(decoded.start_byte..decoded.end_byte)
            .context("illegal slice")?;
        // edit document
        text.replace_range(decoded.start_byte..decoded.end_byte, &change.text);
        // rebuild index
        line_index = LineIndex::new(&text);
        // edit parse tree
        let new_end_byte = decoded
            .start_byte
            .checked_add(change.text.len())
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
    parser.reset();
    let tree = parser
        .parse(&text, Some(&old_tree))
        .context("broken parser setup")?;
    let newdoc = Document {
        text,
        version: params.text_document.version,
        tree,
        line_index,
    };

    let push = if client.supports_pull_diagnostics() {
        None
    } else {
        Some(diagnostics::push(client, &newdoc, &uri)?)
    };
    docs.insert(uri.to_string(), newdoc);
    Ok(push)
}

pub fn did_close(
    client: &Client,
    params: DidCloseTextDocumentParams,
    docs: &mut HashMap<String, Document>,
) -> Option<PublishDiagnosticsParams> {
    let uri = params.text_document.uri;
    docs.remove(&uri.to_string());
    // according to LSP spec, we should clear on close if we are pushing
    if client.supports_pull_diagnostics() {
        None
    } else {
        Some(PublishDiagnosticsParams {
            diagnostics: vec![],
            uri,
            version: None,
        })
    }
}
