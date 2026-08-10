use aho_corasick::{AhoCorasick, AhoCorasickKind};
use anyhow::{Context as _, Error};
use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};
use rustc_hash::FxHashMap;
use std::cmp::max;
use std::path::Path;
use std::sync::LazyLock;
use tree_sitter::{
    Node, Query, QueryCursor, QueryCursorOptions, QueryCursorState, Range, StreamingIterator as _,
    Tree,
};

use crate::java_constants::{fields, kinds};
use crate::java_queries::diagnostics::captures;
use crate::java_queries::diagnostics::predicates::{PREDICATES, PREDICATES_BY_PATTERN};
use crate::java_queries::diagnostics::properties::{
    PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN,
};
use crate::support::fix::Fix;
use crate::support::queries::{KindSet, PredicateMatch as _, const_array_from_fn};

/// Single diagnostic result
pub struct Diagnostic {
    /// Matched rule
    pub pattern_id: usize,
    /// Primary matching error node range
    pub range: Range,
    /// Formatted title of problem
    pub title: String,
    /// Formatted instructions to address the issue
    pub help: String,
    /// Formatted Text describing the matching error range
    pub label: Option<String>,
    /// Range that provides additional information
    pub context: Option<Range>,

    // CLI only features that can't translate to LSP
    /// Range that should be visible
    pub visible: Option<Range>,
    /// Computed top context (e.g. what function you are in)
    pub top_context: Option<Range>,
}

/// Returns any lint errors found against the document.
///
/// # Errors
///
/// This function will return an error if rules are misconfigured.
pub fn lint(
    tree: &Tree,
    data: &[u8],
    cancel: &AtomicBool,
    extras: bool,
    path: Option<&Path>,
) -> Result<Vec<Diagnostic>, Error> {
    let mut lints = Vec::new();
    let mut cursor = QueryCursor::new();

    // this callback MUST be a separate let-binding. do *NOT* factor into anonymous closure!
    let mut cancellation = |_: &QueryCursorState| {
        if cancel.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };

    #[expect(clippy::indexing_slicing, reason = "checked at compile-time")]
    let mut matches = cursor
        .matches_with_options(
            &QUERY,
            tree.root_node(),
            data,
            QueryCursorOptions::new().progress_callback(&mut cancellation),
        )
        .filter(|hit| {
            let list = &PREDICATES_BY_PATTERN[hit.pattern_index];
            for index in list.start..list.end {
                if !PREDICATES[index as usize].matches(hit, data, path) {
                    return false;
                }
            }
            true
        });
    while let Some(hit) = matches.next() {
        let rule = pattern(hit.pattern_index);

        let node = hit
            .nodes_for_capture_index(captures::ERROR)
            .next()
            .context("error capture should exist")?;

        let replacements = [node.utf8_text(data)?, node.kind()];
        let label = rule
            .label
            .as_ref()
            .map(|value| TEMPLATE_ENGINE.replace_all(value, &replacements));

        // explicitly marked context in the query
        let context = hit
            .nodes_for_capture_index(captures::CONTEXT)
            .map(|item| item.range())
            .next();

        // explicitly marked visible in the query
        let visible = if extras {
            hit.nodes_for_capture_index(captures::VISIBLE)
                .map(|item| item.range())
                .next()
        } else {
            None
        };

        // computed top context
        let top_context = if extras {
            top_context(tree.root_node(), node)
        } else {
            None
        };

        lints.push(Diagnostic {
            pattern_id: hit.pattern_index,
            range: node.range(),
            title: TEMPLATE_ENGINE.replace_all(rule.title, &replacements),
            help: TEMPLATE_ENGINE.replace_all(rule.help, &replacements),
            label,
            visible,
            context,
            top_context,
        });
        // stop linting the document at the first ERROR or MISSING node
        // alerts to the issue, but prevents annoying cascade
        if hit.pattern_index < 2 {
            break;
        }
    }
    Ok(lints)
}

/// single rule (compiled pattern)
pub struct Pattern {
    /// Name such as `[missing-foobar]`
    pub name: &'static str,
    /// Template description of problem
    pub title: &'static str,
    /// Template of instructions to address the issue
    pub help: &'static str,
    /// Template describing the matching error range
    pub label: Option<&'static str>,
    /// Describes context ranges (applied to first one)
    pub context_label: Option<&'static str>,
    /// Optional automatic fix
    pub fix: Option<Fix>,
    /// Severity of problem
    pub severity: Severity,
}

impl Pattern {
    pub fn url(&self) -> String {
        format!(
            "https://github.com/rmuir/pegon/wiki/diagnostics#{}",
            self.name
        )
    }
}

/// rule severity
#[derive(Copy, Clone)]
pub enum Severity {
    /// Serious problem that must be addressed (e.g. invalid code)
    Error,
    /// Problem that should definitely be addressed
    Warn,
    /// Minor problem
    Info,
    /// Nitpick that can be automatically fixed
    Hint,
}

impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }
}

/// Look up rule by pattern index
#[expect(clippy::indexing_slicing, reason = "compile time safety")]
pub const fn pattern(index: usize) -> &'static Pattern {
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
    let mut name: Option<&str> = None;
    let mut title: Option<&str> = None;
    let mut severity: Option<Severity> = None;
    let mut help: Option<&str> = None;
    let mut label: Option<&str> = None;
    let mut context_label: Option<&str> = None;
    let mut fix_arg: Option<&str> = None;
    let mut fix_kind: Option<&str> = None;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"diagnostic.name" => name = Some(property.1),
            b"diagnostic.title" => title = Some(property.1),
            b"diagnostic.severity" => severity = Some(to_severity(property.1)),
            b"diagnostic.help" => help = Some(property.1),
            b"diagnostic.label" => label = Some(property.1),
            b"diagnostic.context.label" => context_label = Some(property.1),
            b"diagnostic.fix.kind" => fix_kind = Some(property.1),
            b"diagnostic.fix.arg" => fix_arg = Some(property.1),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    Pattern {
        name: name.expect("pattern should have a name"),
        title: title.expect("pattern should have a title"),
        severity: severity.expect("pattern should have a severity"),
        help: help.expect("pattern should have a help"),
        label,
        context_label,
        fix: if let Some(fix_kind) = fix_kind {
            Some(to_fix(fix_kind, fix_arg))
        } else {
            None
        },
    }
}

const fn to_severity(string: &str) -> Severity {
    match string.as_bytes() {
        b"error" => Severity::Error,
        b"warn" => Severity::Warn,
        b"info" => Severity::Info,
        b"hint" => Severity::Hint,
        _ => panic!("unknown severity value"),
    }
}

const fn to_fix(string: &str, fix_arg: Option<&'static str>) -> Fix {
    match string.as_bytes() {
        b"escape_whitespace" => Fix::EscapeWhitespace,
        b"line_unwrap" => Fix::LineUnwrap,
        b"static" => Fix::Static(fix_arg.expect("static fix should have an arg")),
        b"to_upper" => Fix::ToUpper,
        b"organize_imports" => Fix::OrganizeImports,
        _ => panic!("unknown fix type"),
    }
}

/// Lookup rule by name
#[must_use]
pub fn pattern_by_name(name: &str) -> Option<&'static Pattern> {
    PATTERNS_BY_NAME.get(name).map(|index| pattern(*index))
}

/// Returns optional range of "top context" for the node.
/// This is typically the containing method or class declaration.
///
/// To minimize the output, only the range containing the name is returned.
///
/// Super-simplified version of nvim-treesitter-context
/// <https://github.com/nvim-treesitter/nvim-treesitter-context>
///
/// For example, returns the range associated with line `167`:
/// ```text
///     ╭▸ TestIndexWriterOnDiskFull.java:174:9
///     │
/// 167 │   public void testAddIndexOnDiskFull() throws IOException {
///     ‡
/// 174 │     int START_COUNT = 57;
///     │         ━━━━━━━━━━━
///     ╰╴
/// ```
fn top_context(root: Node, error_node: Node) -> Option<Range> {
    let mut range = None;
    let mut node = root;
    while let Some(child) = node.child_with_descendant(error_node)
        && child.id() != error_node.id()
    {
        if TOP_CONTEXT_KINDS.contains(child.kind_id())
            && let Some(name) = child.child_by_field_id(fields::NAME)
            && name.start_position().row != error_node.start_position().row
        {
            range = Some(name.range());
        }
        node = child;
    }
    range
}

/// set of context parent node kinds
const TOP_CONTEXT_KINDS: KindSet = KindSet::new(&[
    kinds::METHOD_DECLARATION,
    kinds::VARIABLE_DECLARATOR,
    kinds::CONSTRUCTOR_DECLARATION,
    kinds::CLASS_DECLARATION,
    kinds::INTERFACE_DECLARATION,
    kinds::ENUM_DECLARATION,
    kinds::RECORD_DECLARATION,
]);

/// compiled query that matches all lint rules
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::language(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/diagnostics.scm"
        )),
    )
    .expect("query should compile")
});

static PATTERNS_BY_NAME: LazyLock<FxHashMap<&str, usize>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .enumerate()
        .map(|(index, item)| (item.name, index))
        .collect()
});

/// simple error templating engine
static TEMPLATE_ENGINE: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::builder()
        .kind(AhoCorasickKind::DFA.into())
        .build(["{node.text}", "{node.kind}"])
        .expect("dfa should build")
});

impl Diagnostic {
    /// compute diagnostic's bounding box for more efficient rendering
    pub fn bounds(&self, source: &str) -> DiagnosticBounds {
        // 4 possible ranges
        let ranges = [
            self.top_context.as_ref(),
            Some(&self.range),
            self.context.as_ref(),
            self.visible.as_ref(),
        ];

        let mut start_byte = usize::MAX;
        let mut line_start = 0;
        let mut end_byte = 0;

        // compute the box
        for range in ranges.iter().flatten() {
            if range.start_byte < start_byte {
                start_byte = range.start_byte;
                line_start = range.start_point.row;
            }
            end_byte = max(end_byte, range.end_byte);
        }

        // expand the box so it includes full lines
        let start = source
            .get(..start_byte)
            .and_then(|text| text.rfind('\n'))
            .and_then(|offset| offset.checked_add(1))
            .unwrap_or_default();
        let end = source
            .get(end_byte..)
            .and_then(|text| text.find('\n'))
            .and_then(|offset| offset.checked_add(end_byte))
            .and_then(|offset| offset.checked_add(1))
            .unwrap_or(source.len());

        DiagnosticBounds {
            range: start..end,
            line_start,
        }
    }
}

/// Bounding box of a diagnostic
pub struct DiagnosticBounds {
    /// byte range containing all the diagnostic info
    pub range: std::ops::Range<usize>,
    /// line number the diagnostic starts at
    pub line_start: usize,
}
