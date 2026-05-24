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
            text: indoc! {r#"
                public class foo {
                    public void bar(int x) {
                        int y;
                    }
                }
            "#}
            .into(),
        },
    });
    let result = client
        .request::<SelectionRangeRequest>(SelectionRangeParams {
            text_document: TextDocumentIdentifier {
                uri: "file:///Foo.java".into(),
            },
            positions: vec![Position {
                line: 2,
                character: 12,
            }],
            partial_result_params: PartialResultParams::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .unwrap();
    assert_eq!(
        result,
        [SelectionRange {
            // y
            range: Range {
                start: Position {
                    line: 2,
                    character: 12
                },
                end: Position {
                    line: 2,
                    character: 13
                }
            },
            // int y;
            parent: Some(Box::new(SelectionRange {
                range: Range {
                    start: Position {
                        line: 2,
                        character: 8
                    },
                    end: Position {
                        line: 2,
                        character: 14
                    }
                },
                // {}
                parent: Some(Box::new(SelectionRange {
                    range: Range {
                        start: Position {
                            line: 1,
                            character: 27
                        },
                        end: Position {
                            line: 3,
                            character: 5
                        }
                    },
                    // public void bar(int x) {}
                    parent: Some(Box::new(SelectionRange {
                        range: Range {
                            start: Position {
                                line: 1,
                                character: 4
                            },
                            end: Position {
                                line: 3,
                                character: 5
                            }
                        },
                        // { public void bar(int x) {} }
                        parent: Some(Box::new(SelectionRange {
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 17
                                },
                                end: Position {
                                    line: 4,
                                    character: 1
                                }
                            },
                            // public class Foo {}
                            parent: Some(Box::new(SelectionRange {
                                range: Range {
                                    start: Position {
                                        line: 0,
                                        character: 0
                                    },
                                    end: Position {
                                        line: 4,
                                        character: 1
                                    }
                                },
                                // entire document
                                parent: Some(Box::new(SelectionRange {
                                    range: Range {
                                        start: Position {
                                            line: 0,
                                            character: 0
                                        },
                                        end: Position {
                                            line: 5,
                                            character: 0
                                        }
                                    },
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
