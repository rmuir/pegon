use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};

use std::sync::{Arc, LazyLock};

use anyhow::{Context as _, Result};
use gen_lsp_types::{Definition, DefinitionParams, DefinitionResponse, Location, LocationLink};
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _,
};

use crate::support::queries::capture_id;

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &DefinitionParams,
    cancel_token: &Arc<AtomicBool>,
) -> Result<Option<DefinitionResponse>> {
    let position = params.text_document_position_params.position;
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
        doc.tree.root_node(),
        bytes,
        QueryCursorOptions::new().progress_callback(&mut cancellation),
    );
    let mut best_match = 0;
    while let Some(hit) = matches.next() {
        // ensure last pattern-wins
        if hit.pattern_index < best_match {
            continue;
        }
        // check if it is a true match, we must be inside the selection capture
        let selection = hit
            .nodes_for_capture_index(*SELECTION_CAPTURE)
            .next()
            .context("should have selection capture")?;
        if source_position < selection.range().start_byte
            || source_position > selection.range().end_byte
        {
            continue;
        }

        let target = hit
            .nodes_for_capture_index(*RANGE_CAPTURE)
            .next()
            .context("should have range capture")?;
        let target_selection_range = client
            .encode_range(&selection.range(), &doc.line_index)
            .context("valid range")?;
        let target_range = client
            .encode_range(&target.range(), &doc.line_index)
            .context("valid range")?;
        result = Some(LocationLink {
            target_range,
            origin_selection_range: Some(target_selection_range),
            target_uri: params
                .text_document_position_params
                .text_document
                .uri
                .clone(),
            target_selection_range,
        });
        best_match = hit.pattern_index;
    }
    result.map_or_else(
        || Ok(None),
        |result| {
            if client.supports_links() {
                Ok(Some(DefinitionResponse::DefinitionLinkList(vec![result])))
            } else {
                Ok(Some(DefinitionResponse::Definition(Definition::Location(
                    Location::new(result.target_uri, result.target_range),
                ))))
            }
        },
    )
}

/// compiled query that matches all folding patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::language(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/definitions.scm"
        )),
    )
    .expect("query should compile")
});

static RANGE_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "range"));

static SELECTION_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "selection"));

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        Definition, DefinitionParams, DefinitionRequest, DefinitionResponse,
        DidOpenTextDocumentNotification, DidOpenTextDocumentParams, InitializeParams, Location,
        PartialResultParams, Position, Range, TextDocumentIdentifier, TextDocumentItem,
        TextDocumentPositionParams, WorkDoneProgressParams,
    };
    use indoc::indoc;

    use crate::lsp::test_client::TestClient;

    #[test]
    fn flat() {
        let client = TestClient::new(InitializeParams::default());
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                public class foo {
                    public abstract void bar(int x) {
                    }
                }
            "}
                .into(),
            },
        });
        let result = client
            .request::<DefinitionRequest>(DefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
                    position: Position::new(1, 12),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .unwrap();
        assert_eq!(
            result,
            DefinitionResponse::Definition(Definition::Location(Location {
                uri: "file:///Foo.java".into(),
                range: Range::new(Position::new(1, 11), Position::new(1, 19)),
            })),
        );
    }
}
