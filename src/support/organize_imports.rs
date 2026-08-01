use anyhow::{Context as _, Result, anyhow};
use bstr::ByteSlice as _;
use std::ops::Range;
use std::str::from_utf8;
use std::sync::LazyLock;
use tree_sitter::{Query, QueryCursor, StreamingIterator as _, Tree};

use crate::support::fix::Edit;
use crate::support::queries::{KindSet, capture_id};

struct Import {
    /// type of the import, primary sort key
    pattern: usize,
    /// text of the import, secondary sort key
    text: Range<usize>,
    /// range of the import, including attached comments
    range: Range<usize>,
}

#[expect(clippy::indexing_slicing, reason = "for another day")]
#[expect(clippy::arithmetic_side_effects, reason = "for another day")]
#[expect(clippy::map_err_ignore, reason = "because the error sucks!")]
pub fn organize(tree: &Tree, data: &[u8]) -> Result<Option<Edit>> {
    let mut imports = imports(tree, data)?;
    // no imports or only one: already sorted
    if imports.len() < 2 {
        return Ok(None);
    }
    // create range of entire edit
    let start_offset = imports.first().context("exists")?.range.start;
    let end_offset = imports.last().context("exists")?.range.end;
    // sort the imports
    imports.sort_unstable_by(|left, right| {
        left.pattern
            .cmp(&right.pattern)
            .then_with(|| data[left.text.clone()].cmp(&data[right.text.clone()]))
    });
    // fold into an accumulated string. insert a newline between import sections.
    let replacement = imports.iter().enumerate().try_fold(
        String::with_capacity(end_offset - start_offset),
        |mut buffer, (index, import)| -> Result<_> {
            if index > 0 && imports[index - 1].pattern != import.pattern {
                buffer.push('\n');
            }
            buffer.push_str(
                from_utf8(&data[import.range.clone()]).map_err(|_| anyhow!("invalid utf-8"))?,
            );
            Ok(buffer)
        },
    )?;
    Ok(Some(Edit {
        range: start_offset..end_offset,
        replacement,
    }))
}

/// returns list of import ranges
fn imports(tree: &Tree, data: &[u8]) -> Result<Vec<Import>> {
    let mut imports = Vec::with_capacity(16);
    let mut cursor = QueryCursor::new();

    let mut matches = cursor.matches(&QUERY, tree.root_node(), data);
    let mut last_end_offset = 0;
    while let Some(hit) = matches.next() {
        let node = hit
            .nodes_for_capture_index(*NODE_CAPTURE)
            .next()
            .context("node capture should exist")?;

        let text = hit
            .nodes_for_capture_index(*TEXT_CAPTURE)
            .next()
            .context("text capture should exist")?;

        let mut start_offset = node.byte_range().start;
        let mut end_offset = node.byte_range().end;

        // extend nodes start to include preceding comment
        // TODO: loop here to preserve multi-line comments?
        if let Some(sibling) = node.prev_named_sibling()
            && sibling.byte_range().start >= last_end_offset
            && COMMENT_KINDS.contains(sibling.kind_id())
        {
            start_offset = sibling.byte_range().start;
        }

        // extend node's end until the end of the line
        if let Some(newline) = data.get(end_offset..).context("valid range")?.find(b"\n") {
            end_offset = end_offset
                .checked_add(newline.checked_add(1).context("valid offset")?)
                .context("valid offset")?;
        }

        // record where this node ended
        last_end_offset = end_offset;

        imports.push(Import {
            pattern: hit.pattern_index,
            text: text.byte_range(),
            range: start_offset..end_offset,
        });
    }
    Ok(imports)
}

/// compiled query that matches all import patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::language(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/imports.scm"
        )),
    )
    .expect("query should compile")
});

/// comment node kinds
static COMMENT_KINDS: LazyLock<KindSet> =
    LazyLock::new(|| KindSet::new(&["line_comment", "block_comment"]));

/// index of the `@node` capture
static NODE_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "node"));

/// index of the `@text` capture
static TEXT_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "text"));
