use anyhow::{Context as _, Result};
use ls_types::{SelectionRange, SelectionRangeParams};

use crate::lsp::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &SelectionRangeParams,
) -> Result<Option<Vec<SelectionRange>>> {
    let mut result = Vec::with_capacity(params.positions.len());
    for position in &params.positions {
        let linecol = client
            .decode_pos(*position, &doc.line_index)
            .context("valid position")?;
        let offset = doc.line_index.offset(linecol).context("valid offset")?;
        result.push(ranges(client, doc, offset.into())?);
    }
    Ok(Some(result))
}

fn ranges(client: &Client, doc: &Document, offset: usize) -> Result<SelectionRange> {
    let mut node = doc.tree.root_node();
    let descendant = node
        .descendant_for_byte_range(offset, offset)
        .unwrap_or(node);
    let mut selection_range = SelectionRange {
        range: client
            .encode_range(&node.range(), &doc.line_index)
            .context("valid range")?,
        parent: None,
    };
    while let Some(child) = node.child_with_descendant(descendant) {
        node = child;

        let range = client
            .encode_range(&node.range(), &doc.line_index)
            .context("valid range")?;
        if range == selection_range.range {
            continue;
        }

        let new_selection_range = SelectionRange {
            range,
            parent: Some(selection_range.into()),
        };
        selection_range = new_selection_range;
    }
    Ok(selection_range)
}
