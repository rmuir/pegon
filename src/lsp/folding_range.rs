use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};

use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::FoldingRange;
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _,
};

use super::{Client, server::Document};
use crate::java_queries::folds::captures;
use crate::java_queries::folds::properties::{PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN};
use crate::support::queries::{const_array_from_fn, to_bool_const};

pub fn request(
    client: &Client,
    doc: &Document,
    cancel: &AtomicBool,
) -> Result<Option<Vec<FoldingRange>>> {
    let bytes = doc.text.as_bytes();
    let mut result = Vec::with_capacity(16);
    let mut cursor = QueryCursor::new();

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
    while let Some(hit) = matches.next() {
        let pattern = pattern(hit.pattern_index);
        let mut nodes = hit.nodes_for_capture_index(captures::RANGE);
        let node = nodes.next().context("should have range capture")?;
        let start_range = node.range();
        let end_range = nodes.last().unwrap_or(node).range();
        let range = tree_sitter::Range {
            start_byte: start_range.start_byte,
            end_byte: end_range.end_byte,
            start_point: start_range.start_point,
            end_point: end_range.end_point,
        };
        let range = client
            .encode_range(&range, &doc.line_index)
            .context("valid range")?;
        if pattern.nextline {
            result.push(FoldingRange {
                start_line: range
                    .start
                    .line
                    .checked_add(1)
                    .context("should not overflow")?,
                start_character: Some(0),
                end_line: range.end.line,
                end_character: Some(range.end.character),
                kind: Some(pattern.kind.into()),
                collapsed_text: None,
            });
        } else {
            result.push(FoldingRange {
                start_line: range.start.line,
                start_character: Some(range.start.character),
                end_line: range.end.line,
                end_character: Some(range.end.character),
                kind: Some(pattern.kind.into()),
                collapsed_text: None,
            });
        }
    }
    Ok(Some(result))
}

/// single compiled pattern
struct Pattern {
    /// kind of fold
    kind: &'static str,
    /// adjustment to start line
    nextline: bool,
}

/// Look up rule by pattern index
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
    let mut kind: Option<&str> = None;
    let mut nextline = false;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"fold.kind" => kind = Some(to_kind(property.1)),
            b"fold.nextline" => nextline = to_bool_const(property.1),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    Pattern {
        kind: kind.expect("kind should be set"),
        nextline,
    }
}

const fn to_kind(string: &str) -> &str {
    match string.as_bytes() {
        b"comment" | b"imports" | b"region" => string,
        _ => panic!("unknown kind"),
    }
}

/// compiled query that matches all folding patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::LANGUAGE,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/folds.scm"
        )),
    )
    .expect("query should compile")
});

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        DidOpenTextDocumentNotification, DidOpenTextDocumentParams, FoldingRange, FoldingRangeKind,
        FoldingRangeParams, FoldingRangeRequest, PartialResultParams, TextDocumentIdentifier,
        TextDocumentItem, WorkDoneProgressParams,
    };
    use indoc::indoc;

    use crate::lsp::test_client::TestClient;

    /// simple document
    #[test]
    fn flat() {
        let client = TestClient::default();
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                import foo.bar.One;
                import foo.bar.Two;

                /**
                 * test
                 */
                public class foo {
                    public void bar(int x) {
                    }
                }
            "}
                .into(),
            },
        });
        let result = client
            .request::<FoldingRangeRequest>(FoldingRangeParams {
                text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
                partial_result_params: PartialResultParams::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        assert_eq!(
            result,
            [
                FoldingRange {
                    start_line: 0,
                    start_character: Some(0),
                    end_line: 1,
                    end_character: Some(19),
                    kind: Some(FoldingRangeKind::Imports),
                    collapsed_text: None
                },
                FoldingRange {
                    start_line: 4,
                    start_character: Some(0),
                    end_line: 5,
                    end_character: Some(3),
                    kind: Some(FoldingRangeKind::Comment),
                    collapsed_text: None
                },
                FoldingRange {
                    start_line: 7,
                    start_character: Some(27),
                    end_line: 8,
                    end_character: Some(5),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: None
                }
            ]
        );
    }
}
