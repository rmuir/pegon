use anyhow::{Context as _, Error, bail};
use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use tree_sitter::{
    Node, Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _, Tree,
};

use crate::java_queries::format::captures;
use crate::java_queries::format::predicates::{PREDICATES, PREDICATES_BY_PATTERN};
use crate::java_queries::format::properties::{PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN};
use crate::support::queries::{
    PredicateMatch as _, const_array_from_fn, to_bool_const, to_i8_const,
};

pub struct JavaFormatter {
    /// number of spaces to indent
    indent_size: u8,
    /// sequence to represent newline (e.g. '\n' or '\r\n')
    newline: &'static [u8],
}

impl JavaFormatter {
    /// Creates new formatter with specified settings
    pub const fn new(indent_size: u8, newline: &'static [u8]) -> Self {
        Self {
            indent_size,
            newline,
        }
    }

    /// Formats the document into `buffer`
    ///
    /// # Errors
    ///
    /// This function will return an error if rules are misconfigured.
    pub fn format(
        &self,
        tree: &Tree,
        data: &[u8],
        newdata: &mut Vec<u8>,
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

        #[expect(clippy::indexing_slicing, reason = "checked at compile-time")]
        let mut captures = cursor
            .captures_with_options(
                &QUERY,
                tree.root_node(),
                data,
                QueryCursorOptions::new().progress_callback(&mut cancellation),
            )
            .filter(|(hit, _)| {
                let list = &PREDICATES_BY_PATTERN[hit.pattern_index];
                for index in list.start..list.end {
                    if !PREDICATES[index as usize].matches(hit, data, None) {
                        return false;
                    }
                }
                true
            });

        let mut state = GroupState::new();
        // temporary per "line" buffer
        let mut buffer = Vec::with_capacity(256);

        while let Some((hit, capture_id)) = captures.next() {
            // primary node captures only
            let capture = hit
                .captures()
                .get(*capture_id)
                .context("valid capture_id")?;
            if capture.index != captures::NODE {
                continue;
            }
            let node = capture.node;

            // first pattern wins
            let node_id = node.id();
            if node_id == state.previous_node_id {
                continue;
            }
            state.previous_node_id = node_id;

            let pattern = pattern(hit.pattern_index);
            self.nonterminal(&node, pattern, data, newdata, &mut state, &mut buffer)?;
        }

        // write any final pending newline
        if !buffer.is_empty() {
            if state.pending_newline {
                buffer.extend_from_slice(self.newline);
            }
            newdata.extend_from_slice(&buffer);
        }

        Ok(())
    }

    fn nonterminal(
        &self,
        node: &Node,
        pattern: &Pattern,
        data: &[u8],
        newdata: &mut Vec<u8>,
        state: &mut GroupState,
        buffer: &mut Vec<u8>,
    ) -> Result<(), Error> {
        // let comments be "sticky" / chain on the same line
        if (pattern.comment || state.previous_comment)
            && state.pending_newline
            && node.start_position().row == state.previous_line
        {
            state.pending_newline = false;
            state.pending_space = true;
        }
        state.previous_comment = pattern.comment;

        // adjust indent before node
        state.adjust_indent(pattern.indent_before)?;

        // write newline before, if required
        if !buffer.is_empty() && (pattern.newline_before || state.pending_newline) {
            // preserve existing blank line separators
            if state.pending_newline
                && node
                    .start_position()
                    .row
                    .saturating_sub(state.previous_line)
                    > 1
            {
                buffer.extend_from_slice(self.newline);
            }
            buffer.extend_from_slice(self.newline);
            newdata.extend_from_slice(buffer);
            buffer.clear();
        }

        // write any indent/space before
        if buffer.is_empty() {
            let indent = state
                .current_indent
                .checked_mul(self.indent_size.into())
                .context("no overflow")?;
            buffer.resize(
                buffer
                    .len()
                    .checked_add(indent as usize)
                    .context("no overflow")?,
                b' ',
            );
        } else if pattern.space_before || state.pending_space {
            buffer.resize(buffer.len().checked_add(1).context("no overflow")?, b' ');
        }

        // write the node
        let bytes = data.get(node.byte_range()).context("valid range")?;
        buffer.extend_from_slice(bytes);
        state.pending_space = false;
        state.pending_newline = false;

        // write newline/space after, if required
        if pattern.space_after {
            state.pending_space = true;
        } else if pattern.newline_after {
            state.pending_newline = true;
            state.previous_line = node.end_position().row;
        }

        // adjust indent after node
        state.adjust_indent(pattern.indent_after)?;

        Ok(())
    }
}

/// state per "group"
/// should fit on one line if possible.
struct GroupState {
    /// current indentation LEVEL.
    current_indent: u32,
    /// previous iterated node id, to only handle a node with one pattern
    previous_node_id: usize,
    /// previous line (row)
    previous_line: usize,
    /// if the previous terminal node was a comment node
    previous_comment: bool,
    /// if there's a pending space from after the previous node
    pending_space: bool,
    /// if there's a pending newline from after the previous node
    pending_newline: bool,
}

impl GroupState {
    const fn new() -> Self {
        Self {
            current_indent: 0,
            previous_node_id: usize::MAX,
            previous_line: usize::MAX,
            previous_comment: false,
            pending_space: false,
            pending_newline: false,
        }
    }

    /// adjusts current indentation by the delta
    fn adjust_indent(&mut self, delta: i8) -> Result<(), Error> {
        self.current_indent = self
            .current_indent
            .checked_add_signed(delta.into())
            .context("no overflow")?;
        Ok(())
    }
}

/// single pattern
#[expect(clippy::struct_excessive_bools, reason = "no excuse, just iterating")]
struct Pattern {
    // adjust indentation level after node by delta
    indent_after: i8,
    // adjust indentation level before node by delta
    indent_before: i8,
    // add newline after the node
    newline_after: bool,
    // add newline before the node
    newline_before: bool,
    // add space after the node
    space_after: bool,
    // add space after the node
    space_before: bool,
    // sticks to previous and next node on same line
    comment: bool,
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
    let mut indent_after = 0;
    let mut indent_before = 0;
    let mut newline_after = false;
    let mut newline_before = false;
    let mut space_after = false;
    let mut space_before = false;
    let mut comment = false;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"format.indent.after" => indent_after = to_i8_const(property.1),
            b"format.indent.before" => indent_before = to_i8_const(property.1),
            b"format.newline.after" => newline_after = to_bool_const(property.1),
            b"format.newline.before" => newline_before = to_bool_const(property.1),
            b"format.space.after" => space_after = to_bool_const(property.1),
            b"format.space.before" => space_before = to_bool_const(property.1),
            b"format.comment" => comment = to_bool_const(property.1),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    Pattern {
        indent_after,
        indent_before,
        newline_after,
        newline_before,
        space_after,
        space_before,
        comment,
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
