#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use gen_lsp_types::{
    DidOpenTextDocumentNotification, DidOpenTextDocumentParams, FoldingRange, FoldingRangeKind,
    FoldingRangeParams, FoldingRangeRequest, InitializeParams, PartialResultParams,
    TextDocumentIdentifier, TextDocumentItem, WorkDoneProgressParams,
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
                import foo.bar.One;
                import foo.bar.Two;

                /**
                 * test
                 */
                public class foo {
                    public void bar(int x) {
                    }
                }
            "}
            .into(),
        },
    });
    let result = client
        .request::<FoldingRangeRequest>(FoldingRangeParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///Foo.java".into(),
            },
            partial_result_params: PartialResultParams::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .unwrap();
    assert_eq!(
        result,
        [
            FoldingRange {
                start_line: 0,
                start_character: Some(0),
                end_line: 1,
                end_character: Some(19),
                kind: Some(FoldingRangeKind::Imports),
                collapsed_text: None
            },
            FoldingRange {
                start_line: 4,
                start_character: Some(0),
                end_line: 5,
                end_character: Some(3),
                kind: Some(FoldingRangeKind::Comment),
                collapsed_text: None
            },
            FoldingRange {
                start_line: 7,
                start_character: Some(27),
                end_line: 8,
                end_character: Some(5),
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: None
            }
        ]
    );
}
