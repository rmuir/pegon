use anyhow::{Context as _, Error};
use core::ops::{ControlFlow, Range};
use core::sync::atomic::{AtomicBool, Ordering};
use rustc_hash::FxHashMap;
use std::sync::{Arc, LazyLock};
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _, Tree,
};

use crate::support::queries::capture_id;

/// Single variable scope entry
pub struct Scope {
    /// range of the identifier declaration
    pub identifier: Range<usize>,
    /// range where the identifier is valid
    pub range: Range<usize>,
    /// semantic token type (indexes into the legend)
    pub token_type: u32,
}

/// Returns a map of scopes keyed by identifier in the document
///
/// # Errors
///
/// This function will return an error if rules are misconfigured.
pub fn scopes(
    tree: &Tree,
    data: &[u8],
    cancel_token: &Arc<AtomicBool>,
) -> Result<FxHashMap<String, Vec<Scope>>, Error> {
    let mut scopes = FxHashMap::default();
    let mut cursor = QueryCursor::new();

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
        tree.root_node(),
        data,
        QueryCursorOptions::new().progress_callback(&mut cancellation),
    );
    while let Some(hit) = matches.next() {
        let pattern = pattern(hit.pattern_index);

        let var_node = hit
            .nodes_for_capture_index(*DEFINITION_CAPTURE)
            .next()
            .context("definition capture should exist")?;

        let mut start_node = hit
            .nodes_for_capture_index(*START_CAPTURE)
            .next()
            .context("start capture should exist")?;

        let mut end_node = hit
            .nodes_for_capture_index(*END_CAPTURE)
            .next()
            .context("end capture should exist")?;

        if pattern.flow {
            let mut node = tree.root_node();
            while let Some(child) = node.child_with_descendant(var_node) {
                if child.kind() == "block" {
                    start_node = child;
                    end_node = child;
                }
                node = child;
            }
        }

        let key = var_node.utf8_text(data)?.to_owned();
        let value = scopes.entry(key).or_insert_with(|| Vec::with_capacity(4));
        value.push(Scope {
            identifier: var_node.start_byte()..var_node.end_byte(),
            token_type: pattern.token_type,
            range: if pattern.start_inclusive {
                start_node.start_byte()..end_node.end_byte()
            } else {
                start_node.end_byte()..end_node.end_byte()
            },
        });
    }
    Ok(scopes)
}

/// single compiled pattern
pub struct Pattern {
    /// semantic token type (indexes into the legend)
    pub token_type: u32,
    /// whether the start capture is inclusive or exclusive
    pub start_inclusive: bool,
    /// whether scope is based on control flow rather than lexical
    pub flow: bool,
}

/// Look up rule by pattern index
#[must_use]
pub fn pattern(index: usize) -> &'static Pattern {
    PATTERNS.get(index).expect("pattern should exist")
}

/// compiled query that matches all lint rules
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::language(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/scopes.scm"
        )),
    )
    .expect("query should compile")
});

/// array of patterns indexed by patterns of `QUERY`
static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    let count = QUERY.pattern_count();
    let mut patterns = Vec::with_capacity(count);
    for index in 0..count {
        let mut token_type = None;
        let mut start_inclusive = true;
        let mut flow = false;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "scope.flow" => {
                    let value = value.expect("scope.flow should have a value");
                    flow = value.parse::<bool>().expect("valid boolean");
                }
                "scope.type" => {
                    let value = value.expect("token.type should have a value");
                    token_type = crate::lsp::SEMANTIC_TOKEN_TYPES.binary_search(&value).ok();
                    assert!(token_type.is_some(), "unknown token type: {value}");
                }
                "scope.start.inclusive" => {
                    let value = value.expect("scope.start.inclusive should have a value");
                    start_inclusive = value.parse::<bool>().expect("valid boolean");
                }
                _ => panic!("{key}: unknown metadata key"),
            }
        }
        patterns.push(Pattern {
            token_type: token_type
                .expect("token.type should be set")
                .try_into()
                .expect("should be u32"),
            start_inclusive,
            flow,
        });
    }
    patterns
});

/// index of the `@definition` capture
static DEFINITION_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "definition"));

/// index of the `@start` capture
static START_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "start"));

/// index of the `@end` capture
static END_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "end"));
