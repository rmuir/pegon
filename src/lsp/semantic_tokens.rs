use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{SemanticToken, SemanticTokens, SemanticTokensLegend, SemanticTokensParams};
use tree_sitter::{Query, QueryCursor, StreamingIterator as _};

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    _params: &SemanticTokensParams,
) -> Result<SemanticTokens> {
    let data = doc.text.as_bytes();
    let mut result = Vec::with_capacity(3);
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(&QUERY, doc.tree.root_node(), data);

    let mut previous_range = 0..0;
    let mut previous_pattern = 0;
    let mut previous_line = 0;
    let mut previous_start = 0;
    while let Some((hit, capture_id)) = captures.next() {
        let capture = hit.captures[*capture_id];
        if previous_range == capture.node.byte_range() {
            debug_assert!(
                false,
                "uhoh {} / {} : {:?}",
                previous_pattern, hit.pattern_index, previous_range
            );
            continue;
        }
        let range = client
            .encode_range(&capture.node.range(), &doc.line_index)
            .context("should encode")?;
        debug_assert!(range.start.line == range.end.line, "multiline unsupported");
        let pattern = pattern(hit.pattern_index);
        result.push(SemanticToken {
            delta_line: range
                .start
                .line
                .checked_sub(previous_line)
                .context("valid delta")?,
            delta_start: if range.start.line == previous_line {
                range
                    .start
                    .character
                    .checked_sub(previous_start)
                    .context("valid delta")?
            } else {
                range.start.character
            },
            length: range
                .end
                .character
                .checked_sub(range.start.character)
                .context("valid delta")?,
            token_type: pattern.token_type,
            token_modifiers_bitset: pattern.token_modifiers_bitset,
        });
        previous_line = range.start.line;
        previous_start = range.start.character;
        previous_range = capture.node.byte_range();
        previous_pattern = hit.pattern_index;
    }
    Ok(SemanticTokens {
        result_id: None,
        data: result,
    })
}

/// compiled query that matches all semantic tokens patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::LANGUAGE.into(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/tokens.scm"
        )),
    )
    .expect("query should compile")
});

pub static LEGEND: LazyLock<SemanticTokensLegend> = LazyLock::new(|| SemanticTokensLegend {
    token_types: vec![
        "method".into(),
        "namespace".into(),
        "property".into(),
        "type".into(),
    ],
    token_modifiers: vec!["definition".into(), "readonly".into(), "static".into()],
});

// single compiled pattern
struct Pattern {
    token_type: u32,
    token_modifiers_bitset: u32,
}

/// Look up rule by pattern index
#[must_use]
fn pattern(index: usize) -> &'static Pattern {
    PATTERNS.get(index).expect("pattern should exist")
}

/// array of rules indexed by patterns of `QUERY`
static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    let count = QUERY.pattern_count();
    let mut patterns = Vec::with_capacity(count);
    for index in 0..count {
        let mut token_type = None;
        let mut token_modifiers_bitset = 0;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "tokens.type" => {
                    let value = value.expect("tokens.type should have a value");
                    token_type = LEGEND.token_types.binary_search(&value.to_owned()).ok();
                }
                "tokens.modifiers" => {
                    let value = value.expect("tokens.modifiers should have a value");
                    for modifier in value.split(',') {
                        let bit = LEGEND
                            .token_modifiers
                            .binary_search(&modifier.to_owned())
                            .expect("valid modifier");
                        token_modifiers_bitset |= 1 << bit;
                    }
                }
                _ => panic!("{key}: unknown metadata key"),
            }
        }
        patterns.push(Pattern {
            token_type: token_type
                .expect("token.type should be set")
                .try_into()
                .expect("should be u32"),
            token_modifiers_bitset,
        });
    }
    patterns
});
