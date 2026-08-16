use anyhow::{Context as _, Error, bail};
use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _, Tree,
};

use crate::java_queries::format::captures;
use crate::java_queries::format::properties::{PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN};
use crate::support::queries::{const_array_from_fn, to_bool_const, to_i8_const};

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
    if tree.root_node().has_error() {
        bail!("parse error");
    }

    let mut cursor = QueryCursor::new();
    // this callback MUST be a separate let-binding. do *NOT* factor into anonymous closure!
    let mut cancellation = |_: &QueryCursorState| {
        if cancel.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };

    let mut captures = cursor.captures_with_options(
        &QUERY,
        tree.root_node(),
        data,
        QueryCursorOptions::new().progress_callback(&mut cancellation),
    );

    let indent_size = 2; // TODO
    let newline = b"\n"; // TODO

    let mut current_indent: i32 = 0;
    let mut current_line_size = 0;
    let mut previous_node_id = usize::MAX;
    let mut previous_space = false;

    while let Some((hit, capture_id)) = captures.next() {
        // primary node captures only
        let capture = hit.captures.get(*capture_id).context("valid capture_id")?;
        if capture.index != captures::NODE {
            continue;
        }
        let node = capture.node;

        // terminal nodes only
        if node.child_count() > 0 {
            continue;
        }

        // first pattern wins
        let node_id = node.id();
        if node_id == previous_node_id {
            continue;
        }
        previous_node_id = node_id;

        let pattern = pattern(hit.pattern_index);

        current_indent += pattern.indent_delta as i32;

        if current_line_size == 0 {
            let indent = indent_size * current_indent as usize;
            buffer.resize(buffer.len() + indent, b' ');
            current_line_size += indent;
        } else {
            if pattern.space_before && !previous_space {
                buffer.resize(buffer.len() + 1, b' ');
                current_line_size += 1;
            }
        }

        // write the node
        let bytes = data.get(node.byte_range()).context("valid range")?;
        buffer.extend_from_slice(bytes);
        current_line_size += bytes.len();
        previous_space = false;

        // write newline after, if required
        if pattern.space_after {
            buffer.resize(buffer.len() + 1, b' ');
            current_line_size += 1;
            previous_space = true;
        } else if pattern.newline_after != 0 {
            for _ in 0..pattern.newline_after {
                buffer.extend_from_slice(newline);
            }
            current_line_size = 0;
        }
    }
    Ok(())
}

/// single pattern
struct Pattern {
    // adjust indentation level by delta
    indent_delta: i8,
    // add newline after the node
    newline_after: i8,
    // add space after the node
    space_after: bool,
    // add space after the node
    space_before: bool,
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
    let mut indent_delta = 0;
    let mut newline_after = 0;
    let mut space_after = false;
    let mut space_before = false;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"format.indent.delta" => indent_delta = to_i8_const(property.1),
            b"format.newline.after" => newline_after = to_i8_const(property.1),
            b"format.space.after" => space_after = to_bool_const(property.1),
            b"format.space.before" => space_before = to_bool_const(property.1),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    Pattern {
        newline_after,
        indent_delta,
        space_after,
        space_before,
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
