use anyhow::{Context as _, Error};
use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::sync::LazyLock;
use tree_sitter::{
    Node, Query, QueryCursor, QueryCursorOptions, QueryCursorState, Range, StreamingIterator as _,
    Tree,
};

use crate::java_constants::kinds;
use crate::java_queries::locals::captures;
use crate::java_queries::locals::properties::{PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN};
use crate::support::queries::{KindSet, const_array_from_fn, const_table_search, to_bool_const};

pub struct Scopes<'data, 'tree> {
    pub locals: FxHashMap<&'data str, Vec<LocalScope<'tree>>>,
    // pub types (should be useful for qualification)
}

/// Single variable scope entry
pub struct LocalScope<'tree> {
    /// range of the identifier declaration
    pub identifier: Range,
    /// range where the identifier is valid
    pub range: Range,
    /// semantic token type
    pub token_type: u32,
    /// java unqualified type
    pub java_type: Option<Node<'tree>>,
}

impl LocalScope<'_> {
    /// true if the scope contains specified position
    pub const fn contains(&self, position: usize) -> bool {
        (self.range.start_byte <= position && self.range.end_byte >= position)
            || (self.identifier.start_byte <= position && self.identifier.end_byte >= position)
    }
}

/// Returns a map of scopes keyed by identifier in the document
///
/// # Errors
///
/// This function will return an error if rules are misconfigured.
pub fn scopes<'tree, 'data>(
    tree: &'tree Tree,
    data: &'data [u8],
    cancel: &AtomicBool,
) -> Result<Scopes<'data, 'tree>, Error> {
    let mut locals = FxHashMap::with_capacity_and_hasher(64, FxBuildHasher);
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
        tree.root_node(),
        data,
        QueryCursorOptions::new().progress_callback(&mut cancellation),
    );
    while let Some(hit) = matches.next() {
        let pattern = pattern(hit.pattern_index);

        let var_node = hit
            .nodes_for_capture_index(captures::DEFINITION)
            .next()
            .context("definition capture should exist")?;

        let mut start_node = hit
            .nodes_for_capture_index(captures::START)
            .next()
            .context("start capture should exist")?;

        let mut end_node = hit
            .nodes_for_capture_index(captures::END)
            .next()
            .context("end capture should exist")?;

        let type_node = hit.nodes_for_capture_index(captures::TYPE).next();

        if pattern.flow {
            let mut node = tree.root_node();
            while let Some(child) = node.child_with_descendant(var_node) {
                if FLOW_BLOCK_KINDS.contains(child.kind_id()) {
                    start_node = child;
                    end_node = child;
                }
                node = child;
            }
        }

        let key = var_node.utf8_text(data)?;
        let value = locals.entry(key).or_insert_with(|| Vec::with_capacity(4));
        let start_range = start_node.range();
        let end_range = end_node.range();
        value.push(LocalScope {
            identifier: var_node.range(),
            token_type: pattern.token_type,
            java_type: type_node,
            range: if pattern.start_inclusive {
                Range {
                    start_byte: start_range.start_byte,
                    start_point: start_range.start_point,
                    end_byte: end_range.end_byte,
                    end_point: end_range.end_point,
                }
            } else {
                Range {
                    start_byte: start_range.end_byte,
                    start_point: start_range.end_point,
                    end_byte: end_range.end_byte,
                    end_point: end_range.end_point,
                }
            },
        });
    }
    Ok(Scopes { locals })
}

/// single compiled pattern
pub struct Pattern {
    /// semantic token type
    pub token_type: u32,
    /// whether the start capture is inclusive or exclusive
    pub start_inclusive: bool,
    /// whether scope is based on control flow rather than lexical
    pub flow: bool,
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
#[expect(clippy::cast_possible_truncation, reason = "compile time safety")]
const fn to_pattern(pattern: usize) -> Pattern {
    let range = &PROPERTIES_BY_PATTERN[pattern];
    let mut index = range.start;
    let mut token_type = None;
    let mut start_inclusive = true;
    let mut flow = false;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"local.type" => {
                token_type = Some(const_table_search(
                    &super::semantic_tokens::TOKEN_TYPES,
                    property.1,
                ));
            }
            b"local.flow" => flow = to_bool_const(property.1),
            b"local.start.inclusive" => start_inclusive = to_bool_const(property.1),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    Pattern {
        token_type: token_type.expect("local.type should be set") as u32,
        start_inclusive,
        flow,
    }
}

/// compiled query that matches all lint rules
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::language(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/locals.scm"
        )),
    )
    .expect("query should compile")
});

/// for flow scoping, the parent node types where variables scope can "escape" into
const FLOW_BLOCK_KINDS: KindSet = KindSet::new(&[kinds::BLOCK, kinds::CONSTRUCTOR_BODY]);
