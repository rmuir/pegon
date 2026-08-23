use std::sync::atomic::AtomicBool;

use anyhow::{Context as _, Result};
use gen_lsp_types::{DocumentFormattingParams, TextEdit};

use crate::support::fix::Edit;

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    _params: &DocumentFormattingParams,
    cancel: &AtomicBool,
) -> Result<Option<Vec<TextEdit>>> {
    let data = doc.text.as_bytes();
    let mut buffer = Vec::with_capacity(doc.text.len());
    crate::support::formatting::format(&doc.tree, data, &mut buffer, cancel)?;
    if buffer == data {
        Ok(None)
    } else {
        let edits = Edit::diff(data, &buffer)?;
        let textedits = edits
            .into_iter()
            .map(|edit| {
                Ok(TextEdit {
                    range: client
                        .encode_byte_range(&edit.range, &doc.line_index)
                        .context("valid range")?,
                    new_text: edit.replacement,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Some(textedits))
    }
}

#[cfg(test)]
mod tests {
    use crate::lsp::test_client::TestClient;
    use gen_lsp_types::{
        DidOpenTextDocumentNotification, DidOpenTextDocumentParams, DocumentFormattingParams,
        DocumentFormattingRequest, FormattingOptions, Position, Range, TextDocumentIdentifier,
        TextDocumentItem, TextEdit, WorkDoneProgressParams,
    };
    use indoc::indoc;

    /// simple document
    #[test]
    fn simple() {
        let client = TestClient::default();
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                @Something public class Foo { int field=1; }
            "}
                .into(),
            },
        });
        let result = client
            .request::<DocumentFormattingRequest>(DocumentFormattingParams {
                text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
                options: FormattingOptions {
                    tab_size: 2,
                    insert_spaces: true,
                    ..FormattingOptions::default()
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        assert_eq!(
            result,
            vec![
                // replace space with newline before Foo's }
                TextEdit {
                    range: Range::new(Position::new(0, 42), Position::new(0, 43)),
                    new_text: "\n".into(),
                },
                // insert space after = operator
                TextEdit {
                    range: Range::new(Position::new(0, 40), Position::new(0, 40)),
                    new_text: " ".into(),
                },
                // insert space before = operator
                TextEdit {
                    range: Range::new(Position::new(0, 39), Position::new(0, 39)),
                    new_text: " ".into(),
                },
                // insert newline and additional space after Foo's {
                TextEdit {
                    range: Range::new(Position::new(0, 29), Position::new(0, 29)),
                    new_text: "\n ".into(),
                },
                // replace space after @Something with newline
                TextEdit {
                    range: Range::new(Position::new(0, 10), Position::new(0, 11)),
                    new_text: "\n".into()
                },
            ]
        );
    }
}
