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
