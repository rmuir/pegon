#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use core::str::FromStr as _;

use indoc::indoc;
use ls_types::{
    DidOpenTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse, InitializeParams,
    Location, PartialResultParams, Position, Range, SymbolInformation, SymbolKind,
    TextDocumentIdentifier, TextDocumentItem, Uri, WorkDoneProgressParams,
    notification::DidOpenTextDocument, request::DocumentSymbolRequest,
};
use lsp_client::LspClient;

pub mod lsp_client;

/// simple document, flat results
#[test]
fn flat() {
    let client = LspClient::new(InitializeParams::default());
    client.notify::<DidOpenTextDocument>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: Uri::from_str("file:///Foo.java").unwrap(),
            language_id: "java".into(),
            version: 0,
            text: indoc! {r#"
                public class foo {
                    public void bar(int x) {
                    }
                }
            "#}
            .into(),
        },
    });
    let result = client
        .request::<DocumentSymbolRequest>(DocumentSymbolParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str("file:///Foo.java").unwrap(),
            },
            partial_result_params: PartialResultParams::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .unwrap();
    assert_eq!(
        result,
        DocumentSymbolResponse::Flat(vec![
            SymbolInformation {
                name: "foo".into(),
                kind: SymbolKind::CLASS,
                tags: None,
                #[expect(deprecated, reason = "unavoidable")]
                deprecated: None,
                location: Location {
                    uri: Uri::from_str("file:///Foo.java").unwrap(),
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0
                        },
                        end: Position {
                            line: 3,
                            character: 1
                        }
                    }
                },
                container_name: None
            },
            SymbolInformation {
                name: "bar(int)".into(),
                kind: SymbolKind::METHOD,
                tags: None,
                #[expect(deprecated, reason = "unavoidable")]
                deprecated: None,
                location: Location {
                    uri: Uri::from_str("file:///Foo.java").unwrap(),
                    range: Range {
                        start: Position {
                            line: 1,
                            character: 4
                        },
                        end: Position {
                            line: 2,
                            character: 5
                        }
                    }
                },
                container_name: Some("foo".into())
            }
        ])
    );
}
