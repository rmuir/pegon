use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};

use std::{
    collections::HashSet,
    sync::{Arc, LazyLock},
};

use anyhow::{Context as _, Result};
use gen_lsp_types::{DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams};
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _,
};

use crate::support::queries::capture_id;

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &DocumentHighlightParams,
    cancel_token: &Arc<AtomicBool>,
) -> Result<Vec<DocumentHighlight>> {
    let bytes = doc.text.as_bytes();
    let position = params.text_document_position_params.position;
    let mut result = Vec::with_capacity(3);
    let mut cursor = QueryCursor::new();
    let linecol = client
        .decode_pos(position, &doc.line_index)
        .context("valid offset")?;
    let source_position: usize = doc
        .line_index
        .offset(linecol)
        .context("valid offset")?
        .into();
    cursor.set_byte_range(source_position..source_position.checked_add(1).context("no overflow")?);

    // this callback MUST be a separate let-binding. do *NOT* factor into anonymous closure!
    let mut cancellation = |_: &QueryCursorState| {
        if cancel_token.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };

    let mut matches = cursor.matches_with_options(
        &QUERY,
        doc.tree.root_node(),
        bytes,
        QueryCursorOptions::new().progress_callback(&mut cancellation),
    );
    let mut seen_matches = HashSet::new();
    while let Some(hit) = matches.next() {
        let mut found = false;
        // check if it is a true match, we must be inside a range capture
        for node in hit.nodes_for_capture_index(*RANGE_CAPTURE) {
            if source_position < node.range().start_byte || source_position > node.range().end_byte
            {
                continue;
            }
            found = true;
            break;
        }
        if !found {
            continue;
        }
        let pattern = pattern(hit.pattern_index);
        for node in hit.nodes_for_capture_index(*REFERENCE_CAPTURE) {
            if !seen_matches.insert(node.id()) {
                continue;
            }
            let range = client
                .encode_range(&node.range(), &doc.line_index)
                .context("valid range")?;
            let kind = Some(pattern.kind);
            result.push(DocumentHighlight { range, kind });
        }
    }
    Ok(result)
}

/// single compiled pattern
struct Pattern {
    /// kind of references
    kind: DocumentHighlightKind,
}

/// Look up rule by pattern index
#[must_use]
fn pattern(index: usize) -> &'static Pattern {
    PATTERNS.get(index).expect("pattern should exist")
}

/// compiled query that matches all folding patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::LANGUAGE.into(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/highlights.scm"
        )),
    )
    .expect("query should compile")
});

/// array of rules indexed by patterns of `QUERY`
static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    let count = QUERY.pattern_count();
    let mut patterns = Vec::with_capacity(count);
    for index in 0..count {
        let mut kind: Option<DocumentHighlightKind> = None;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "highlight.kind" => {
                    let code = value
                        .expect("kind should have a value")
                        .parse::<u32>()
                        .expect("kind should be an integer");
                    kind = Some(
                        DocumentHighlightKind::try_from(code)
                            .expect("kind should be a valid DocumentHighlightKind"),
                    );
                }
                _ => panic!("{key}: unknown metadata key"),
            }
        }
        patterns.push(Pattern {
            kind: kind.expect("should exist"),
        });
    }
    patterns
});

static RANGE_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "range"));

static REFERENCE_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "reference"));
