use anyhow::{Context as _, Result};
use gen_lsp_types::{SelectionRange, SelectionRangeParams};

use super::{Client, server::Document};

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

#[cfg(test)]
mod tests {
    use crate::lsp::test_client::TestClient;
    use gen_lsp_types::{
        DidOpenTextDocumentNotification, DidOpenTextDocumentParams, InitializeParams,
        PartialResultParams, Position, Range, SelectionRange, SelectionRangeParams,
        SelectionRangeRequest, TextDocumentIdentifier, TextDocumentItem, WorkDoneProgressParams,
    };
    use indoc::indoc;

    /// simple document
    #[test]
    fn simple() {
        let client = TestClient::new(InitializeParams::default());
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                public class foo {
                    public void bar(int x) {
                        int y;
                    }
                }
            "}
                .into(),
            },
        });
        let result = client
            .request::<SelectionRangeRequest>(SelectionRangeParams {
                text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
                positions: vec![Position::new(2, 12)],
                partial_result_params: PartialResultParams::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        assert_eq!(
            result,
            [SelectionRange {
                // y
                range: Range::new(Position::new(2, 12), Position::new(2, 13)),
                // int y;
                parent: Some(Box::new(SelectionRange {
                    range: Range::new(Position::new(2, 8), Position::new(2, 14)),
                    // {}
                    parent: Some(Box::new(SelectionRange {
                        range: Range::new(Position::new(1, 27), Position::new(3, 5)),
                        // public void bar(int x) {}
                        parent: Some(Box::new(SelectionRange {
                            range: Range::new(Position::new(1, 4), Position::new(3, 5)),
                            // { public void bar(int x) {} }
                            parent: Some(Box::new(SelectionRange {
                                range: Range::new(Position::new(0, 17), Position::new(4, 1)),
                                // public class Foo {}
                                parent: Some(Box::new(SelectionRange {
                                    range: Range::new(Position::new(0, 0), Position::new(4, 1)),
                                    // entire document
                                    parent: Some(Box::new(SelectionRange {
                                        range: Range::new(Position::new(0, 0), Position::new(5, 0)),
                                        parent: None
                                    }))
                                }))
                            }))
                        }))
                    }))
                }))
            }]
        );
    }
}
