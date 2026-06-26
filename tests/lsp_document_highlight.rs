#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use gen_lsp_types::{
    DidOpenTextDocumentNotification, DidOpenTextDocumentParams, DocumentHighlight,
    DocumentHighlightKind, DocumentHighlightParams, DocumentHighlightRequest, InitializeParams,
    PartialResultParams, Position, Range, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, WorkDoneProgressParams,
};
use indoc::indoc;
use lsp_client::LspClient;

pub mod lsp_client;

/// simple document
#[test]
fn flat() {
    let client = LspClient::new(InitializeParams::default());
    client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: "file:///Foo.java".into(),
            language_id: "java".into(),
            version: 0,
            text: indoc! {"
                public class foo {
                    public abstract void bar(int x) {
                        try {
                            baz();
                        } finally {
                            System.exit(0);
                        }
                    }
                }
            "}
            .into(),
        },
    });
    let result = client
        .request::<DocumentHighlightRequest>(DocumentHighlightParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
                position: Position::new(2, 9),
            },
            partial_result_params: PartialResultParams::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .unwrap();
    assert_eq!(
        result,
        vec![
            // try
            DocumentHighlight {
                kind: Some(DocumentHighlightKind::Read),
                range: Range::new(Position::new(2, 8), Position::new(2, 11))
            },
            // finally
            DocumentHighlight {
                kind: Some(DocumentHighlightKind::Read),
                range: Range::new(Position::new(4, 10), Position::new(4, 17))
            }
        ]
    );
}
