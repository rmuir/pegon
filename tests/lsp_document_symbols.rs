#![expect(clippy::unwrap_used, reason = "tests")]
#![expect(clippy::tests_outside_test_module, reason = "false positive")]

use gen_lsp_types::{
    BaseSymbolInformation, DidOpenTextDocumentNotification, DidOpenTextDocumentParams,
    DocumentSymbolParams, DocumentSymbolRequest, DocumentSymbolResponse, InitializeParams,
    Location, PartialResultParams, Position, Range, SymbolInformation, SymbolKind,
    TextDocumentIdentifier, TextDocumentItem, WorkDoneProgressParams,
};
use indoc::indoc;
use lsp_client::LspClient;

pub mod lsp_client;

/// simple document, flat results
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
                uri: "file:///Foo.java".into(),
            },
            partial_result_params: PartialResultParams::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .unwrap();
    assert_eq!(
        result,
        DocumentSymbolResponse::SymbolInformationList(vec![
            SymbolInformation {
                base_symbol_information: BaseSymbolInformation {
                    name: "foo".into(),
                    kind: SymbolKind::Class,
                    tags: None,
                    container_name: None
                },
                #[expect(deprecated, reason = "unavoidable")]
                deprecated: None,
                location: Location {
                    uri: "file:///Foo.java".into(),
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
            },
            SymbolInformation {
                base_symbol_information: BaseSymbolInformation {
                    name: "bar(int)".into(),
                    kind: SymbolKind::Method,
                    tags: None,
                    container_name: Some("foo".into())
                },
                #[expect(deprecated, reason = "unavoidable")]
                deprecated: None,
                location: Location {
                    uri: "file:///Foo.java".into(),
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
            }
        ])
    );
}
