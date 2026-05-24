use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{FoldingRange, FoldingRangeKind};
use tree_sitter::{Query, QueryCursor, StreamingIterator as _};

use crate::lsp::{Client, server::Document};

pub fn request(client: &Client, doc: &Document) -> Result<Vec<FoldingRange>> {
    let bytes = doc.text.as_bytes();
    let mut result = Vec::new();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&QUERY, doc.tree.root_node(), bytes);
    while let Some(hit) = matches.next() {
        let pattern = pattern(hit.pattern_index);
        let mut nodes = hit.nodes_for_capture_index(*RANGE_CAPTURE);
        let node = nodes.next().expect("should have range capture");
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
        if pattern.line_offset > 0 {
            result.push(FoldingRange {
                start_line: range
                    .start
                    .line
                    .checked_add(pattern.line_offset)
                    .context("should not overflow")?,
                start_character: Some(0),
                end_line: range.end.line,
                end_character: Some(range.end.character),
                kind: pattern.kind.clone(),
                collapsed_text: None,
            });
        } else {
            result.push(FoldingRange {
                start_line: range.start.line,
                start_character: Some(range.start.character),
                end_line: range.end.line,
                end_character: Some(range.end.character),
                kind: pattern.kind.clone(),
                collapsed_text: None,
            });
        }
    }
    Ok(result)
}

/// single compiled pattern
struct Pattern {
    /// kind of fold
    kind: Option<FoldingRangeKind>,
    /// adjustment to start line
    line_offset: u32,
}

// Look up rule by pattern index
#[must_use]
fn pattern(index: usize) -> &'static Pattern {
    PATTERNS.get(index).expect("pattern should exist")
}

/// compiled query that matches all folding patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::LANGUAGE.into(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/folds.scm"
        )),
    )
    .expect("query should compile")
});

/// array of rules indexed by patterns of `QUERY`
static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    let count = QUERY.pattern_count();
    let mut patterns = Vec::with_capacity(count);
    for index in 0..count {
        let mut kind: Option<&str> = None;
        let mut line_offset: Option<u32> = None;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "kind" => {
                    kind = value;
                }
                "lineoffset" => {
                    line_offset = Some(1);
                }
                _ => {}
            }
        }
        patterns.push(Pattern {
            kind: match kind {
                Some("comment") => Some(FoldingRangeKind::Comment),
                Some("imports") => Some(FoldingRangeKind::Imports),
                Some(_) => panic!("unsupported fold kind {kind:?}"),
                None => Some(FoldingRangeKind::Region),
            },
            line_offset: line_offset.unwrap_or_default(),
        });
    }
    patterns
});

static RANGE_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("range")
        .expect("range capture should exist")
});
