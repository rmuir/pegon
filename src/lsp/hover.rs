use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};

use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{Contents, Hover, MarkupContent, MarkupKind, Position};
use indoc::formatdoc;
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _,
};

use super::{Client, server::Document};
use crate::java_queries::hover::captures;
use crate::java_queries::hover::properties::{PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN};
use crate::support::queries::const_array_from_fn;

pub fn request(
    client: &Client,
    doc: &Document,
    position: Position,
    cancel: &AtomicBool,
) -> Result<Option<Hover>> {
    let markdown = client.prefers_hover_markdown();
    let bytes = doc.text.as_bytes();
    // TODO: do this lazily
    let locals = super::locals::scopes(&doc.tree, bytes, cancel)?.locals;
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
        if cancel.load(Ordering::Relaxed) {
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
            .nodes_for_capture_index(captures::RANGE)
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
                if let Some(stack) = locals.get(text) {
                    for scope in stack.iter().rev() {
                        if scope.contains(node_range.start_byte) {
                            let kind = super::semantic_tokens::TOKEN_TYPES
                                .get(scope.token_type as usize)
                                .context("valid token type")?;
                            let java_type = if let Some(java_type) = scope.java_type {
                                java_type.utf8_text(bytes)?
                            } else {
                                "var"
                            };
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
    summary: &'static str,
    /// description
    description: &'static str,
    /// reference
    reference: &'static str,
}

/// Look up rule by pattern index
#[expect(clippy::indexing_slicing, reason = "compile time safety")]
const fn pattern(index: usize) -> &'static Pattern {
    &PATTERNS[index]
}

/// array of pattern metadata from `QUERY` by index
#[expect(clippy::indexing_slicing, reason = "compile time safety")]
#[expect(clippy::arithmetic_side_effects, reason = "compile time safety")]
const PATTERNS: [Pattern; PATTERN_COUNT] = const_array_from_fn!(to_pattern, PATTERN_COUNT);

#[expect(clippy::indexing_slicing, reason = "compile time safety")]
#[expect(clippy::arithmetic_side_effects, reason = "compile time safety")]
const fn to_pattern(pattern: usize) -> Pattern {
    let range = &PROPERTIES_BY_PATTERN[pattern];
    let mut index = range.start;
    let mut kind: Option<&str> = None;
    let mut summary: Option<&str> = None;
    let mut reference: Option<&str> = None;
    let mut description: Option<&str> = None;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"hover.spec.description" => description = Some(property.1),
            b"hover.spec.summary" => summary = Some(property.1),
            b"hover.spec.reference" => reference = Some(property.1),
            b"hover.kind" => kind = Some(to_kind(property.1)),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    match kind.expect("should be set").as_bytes() {
        b"reference" => Pattern::Reference,
        b"bail" => Pattern::Bail,
        b"spec" => Pattern::Spec(SpecPattern {
            summary: summary.expect("summary should be set"),
            description: description.expect("description should be set"),
            reference: reference.expect("reference should be set"),
        }),
        _ => panic!("should not reach here"),
    }
}

const fn to_kind(string: &str) -> &str {
    match string.as_bytes() {
        b"reference" | b"bail" | b"spec" => string,
        _ => panic!("unknown kind"),
    }
}

/// compiled query that matches all folding patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::LANGUAGE,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/hover.scm"
        )),
    )
    .expect("query should compile")
});

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        Contents, DidOpenTextDocumentNotification, DidOpenTextDocumentParams, Hover, HoverParams,
        HoverRequest, MarkupContent, MarkupKind, Position, Range, TextDocumentIdentifier,
        TextDocumentItem, TextDocumentPositionParams, WorkDoneProgressParams,
    };
    use indoc::indoc;

    use crate::lsp::test_client::TestClient;

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
