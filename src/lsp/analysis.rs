use anyhow::{Context as _, Error};
use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};
use rustc_hash::FxHashMap;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, LazyLock};
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, Range, StreamingIterator as _, Tree,
};

use crate::support::queries::capture_id;

/// Single variable scope entry
pub struct Scope {
    /// range of the identifier declaration
    pub identifier: Range,
    /// range where the identifier is valid
    pub range: Range,
    /// range describing the java type
    pub java_type: Option<Range>,
    /// pattern that was matched
    pub pattern_id: usize,
}

impl Scope {
    /// true if the scope contains specified position
    pub const fn contains(&self, position: usize) -> bool {
        (self.range.start_byte <= position && self.range.end_byte >= position)
            || (self.identifier.start_byte <= position && self.identifier.end_byte >= position)
    }

    pub fn semantic_token_type(&self) -> u32 {
        pattern(self.pattern_id).token_type
    }
}

/// Returns a map of scopes keyed by identifier in the document
///
/// # Errors
///
/// This function will return an error if rules are misconfigured.
pub fn scopes<'data>(
    tree: &Tree,
    data: &'data [u8],
    cancel_token: &Arc<AtomicBool>,
) -> Result<FxHashMap<&'data str, Vec<Scope>>, Error> {
    let mut scopes = FxHashMap::default();
    let mut cursor = QueryCursor::new();

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
        tree.root_node(),
        data,
        QueryCursorOptions::new().progress_callback(&mut cancellation),
    );
    while let Some(hit) = matches.next() {
        let pattern = pattern(hit.pattern_index);

        let var_node = hit
            .nodes_for_capture_index(*DEFINITION_CAPTURE)
            .next()
            .context("definition capture should exist")?;

        let mut start_node = hit
            .nodes_for_capture_index(*START_CAPTURE)
            .next()
            .context("start capture should exist")?;

        let mut end_node = hit
            .nodes_for_capture_index(*END_CAPTURE)
            .next()
            .context("end capture should exist")?;

        let type_node = hit.nodes_for_capture_index(*TYPE_CAPTURE).next();

        if pattern.flow {
            let mut node = tree.root_node();
            while let Some(child) = node.child_with_descendant(var_node) {
                if child.kind() == "block" {
                    start_node = child;
                    end_node = child;
                }
                node = child;
            }
        }

        let key = var_node.utf8_text(data)?;
        let value = scopes.entry(key).or_insert_with(|| Vec::with_capacity(4));
        let start_range = start_node.range();
        let end_range = end_node.range();
        value.push(Scope {
            identifier: var_node.range(),
            pattern_id: hit.pattern_index,
            java_type: type_node.map(|node| node.range()),
            range: if pattern.start_inclusive {
                Range {
                    start_byte: start_range.start_byte,
                    start_point: start_range.start_point,
                    end_byte: end_range.end_byte,
                    end_point: end_range.end_point,
                }
            } else {
                Range {
                    start_byte: start_range.end_byte,
                    start_point: start_range.end_point,
                    end_byte: end_range.end_byte,
                    end_point: end_range.end_point,
                }
            },
        });
    }
    Ok(scopes)
}

/// single compiled pattern
pub struct Pattern {
    /// identifier type
    pub kind: Kind,
    /// semantic token type
    pub token_type: u32,
    /// whether the start capture is inclusive or exclusive
    pub start_inclusive: bool,
    /// whether scope is based on control flow rather than lexical
    pub flow: bool,
}

/// Classifies the scope entry
#[derive(Copy, Clone)]
pub enum Kind {
    Type,
    Parameter,
    Property,
    TypeParameter,
    Variable,
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::Type => write!(f, "type"),
            Self::Parameter => write!(f, "parameter"),
            Self::Property => write!(f, "field"),
            Self::TypeParameter => write!(f, "type parameter"),
            Self::Variable => write!(f, "local variable"),
        }
    }
}

/// Look up metadata by pattern index
#[must_use]
pub fn pattern(index: usize) -> &'static Pattern {
    PATTERNS.get(index).expect("pattern should exist")
}

/// compiled query that matches all lint rules
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::language(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/analysis.scm"
        )),
    )
    .expect("query should compile")
});

/// array of patterns indexed by patterns of `QUERY`
static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    let count = QUERY.pattern_count();
    let mut patterns = Vec::with_capacity(count);
    for index in 0..count {
        let mut kind = None;
        let mut token_type = None;
        let mut start_inclusive = true;
        let mut flow = false;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "analysis.flow" => {
                    let value = value.expect("analysis.flow should have a value");
                    flow = value.parse::<bool>().expect("valid boolean");
                }
                "analysis.kind" => {
                    let value = value.expect("analysis.type should have a value");
                    kind = Some(match value {
                        "type" => Kind::Type,
                        "parameter" => Kind::Parameter,
                        "property" => Kind::Property,
                        "typeParameter" => Kind::TypeParameter,
                        "variable" => Kind::Variable,
                        _ => panic!("unknown kind: {value}"),
                    });
                    token_type = super::semantic_tokens::TOKEN_TYPES
                        .binary_search(&value)
                        .ok();
                    assert!(token_type.is_some(), "unknown token type: {value}");
                }
                "analysis.start.inclusive" => {
                    let value = value.expect("analysis.start.inclusive should have a value");
                    start_inclusive = value.parse::<bool>().expect("valid boolean");
                }
                _ => panic!("{key}: unknown metadata key"),
            }
        }
        patterns.push(Pattern {
            kind: kind.expect("analysis.kind should be set"),
            token_type: token_type
                .expect("analysis kind should be set")
                .try_into()
                .expect("should be u32"),
            start_inclusive,
            flow,
        });
    }
    patterns
});

/// index of the `@definition` capture
static DEFINITION_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "definition"));

/// index of the `@start` capture
static START_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "start"));

/// index of the `@end` capture
static END_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "end"));

/// index of the `@type` capture
static TYPE_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "type"));
