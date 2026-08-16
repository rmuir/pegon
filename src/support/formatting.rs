use anyhow::{Context as _, Error};
use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _, Tree,
};

use crate::java_queries::format::captures;
use crate::java_queries::format::properties::{PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN};
use crate::support::queries::{const_array_from_fn, to_bool_const, to_i32_const};

/// Formats the document into `buffer`
///
/// # Errors
///
/// This function will return an error if rules are misconfigured.
pub fn format(
    tree: &Tree,
    data: &[u8],
    buffer: &mut Vec<u8>,
    cancel: &AtomicBool,
) -> Result<(), Error> {
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

    let indent_size = 2; // TODO
    let newline = b"\n"; // TODO

    let mut current_indent: i32 = 0;
    let mut current_line_size = 0;

    while let Some(hit) = matches.next() {
        // primary node node
        let node = hit
            .nodes_for_capture_index(captures::NODE)
            .next()
            .context("error capture should exist")?;
        let pattern = pattern(hit.pattern_index);

        current_indent += pattern.indent_delta;
        if current_line_size == 0 {
            let indent = indent_size * current_indent as usize;
            buffer.resize(buffer.len() + indent, b' ');
        }

        // write the node
        let bytes = data.get(node.byte_range()).context("valid range")?;
        current_line_size += bytes.len();
        buffer.extend_from_slice(bytes);

        // write newline after, if required
        if pattern.newline_after {
            buffer.extend_from_slice(newline);
            current_line_size = 0;
        }
    }
    Ok(())
}

/// single pattern
struct Pattern {
    // add newline after the node
    newline_after: bool,
    // apply indent after the node
    indent_before: bool,
    // adjust indentation level by delta
    indent_delta: i32,
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
    let mut newline_after = false;
    let mut indent_before = false;
    let mut indent_delta = 0;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"format.newline.after" => newline_after = to_bool_const(property.1),
            b"format.indent.before" => indent_before = to_bool_const(property.1),
            b"format.indent.delta" => indent_delta = to_i32_const(property.1),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    Pattern {
        newline_after,
        indent_before,
        indent_delta,
    }
}

/// compiled query that matches all formatting rules
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &super::LANGUAGE,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/format.scm"
        )),
    )
    .expect("query should compile")
});
