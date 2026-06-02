#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use gen_lsp_types::{
    Contents, DidOpenTextDocumentNotification, DidOpenTextDocumentParams, Hover, HoverParams,
    HoverRequest, InitializeParams, MarkupContent, MarkupKind, Position, Range,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, WorkDoneProgressParams,
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
            text: indoc! {r#"
                public class foo {
                    public abstract void bar(int x) {
                    }
                }
            "#}
            .into(),
        },
    });
    let result = client
        .request::<HoverRequest>(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///Foo.java".into(),
                },
                position: Position {
                    line: 1,
                    character: 12,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .unwrap();
    assert_eq!(
        result,
        Hover {
            contents: Contents::MarkupContent(MarkupContent {
                kind: MarkupKind::PlainText,
                value: "kind: https://docs.oracle.com/javase/specs/jls/se26/html/jls-8.html#jls-8.4.3.1\nThis method isn't concrete: a subclass must implement it.\n".into()
            }),
            range: Some(Range {
                start: Position {
                    line: 1,
                    character: 11
                },
                end: Position {
                    line: 1,
                    character: 19,
                },
            })
        }
    );
}
