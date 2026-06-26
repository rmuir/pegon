use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{InlayHint, InlayHintLabelPart, InlayHintParams, Label, Location, TextEdit};
use tree_sitter::{Query, QueryCursor, StreamingIterator as _};

use crate::support::queries::{capture_id, custom_predicate};

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
            text_edits: Some(vec![TextEdit {
                range: gen_lsp_types::Range::new(position, position),
                new_text,
            }]),
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

static LABEL_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "label"));

static LOCATION_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "location"));

static POSITION_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "position"));
