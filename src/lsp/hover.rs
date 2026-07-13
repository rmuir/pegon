use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};

use std::sync::{Arc, LazyLock};

use anyhow::{Context as _, Result};
use gen_lsp_types::{Contents, Hover, MarkupContent, MarkupKind, Position};
use indoc::formatdoc;
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _,
};

use crate::support::queries::capture_id;

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    position: Position,
    cancel_token: &Arc<AtomicBool>,
) -> Result<Option<Hover>> {
    let markdown = client.prefers_hover_markdown();
    let bytes = doc.text.as_bytes();
    // TODO: do this lazily
    let scopes = super::analysis::scopes(&doc.tree, bytes, cancel_token)?;
    let mut result = None;
    let mut cursor = QueryCursor::new();
    let linecol = client
        .decode_pos(position, &doc.line_index)
        .context("should decode")?;
    let source_position: usize = doc
        .line_index
        .offset(linecol)
        .context("should be valid offset")?
        .into();
    cursor.set_byte_range(source_position..source_position.checked_add(1).context("no overflow")?);

    // this callback MUST be a separate let-binding. do *NOT* factor into anonymous closure!
    let mut cancellation = |_: &QueryCursorState| {
        if cancel_token.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };

    let mut matches = cursor.matches_with_options(
        &QUERY,
        doc.tree.root_node(),
        bytes,
        QueryCursorOptions::new().progress_callback(&mut cancellation),
    );
    let mut best_match = 0;
    while let Some(hit) = matches.next() {
        // ensure last pattern-wins
        if hit.pattern_index < best_match {
            continue;
        }
        // check if it is a true match, we must be inside the range capture
        let node = hit
            .nodes_for_capture_index(*RANGE_CAPTURE)
            .next()
            .context("should have range capture")?;
        let node_range = node.range();
        if source_position < node_range.start_byte || source_position > node_range.end_byte {
            continue;
        }

        let text = node.utf8_text(bytes)?;
        let pattern = pattern(hit.pattern_index);

        let value = match pattern {
            Pattern::Bail => None,
            Pattern::Reference => {
                let mut reference: Option<String> = None;
                if let Some(stack) = scopes.get(text) {
                    for scope in stack.iter().rev() {
                        if scope.contains(node_range.start_byte) {
                            let java_type = if let Some(java_type) = scope.java_type {
                                str::from_utf8(
                                    bytes
                                        .get(java_type.start_byte..java_type.end_byte)
                                        .context("valid slice")?,
                                )?
                            } else {
                                "var"
                            };
                            let kind = super::analysis::pattern(scope.pattern_id).kind;
                            reference = if markdown {
                                Some(formatdoc! {"
                                    ```java
                                    {java_type} {text}
                                    ```
                                    ---
                                    `{kind}`
                                "})
                            } else {
                                Some(formatdoc! {"
                                    {java_type} {text}
                                    ---
                                    {kind}
                                "})
                            };
                            break;
                        }
                    }
                }

                reference
            }
            Pattern::Spec(SpecPattern {
                summary,
                description,
                reference,
            }) => {
                let (spec_chapter, _) = reference
                    .split_once('.')
                    .context("should be valid JLS spec ref")?;
                let spec_url = format!("{SPEC_PREFIX}/jls-{spec_chapter}.html#jls-{reference}");
                if markdown {
                    Some(formatdoc! {"
                        ```java
                        {text}
                        ```
                        ---
                        `{summary}`

                        {description}

                        [JLS §{reference}]({spec_url})
                    "})
                } else {
                    Some(formatdoc! {"
                        {text}
                        ---
                        {summary}

                        {description}

                        JLS §{reference}: {spec_url}
                    "})
                }
            }
        };
        let range = client
            .encode_range(&node.range(), &doc.line_index)
            .context("valid range")?;
        result = value.map(|value| Hover {
            contents: Contents::MarkupContent(MarkupContent {
                kind: if markdown {
                    MarkupKind::Markdown
                } else {
                    MarkupKind::PlainText
                },
                value,
            }),
            range: Some(range),
        });
        best_match = hit.pattern_index;
    }
    Ok(result)
}

/// single compiled pattern
enum Pattern {
    Spec(SpecPattern),
    Reference,
    Bail,
}

/// when linking to the specification, use this URL as the base
const SPEC_PREFIX: &str = "https://docs.oracle.com/javase/specs/jls/se26/html";

struct SpecPattern {
    /// summary
    summary: String,
    /// description
    description: String,
    /// reference
    reference: String,
}

/// Look up rule by pattern index
#[must_use]
fn pattern(index: usize) -> &'static Pattern {
    PATTERNS.get(index).expect("pattern should exist")
}

/// compiled query that matches all folding patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::language(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/hover.scm"
        )),
    )
    .expect("query should compile")
});

/// array of rules indexed by patterns of `QUERY`
static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    let count = QUERY.pattern_count();
    let mut patterns = Vec::with_capacity(count);
    for index in 0..count {
        let mut kind: Option<&str> = None;
        let mut summary: Option<&str> = None;
        let mut reference: Option<&str> = None;
        let mut description: Option<&str> = None;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "hover.spec.description" => description = value,
                "hover.spec.summary" => summary = value,
                "hover.spec.reference" => reference = value,
                "hover.kind" => kind = value,
                _ => panic!("{key}: unknown metadata key"),
            }
        }
        patterns.push(match kind {
            Some("reference") => Pattern::Reference,
            Some("bail") => Pattern::Bail,
            Some("spec") => Pattern::Spec(SpecPattern {
                summary: summary.expect("should exist").into(),
                reference: reference.expect("should exist").into(),
                description: description.expect("should exist").into(),
            }),
            _ => panic!("{kind:?}: unknown metadata kind"),
        });
    }
    patterns
});

static RANGE_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "range"));

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        Contents, DidOpenTextDocumentNotification, DidOpenTextDocumentParams, Hover, HoverParams,
        HoverRequest, InitializeParams, MarkupContent, MarkupKind, Position, Range,
        TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams,
        WorkDoneProgressParams,
    };
    use indoc::indoc;

    use crate::lsp::test_client::TestClient;

    /// simple document
    #[test]
    fn simple() {
        let client = TestClient::new(InitializeParams::default());
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
            .request::<HoverRequest>(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
                    position: Position::new(1, 12),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        assert_eq!(
        result,
        Hover {
            contents: Contents::MarkupContent(MarkupContent {
                kind: MarkupKind::PlainText,
                value: indoc! {"
                    abstract
                    ---
                    abstract method modifier

                    This method isn't concrete: a subclass must implement it.

                    JLS \u{a7}8.4.3.1: https://docs.oracle.com/javase/specs/jls/se26/html/jls-8.html#jls-8.4.3.1
                "}
                .into(),
            }),
            range: Some(Range::new(Position::new(1, 11), Position::new(1, 19)))
        }
    );
    }
}
