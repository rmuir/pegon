use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{Contents, Hover, MarkupContent, MarkupKind, Position};
use tree_sitter::{Query, QueryCursor, StreamingIterator as _};

use super::{Client, server::Document};

pub fn request(client: &Client, doc: &Document, position: Position) -> Result<Option<Hover>> {
    let markdown = client.prefers_hover_markdown();
    let bytes = doc.text.as_bytes();
    let mut result = None;
    let mut cursor = QueryCursor::new();
    let linecol = client
        .decode_pos(position, &doc.line_index)
        .context("should decode")?;
    let source_position: usize = doc
        .line_index
        .offset(linecol)
        .context("should be valid offset")?
        .into();
    cursor.set_byte_range(source_position..source_position.checked_add(1).context("no overflow")?);
    let mut matches = cursor.matches(&QUERY, doc.tree.root_node(), bytes);
    let mut best_match = 0;
    while let Some(hit) = matches.next() {
        // ensure last pattern-wins
        if hit.pattern_index < best_match {
            continue;
        }
        // check if it is a true match, we must be inside the range capture
        let node = hit
            .nodes_for_capture_index(*RANGE_CAPTURE)
            .next()
            .expect("should have range capture");
        if source_position < node.range().start_byte || source_position > node.range().end_byte {
            continue;
        }

        let text = node.utf8_text(bytes)?;
        let pattern = pattern(hit.pattern_index);
        let range = client
            .encode_range(&node.range(), &doc.line_index)
            .context("valid range")?;
        let kind = &pattern.kind;
        let spec = &pattern.spec;
        let description = &pattern.description;
        result = Some(Hover {
            contents: Contents::MarkupContent(MarkupContent {
                kind: if markdown {
                    MarkupKind::Markdown
                } else {
                    MarkupKind::PlainText
                },
                value: if markdown {
                    format!(
                        "```java\n{text}\n```\n---\n`{kind}`\n\n{description}\n\n[spec]({SPEC_PREFIX}{spec})"
                    )
                } else {
                    format!("{text}\n---\n{kind}\n\n{description}\n\nspec: {SPEC_PREFIX}{spec}")
                },
            }),
            range: Some(range),
        });
        best_match = hit.pattern_index;
    }
    Ok(result)
}

/// when linking to the specification, use this URL as the base
const SPEC_PREFIX: &str = "https://docs.oracle.com/javase/specs/jls/se26/html/";

/// single compiled pattern
struct Pattern {
    /// kind of node
    kind: String,
    /// link to spec
    spec: String,
    /// description of node
    description: String,
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
            "/queries/java/hover.scm"
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
        let mut spec: Option<&str> = None;
        let mut description: Option<&str> = None;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "hover.description" => description = value,
                "hover.kind" => kind = value,
                "hover.spec" => spec = value,
                _ => panic!("{key}: unknown metadata key"),
            }
        }
        patterns.push(Pattern {
            kind: kind.expect("should exist").into(),
            spec: spec.expect("should exist").into(),
            description: description.expect("should exist").into(),
        });
    }
    patterns
});

static RANGE_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("range")
        .expect("range capture should exist")
});
