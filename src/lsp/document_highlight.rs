use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};

use rustc_hash::FxHashSet;
use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams};
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _,
};

use super::{Client, server::Document};
use crate::java_queries::highlights::captures;
use crate::java_queries::highlights::properties::{
    PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN,
};
use crate::support::queries::const_array_from_fn;

pub fn request(
    client: &Client,
    doc: &Document,
    params: &DocumentHighlightParams,
    cancel: &AtomicBool,
) -> Result<Option<Vec<DocumentHighlight>>> {
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
        if cancel.load(Ordering::Relaxed) {
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
    let mut seen_matches = FxHashSet::default();
    while let Some(hit) = matches.next() {
        let mut found = false;
        // check if it is a true match, we must be inside a range capture
        for node in hit.nodes_for_capture_index(captures::RANGE) {
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
        for node in hit.nodes_for_capture_index(captures::REFERENCE) {
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
    Ok(Some(result))
}

/// single compiled pattern
struct Pattern {
    /// kind of references
    kind: DocumentHighlightKind,
}

/// Look up pattern by index
#[expect(clippy::indexing_slicing, reason = "compile time safety")]
const fn pattern(index: usize) -> &'static Pattern {
    &PATTERNS[index]
}

/// array of pattern metadata from `QUERY` by index
#[expect(clippy::indexing_slicing, reason = "compile time safety")]
#[expect(clippy::arithmetic_side_effects, reason = "compile time safety")]
const PATTERNS: [Pattern; PATTERN_COUNT] = const_array_from_fn!(to_pattern, PATTERN_COUNT);

#[expect(clippy::indexing_slicing, reason = "compile time safety")]
#[expect(clippy::arithmetic_side_effects, reason = "compile time safety")]
const fn to_pattern(pattern: usize) -> Pattern {
    let range = &PROPERTIES_BY_PATTERN[pattern];
    let mut index = range.start;
    let mut kind: Option<DocumentHighlightKind> = None;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"highlight.kind" => kind = Some(to_kind(property.1)),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    Pattern {
        kind: kind.expect("kind should be set"),
    }
}

/// parse a kind into an lsp kind
const fn to_kind(string: &str) -> DocumentHighlightKind {
    match string.as_bytes() {
        b"read" => DocumentHighlightKind::Read,
        b"text" => DocumentHighlightKind::Text,
        b"write" => DocumentHighlightKind::Write,
        _ => panic!("invalid kind"),
    }
}

/// compiled query that matches all folding patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::LANGUAGE,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/highlights.scm"
        )),
    )
    .expect("query should compile")
});

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        DidOpenTextDocumentNotification, DidOpenTextDocumentParams, DocumentHighlight,
        DocumentHighlightKind, DocumentHighlightParams, DocumentHighlightRequest,
        PartialResultParams, Position, Range, TextDocumentIdentifier, TextDocumentItem,
        TextDocumentPositionParams, WorkDoneProgressParams,
    };
    use indoc::indoc;

    use crate::lsp::test_client::TestClient;

    /// simple document
    #[test]
    fn simple() {
        let client = TestClient::default();
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                public class foo {
                    public abstract void bar(int x) {
                        try {
                            baz();
                        } finally {
                            System.exit(0);
                        }
                    }
                }
            "}
                .into(),
            },
        });
        let result = client
            .request::<DocumentHighlightRequest>(DocumentHighlightParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
                    position: Position::new(2, 9),
                },
                partial_result_params: PartialResultParams::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        assert_eq!(
            result,
            vec![
                // try
                DocumentHighlight {
                    kind: Some(DocumentHighlightKind::Read),
                    range: Range::new(Position::new(2, 8), Position::new(2, 11))
                },
                // finally
                DocumentHighlight {
                    kind: Some(DocumentHighlightKind::Read),
                    range: Range::new(Position::new(4, 10), Position::new(4, 17))
                }
            ]
        );
    }
}
