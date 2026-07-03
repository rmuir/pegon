use core::ops::{ControlFlow, Range};
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

use anyhow::{Context as _, Result};
use gen_lsp_types::{
    InlayHint, InlayHintLabelPart, InlayHintParams, Label, Location, TextEdit, Uri,
};
use serde::{Deserialize, Serialize};
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _,
};

use crate::support::queries::{capture_id, custom_predicate};

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &InlayHintParams,
    cancel_token: &Arc<AtomicBool>,
) -> Result<Vec<InlayHint>> {
    let range = client
        .decode_range(&params.range, &doc.line_index)
        .context("valid range")?;

    let can_resolve = client.supports_inlay_hint_resolve_edit()
        && (client.supports_inlay_hint_resolve_label_location()
            || client.supports_inlay_hint_resolve_neovim_location());

    hints(
        client,
        doc,
        &params.text_document.uri,
        range.start_byte..range.end_byte,
        !can_resolve,
        cancel_token,
    )
}

pub fn resolve(
    client: &Client,
    doc: &Document,
    params: &InlayHint,
    data: &CustomData,
    cancel_token: &Arc<AtomicBool>,
) -> Result<InlayHint> {
    let position = client
        .decode_pos(params.position, &doc.line_index)
        .context("valid position")?;
    let offset: usize = doc
        .line_index
        .offset(position)
        .context("valid offset")?
        .into();
    let mut hints = hints(
        client,
        doc,
        &data.uri,
        offset.saturating_sub(1)..offset,
        true,
        cancel_token,
    )?;
    hints.pop().context("matching inlay hint")
}

#[derive(Serialize, Deserialize)]
pub struct CustomData {
    pub uri: Uri,
    pub version: i32,
}

pub fn hints(
    client: &Client,
    doc: &Document,
    uri: &Uri,
    range: Range<usize>,
    populate: bool,
    cancel_token: &Arc<AtomicBool>,
) -> Result<Vec<InlayHint>> {
    let data = doc.text.as_bytes();
    let mut result = Vec::with_capacity(64);
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range.start..range.end);

    // this callback MUST be a separate let-binding. do *NOT* factor into anonymous closure!
    let mut cancellation = |_: &QueryCursorState| {
        if cancel_token.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };

    let mut matches = cursor
        .matches_with_options(
            &QUERY,
            doc.tree.root_node(),
            data,
            QueryCursorOptions::new().progress_callback(&mut cancellation),
        )
        .filter(|hit| {
            for predicate in QUERY.general_predicates(hit.pattern_index) {
                if !custom_predicate(hit, data, &predicate.operator, &predicate.args) {
                    return false;
                }
            }
            true
        });

    let custom_data = serde_json::to_value(CustomData {
        uri: uri.clone(),
        version: doc.version,
    })?;

    while let Some(hit) = matches.next() {
        let node = hit
            .nodes_for_capture_index(*POSITION_CAPTURE)
            .next()
            .context("position capture should exist")?;
        let node_range = node.byte_range();
        if node_range.end < range.start || node_range.start > range.end {
            continue;
        }
        let pattern = pattern(hit.pattern_index);
        let position = client
            .encode_range(&node.range(), &doc.line_index)
            .context("valid offset")?
            .end;

        // raw captured text from pattern/nodes with only internal server-side padding
        let mut value = String::with_capacity(20);
        if let Some(prefix) = pattern.prefix {
            value.push_str(prefix);
        }
        for part in hit.nodes_for_capture_index(*LABEL_CAPTURE) {
            if !value.is_empty() && pattern.pad_medial {
                value.push(' ');
            }
            value.push_str(part.utf8_text(data)?);
        }
        if let Some(suffix) = pattern.suffix {
            value.push_str(suffix);
        }

        // compute the text edit, which should not be truncated.
        let mut new_text = String::with_capacity(value.len().saturating_add(2));
        if pattern.pad_left {
            new_text.push(' ');
        }
        new_text.push_str(value.as_str());
        if pattern.pad_right {
            new_text.push(' ');
        }

        // compute the display form, which should be cleaned up.
        // truncate at newlines
        if let Some(newline) = value.find('\n') {
            value.truncate(newline);
            value.push('\u{2026}');
        }

        // truncate at runs of spaces
        if let Some(spacerun) = value.find("  ") {
            value.truncate(spacerun);
            value.push('\u{2026}');
        }

        // if still too long, truncate with ellipsis
        if value.len() > 60 {
            value.truncate(59);
            value.push('\u{2026}');
        }

        let location = if populate
            && let Some(location) = hit.nodes_for_capture_index(*LOCATION_CAPTURE).next()
        {
            Some(Location {
                uri: uri.clone(),
                range: client
                    .encode_range(&location.range(), &doc.line_index)
                    .context("valid offset")?,
            })
        } else {
            None
        };
        let label = Label::InlayHintLabelPartList(vec![InlayHintLabelPart {
            value,
            tooltip: None,
            location,
            command: None,
        }]);
        result.push(InlayHint {
            position,
            label,
            kind: None,
            text_edits: populate.then_some(vec![TextEdit {
                range: gen_lsp_types::Range::new(position, position),
                new_text,
            }]),
            tooltip: None,
            padding_left: pattern.pad_left.then_some(true),
            padding_right: pattern.pad_right.then_some(true),
            data: Some(custom_data.clone()),
        });
    }
    Ok(result)
}

/// single compiled pattern
struct Pattern {
    /// prefix prepended to the start of the hint
    prefix: Option<&'static str>,
    /// suffix appended to the end of hint
    suffix: Option<&'static str>,
    /// client-side padding before the hint
    pad_left: bool,
    /// server-side padding between captures composing the hint
    pad_medial: bool,
    /// client-side padding before the hint
    pad_right: bool,
}

/// Look up rule by pattern index
#[must_use]
fn pattern(index: usize) -> &'static Pattern {
    PATTERNS.get(index).expect("pattern should exist")
}

/// array of rules indexed by patterns of `QUERY`
static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    let count = QUERY.pattern_count();
    let mut patterns = Vec::with_capacity(count);
    for index in 0..count {
        let mut prefix = None;
        let mut suffix = None;
        let mut pad_left = false;
        let mut pad_medial = false;
        let mut pad_right = false;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "hint.prefix" => prefix = value,
                "hint.suffix" => suffix = value,
                "hint.pad.left" => {
                    pad_left = value.expect("bool value").parse().expect("bool value");
                }
                "hint.pad.medial" => {
                    pad_medial = value.expect("bool value").parse().expect("bool value");
                }
                "hint.pad.right" => {
                    pad_right = value.expect("bool value").parse().expect("bool value");
                }
                _ => panic!("{key}: unknown metadata key"),
            }
        }
        patterns.push(Pattern {
            prefix,
            suffix,
            pad_left,
            pad_medial,
            pad_right,
        });
    }
    patterns
});

/// compiled query that matches all folding patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::language(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/hints.scm"
        )),
    )
    .expect("query should compile")
});

static LABEL_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "label"));

static LOCATION_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "location"));

static POSITION_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "position"));

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        DidOpenTextDocumentNotification, DidOpenTextDocumentParams, InitializeParams, InlayHint,
        InlayHintLabelPart, InlayHintParams, InlayHintRequest, Label::InlayHintLabelPartList,
        Location, Position, Range, TextDocumentIdentifier, TextDocumentItem, TextEdit,
        WorkDoneProgressParams,
    };
    use indoc::indoc;
    use serde_json::json;

    use crate::lsp::test_client::TestClient;

    /// simple document
    #[test]
    fn basic() {
        let client = TestClient::new(InitializeParams::default());
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
}
