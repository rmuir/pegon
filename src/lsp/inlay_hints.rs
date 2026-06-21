use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{InlayHint, InlayHintParams, Label};
use tree_sitter::{Query, QueryCursor, StreamingIterator as _};

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &InlayHintParams,
) -> Result<Vec<InlayHint>> {
    let bytes = doc.text.as_bytes();
    let range = client
        .decode_range(&params.range, &doc.line_index)
        .context("valid range")?;
    let mut result = Vec::with_capacity(3);
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range.start_byte..range.end_byte);
    let mut matches = cursor.matches(&QUERY, doc.tree.root_node(), bytes);
    while let Some(hit) = matches.next() {
        let node = hit
            .nodes_for_capture_index(*POSITION_CAPTURE)
            .next()
            .context("position capture should exist")?;
        // TODO: make a general predicate for "iseol"
        if *bytes.get(node.end_byte()).unwrap_or(&b'\n') == b'\n' {
            let position = client
                .encode_range(&node.range(), &doc.line_index)
                .context("valid offset")?
                .end;
            let mut text = String::new();
            text.push_str("//");
            for part in hit.nodes_for_capture_index(*VALUE_CAPTURE) {
                text.push(' ');
                text.push_str(part.utf8_text(bytes)?);
            }
            let label = Label::String(text);
            result.push(InlayHint {
                position,
                label,
                kind: None,
                text_edits: None,
                tooltip: None,
                padding_left: Some(true),
                padding_right: Some(false),
                data: None,
            });
        }
    }
    Ok(result)
}

/// compiled query that matches all folding patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::LANGUAGE.into(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/hints.scm"
        )),
    )
    .expect("query should compile")
});

static VALUE_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("value")
        .expect("value capture should exist")
});

static POSITION_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("position")
        .expect("position capture should exist")
});
