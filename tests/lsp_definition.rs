#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use gen_lsp_types::{
    Definition, DefinitionParams, DefinitionRequest, DefinitionResponse,
    DidOpenTextDocumentNotification, DidOpenTextDocumentParams, InitializeParams, Location,
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
