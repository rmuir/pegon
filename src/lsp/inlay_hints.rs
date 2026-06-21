use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{InlayHint, InlayHintLabelPart, InlayHintParams, Label, Location};
use tree_sitter::{Query, QueryCursor, StreamingIterator as _};

use crate::support::queries::custom_predicate;

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &InlayHintParams,
) -> Result<Vec<InlayHint>> {
    let data = doc.text.as_bytes();
    let range = client
        .decode_range(&params.range, &doc.line_index)
        .context("valid range")?;
    let mut result = Vec::with_capacity(3);
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range.start_byte..range.end_byte);
    let mut matches = cursor
        .matches(&QUERY, doc.tree.root_node(), data)
        .filter(|hit| {
            for predicate in QUERY.general_predicates(hit.pattern_index) {
                if !custom_predicate(hit, data, &predicate.operator, &predicate.args) {
                    return false;
                }
            }
            true
        });

    while let Some(hit) = matches.next() {
        let node = hit
            .nodes_for_capture_index(*POSITION_CAPTURE)
            .next()
            .context("position capture should exist")?;
        let pattern = pattern(hit.pattern_index);
        let position = client
            .encode_range(&node.range(), &doc.line_index)
            .context("valid offset")?
            .end;
        let mut value = String::new();

        for part in hit.nodes_for_capture_index(*LABEL_CAPTURE) {
            let bytes = part.utf8_text(data)?;
            if !value.is_empty() && pattern.pad_medial {
                value.push(' ');
            }
            if bytes.contains('\n') || bytes.contains("  ") {
                let words: Vec<_> = bytes.split_whitespace().collect();
                value.push_str(words.join(" ").as_str());
            } else {
                value.push_str(bytes);
            }
        }
        if value.len() > 60 {
            value.truncate(59);
            value.push('\u{2026}');
        }

        let location = if let Some(location) = hit.nodes_for_capture_index(*LOCATION_CAPTURE).next()
        {
            Some(Location {
                uri: params.text_document.uri.clone(),
                range: client
                    .encode_range(&location.range(), &doc.line_index)
                    .context("valid offset")?,
            })
        } else {
            None
        };
        let mut parts = Vec::with_capacity(3);
        pattern.prefix.is_some().then(|| {
            parts.push(InlayHintLabelPart {
                value: pattern.prefix.expect("Some").into(),
                location: None,
                tooltip: None,
                command: None,
            });
        });
        parts.push(InlayHintLabelPart {
            value,
            location,
            tooltip: None,
            command: None,
        });
        pattern.suffix.is_some().then(|| {
            parts.push(InlayHintLabelPart {
                value: pattern.suffix.expect("Some").into(),
                location: None,
                tooltip: None,
                command: None,
            });
        });
        let label = Label::InlayHintLabelPartList(parts);
        result.push(InlayHint {
            position,
            label,
            kind: None,
            text_edits: None,
            tooltip: None,
            padding_left: pattern.pad_left.then_some(true),
            padding_right: pattern.pad_right.then_some(true),
            data: None,
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
        &crate::support::LANGUAGE.into(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/hints.scm"
        )),
    )
    .expect("query should compile")
});

static LABEL_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("label")
        .expect("label capture should exist")
});

static LOCATION_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("location")
        .expect("location capture should exist")
});

static POSITION_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("position")
        .expect("position capture should exist")
});
