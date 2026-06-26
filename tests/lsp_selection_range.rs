#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use gen_lsp_types::{
    DidOpenTextDocumentNotification, DidOpenTextDocumentParams, InitializeParams,
    PartialResultParams, Position, Range, SelectionRange, SelectionRangeParams,
    SelectionRangeRequest, TextDocumentIdentifier, TextDocumentItem, WorkDoneProgressParams,
};
use indoc::indoc;
use lsp_client::LspClient;

pub mod lsp_client;

/// simple document
#[test]
fn simple() {
    let client = LspClient::new(InitializeParams::default());
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
