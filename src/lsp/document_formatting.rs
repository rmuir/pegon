use std::sync::atomic::AtomicBool;

use anyhow::{Context as _, Result};
use gen_lsp_types::{DocumentFormattingParams, TextEdit};

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
        // TODO: lets do better
        let full_range = 0..data.len();
        let edit = TextEdit {
            range: client
                .encode_byte_range(&full_range, &doc.line_index)
                .context("valid range")?,
            new_text: str::from_utf8(&buffer)?.into(),
        };
        Ok(Some(vec![edit]))
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
            vec![TextEdit {
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                new_text: indoc! {"
                    @Something
                    public class Foo {
                      int field = 1;
                    }
                "}
                .into()
            }]
        );
    }
}
