use core::cmp::min;
use core::ops::{ControlFlow, Range};
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{
    SemanticToken, SemanticTokens, SemanticTokensDelta, SemanticTokensDeltaParams,
    SemanticTokensDeltaResponse, SemanticTokensParams, SemanticTokensRangeParams,
};
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, StreamingIterator as _,
};

use crate::java_queries::tokens::captures;
use crate::java_queries::tokens::properties::{PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN};
use crate::lsp::semantic_cache::Cache;
use crate::support::queries::{const_array_from_fn, const_table_search, to_bool_const};

use super::{Client, server::Document};

pub fn full(
    client: &Client,
    doc: &Document,
    _params: &SemanticTokensParams,
    cancel: &AtomicBool,
    cache: &Cache,
) -> Result<Option<SemanticTokens>> {
    let tokens = tokens(client, doc, None, cancel)?;
    let result_id = cache.push(&tokens);
    Ok(Some(SemanticTokens::new(Some(result_id), tokens)))
}

pub fn range(
    client: &Client,
    doc: &Document,
    params: &SemanticTokensRangeParams,
    cancel: &AtomicBool,
) -> Result<Option<SemanticTokens>> {
    let range = client
        .decode_range(&params.range, &doc.line_index)
        .context("valid range")?;
    let byte_range = Some(&(range.start_byte..range.end_byte));
    let tokens = tokens(client, doc, byte_range, cancel)?;
    Ok(Some(SemanticTokens::new(None, tokens)))
}

pub fn delta(
    client: &Client,
    doc: &Document,
    params: &SemanticTokensDeltaParams,
    cancel: &AtomicBool,
    cache: &Cache,
) -> Result<Option<SemanticTokensDeltaResponse>> {
    let tokens = tokens(client, doc, None, cancel)?;
    let diff = cache.delta(&params.previous_result_id, &tokens);
    let result_id = cache.push(&tokens);
    if let Some(diff) = diff {
        Ok(Some(SemanticTokensDeltaResponse::SemanticTokensDelta(
            SemanticTokensDelta {
                result_id: Some(result_id),
                edits: diff,
            },
        )))
    } else {
        Ok(Some(SemanticTokensDeltaResponse::SemanticTokens(
            SemanticTokens::new(Some(result_id), tokens),
        )))
    }
}

pub fn tokens(
    client: &Client,
    doc: &Document,
    byte_range: Option<&Range<usize>>,
    cancel: &AtomicBool,
) -> Result<Vec<SemanticToken>> {
    let data = doc.text.as_bytes();
    let locals = super::locals::scopes(&doc.tree, data, cancel)?.locals;
    let mut result = Vec::with_capacity(64);
    let mut cursor = QueryCursor::new();
    if let Some(byte_range) = byte_range {
        cursor.set_byte_range(byte_range.clone());
    }

    // this callback MUST be a separate let-binding. do *NOT* factor into anonymous closure!
    let mut cancellation = |_: &QueryCursorState| {
        if cancel.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };

    let mut captures = cursor.captures_with_options(
        &QUERY,
        doc.tree.root_node(),
        data,
        QueryCursorOptions::new().progress_callback(&mut cancellation),
    );

    let mut previous_range = 0..0;
    let mut previous_index = 0;
    let mut previous_line = 0;
    let mut previous_start = 0;
    while let Some((hit, capture_id)) = captures.next() {
        let capture = hit
            .captures()
            .get(*capture_id)
            .context("valid capture id")?;
        if capture.index != captures::RANGE {
            continue;
        }
        let node_range = capture.node.byte_range();
        if let Some(byte_range) = &byte_range
            && (node_range.end < byte_range.start || node_range.start > byte_range.end)
        {
            continue;
        }

        let pattern = pattern(hit.pattern_index);
        // TODO: optimize
        let mut token_type = pattern.token_type;
        let modifiers_bitset = pattern.modifiers_bitset;
        if pattern.scoped {
            let text = capture.node.utf8_text(data)?;
            if let Some(stack) = locals.get(text) {
                for scope in stack.iter().rev() {
                    if scope.contains(node_range.start) {
                        token_type = scope.token_type;
                        break;
                    }
                }
            }
        }
        if node_range == previous_range {
            let previous: SemanticToken = result.pop().context("should exist")?;
            result.push(SemanticToken {
                delta_line: previous.delta_line,
                delta_start: previous.delta_start,
                length: previous.length,
                // override the type if we are a later pattern
                token_type: if hit.pattern_index > previous_index {
                    token_type
                } else {
                    previous.token_type
                },
                // merge modifiers
                token_modifiers_bitset: previous.token_modifiers_bitset | modifiers_bitset,
            });
            previous_index = min(previous_index, hit.pattern_index);
        } else {
            let range = client
                .encode_range(&capture.node.range(), &doc.line_index)
                .context("should encode")?;
            debug_assert!(range.start.line == range.end.line, "multiline unsupported");
            result.push(SemanticToken {
                delta_line: range
                    .start
                    .line
                    .checked_sub(previous_line)
                    .context("valid delta")?,
                delta_start: if range.start.line == previous_line {
                    range
                        .start
                        .character
                        .checked_sub(previous_start)
                        .context("valid delta")?
                } else {
                    range.start.character
                },
                length: range
                    .end
                    .character
                    .checked_sub(range.start.character)
                    .context("valid delta")?,
                token_type,
                token_modifiers_bitset: modifiers_bitset,
            });
            previous_line = range.start.line;
            previous_start = range.start.character;
            previous_range = capture.node.byte_range();
            previous_index = hit.pattern_index;
        }
    }
    Ok(result)
}

/// Semantic token types legend
pub static TOKEN_TYPES: [&str; 12] = [
    "decorator",
    "keyword",
    "label",
    "method",
    "modifier",
    "namespace",
    "operator",
    "parameter",
    "property",
    "type",
    "typeParameter",
    "variable",
];

/// Semantic token modifiers legend
pub static TOKEN_MODIFIERS: [&str; 5] = [
    "defaultLibrary",
    "definition",
    "modification",
    "readonly",
    "static",
];

/// compiled query that matches all semantic tokens patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::LANGUAGE,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/tokens.scm"
        )),
    )
    .expect("query should compile")
});

// single compiled pattern
struct Pattern {
    token_type: u32,
    modifiers_bitset: u32,
    scoped: bool,
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
#[expect(clippy::cast_possible_truncation, reason = "compile time safety")]
const fn to_pattern(pattern: usize) -> Pattern {
    let range = &PROPERTIES_BY_PATTERN[pattern];
    let mut index = range.start;
    let mut token_type = None;
    let mut modifiers_bitset = 0;
    let mut scoped = false;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"token.type" => token_type = Some(const_table_search(&TOKEN_TYPES, property.1)),
            b"token.modifier" | b"token.modifier2" => {
                modifiers_bitset |= 1 << const_table_search(&TOKEN_MODIFIERS, property.1);
            }
            b"token.scoped" => scoped = to_bool_const(property.1),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    Pattern {
        token_type: token_type.expect("local.type should be set") as u32,
        modifiers_bitset,
        scoped,
    }
}

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        DidOpenTextDocumentNotification, DidOpenTextDocumentParams, PartialResultParams,
        SemanticToken, SemanticTokens, SemanticTokensParams, SemanticTokensRequest,
        TextDocumentIdentifier, TextDocumentItem, WorkDoneProgressParams,
    };
    use indoc::indoc;

    use crate::lsp::test_client::TestClient;

    // returns token type for testing
    fn token_type(ttype: &str) -> u32 {
        super::TOKEN_TYPES
            .binary_search(&ttype)
            .unwrap()
            .try_into()
            .unwrap()
    }

    // returns token type for testing
    fn modifier(modifier: &str) -> u32 {
        1 << super::TOKEN_MODIFIERS.binary_search(&modifier).unwrap()
    }

    // helper since there isn't a ::new for this one
    fn token(
        delta_line: u32,
        delta_start: u32,
        length: u32,
        token_type: u32,
        token_modifiers_bitset: u32,
    ) -> SemanticToken {
        SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset,
        }
    }

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
                    int field = 5;
                    public abstract int bar(int param) {
                        int local = field + param;
                        return local;
                    }
                }
            "}
                .into(),
            },
        });
        let result = client
            .request::<SemanticTokensRequest>(SemanticTokensParams {
                text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
                partial_result_params: PartialResultParams::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        assert_eq!(
            result,
            SemanticTokens {
                result_id: Some(
                    "0c4aa1ee9442436e0a80dc24332baced5cc87e7bc271d2c609ba006733d599c1".into()
                ),
                data: vec![
                    token(0, 0, 6, token_type("modifier"), 0), // public
                    token(0, 7, 5, token_type("keyword"), 0),  // class
                    token(0, 6, 3, token_type("type"), modifier("definition")), // foo
                    token(1, 4, 3, token_type("type"), modifier("defaultLibrary")), // int
                    token(0, 4, 5, token_type("property"), modifier("definition")), // field
                    token(1, 4, 6, token_type("modifier"), 0), // public
                    token(0, 7, 8, token_type("modifier"), 0), // abstract
                    token(0, 9, 3, token_type("type"), modifier("defaultLibrary")), // int
                    token(0, 4, 3, token_type("method"), modifier("definition")), // bar
                    token(0, 4, 3, token_type("type"), modifier("defaultLibrary")), // int
                    token(0, 4, 5, token_type("parameter"), modifier("definition")), // param
                    token(1, 8, 3, token_type("type"), modifier("defaultLibrary")), // int
                    token(0, 4, 5, token_type("variable"), modifier("definition")), // local
                    token(0, 8, 5, token_type("property"), 0), // field
                    token(0, 8, 5, token_type("parameter"), 0), // param
                    token(1, 8, 6, token_type("keyword"), 0),  // return
                    token(0, 7, 5, token_type("variable"), 0), // local
                ]
            }
        );
    }
}
