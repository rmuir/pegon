#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use gen_lsp_types::{
    DidOpenTextDocumentNotification, DidOpenTextDocumentParams, InitializeParams, InlayHint,
    InlayHintLabelPart, InlayHintParams, InlayHintRequest, Label::InlayHintLabelPartList, Location,
    Position, Range, TextDocumentIdentifier, TextDocumentItem, TextEdit, WorkDoneProgressParams,
};
use indoc::indoc;
use lsp_client::LspClient;
use serde_json::json;

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
        .request::<InlayHintRequest>(InlayHintParams {
            text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
            range: Range::new(Position::new(0, 0), Position::new(8, 1)),
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .unwrap();
    assert_eq!(
        result,
        vec![
            InlayHint {
                position: Position::new(6, 9),
                label: InlayHintLabelPartList(vec![InlayHintLabelPart {
                    value: "// finally".into(),
                    tooltip: None,
                    location: Some(Location {
                        uri: "file:///Foo.java".into(),
                        range: Range::new(Position::new(4, 10), Position::new(4, 17)),
                    }),
                    command: None
                }]),
                kind: None,
                text_edits: Some(vec![TextEdit {
                    range: Range::new(Position::new(6, 9), Position::new(6, 9)),
                    new_text: " // finally".into()
                }]),
                tooltip: None,
                padding_left: Some(true),
                padding_right: None,
                data: Some(json!({ "uri": "file:///Foo.java", "version": 0}))
            },
            InlayHint {
                position: Position::new(7, 5),
                label: InlayHintLabelPartList(vec![InlayHintLabelPart {
                    value: "// bar()".into(),
                    tooltip: None,
                    location: Some(Location {
                        uri: "file:///Foo.java".into(),
                        range: Range::new(Position::new(1, 25), Position::new(1, 28))
                    }),
                    command: None
                }]),
                kind: None,
                text_edits: Some(vec![TextEdit {
                    range: Range::new(Position::new(7, 5), Position::new(7, 5)),
                    new_text: " // bar()".into()
                }]),
                tooltip: None,
                padding_left: Some(true),
                padding_right: None,
                data: Some(json!({ "uri": "file:///Foo.java", "version": 0}))
            },
            InlayHint {
                position: Position::new(8, 1),
                label: InlayHintLabelPartList(vec![InlayHintLabelPart {
                    value: "// class foo".into(),
                    tooltip: None,
                    location: Some(Location {
                        uri: "file:///Foo.java".into(),
                        range: Range::new(Position::new(0, 13), Position::new(0, 16))
                    }),
                    command: None
                }]),
                kind: None,
                text_edits: Some(vec![TextEdit {
                    range: Range::new(Position::new(8, 1), Position::new(8, 1)),
                    new_text: " // class foo".into()
                }]),
                tooltip: None,
                padding_left: Some(true),
                padding_right: None,
                data: Some(json!({ "uri": "file:///Foo.java", "version": 0}))
            }
        ]
    );
}
