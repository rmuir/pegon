use aho_corasick::{AhoCorasick, AhoCorasickKind};
use anyhow::{Context as _, Error};
use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use tree_sitter::{
    Node, Query, QueryCursor, QueryCursorOptions, QueryCursorState, Range, StreamingIterator as _,
    Tree,
};

use crate::support::queries::{capture_id, custom_predicate};

/// Single diagnostic result
pub struct Diagnostic {
    /// Matched rule
    pub rule_id: usize,
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
    cancel_token: &Arc<AtomicBool>,
) -> Result<Vec<Diagnostic>, Error> {
    let mut lints = Vec::new();
    let mut cursor = QueryCursor::new();

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
            tree.root_node(),
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
    while let Some(hit) = matches.next() {
        let rule = rule(hit.pattern_index);

        let node = hit
            .nodes_for_capture_index(*ERROR_CAPTURE)
            .next()
            .context("error capture should exist")?;

        let replacements = [node.utf8_text(data)?, node.kind()];
        let label = rule
            .label
            .as_ref()
            .map(|value| TEMPLATE_ENGINE.replace_all(value, &replacements));

        // explicitly marked visible in the query
        let visible = hit
            .nodes_for_capture_index(*VISIBLE_CAPTURE)
            .map(|item| item.range())
            .next();

        // explicitly marked context in the query
        let context = hit
            .nodes_for_capture_index(*CONTEXT_CAPTURE)
            .map(|item| item.range())
            .next();

        lints.push(Diagnostic {
            rule_id: hit.pattern_index,
            range: node.range(),
            title: TEMPLATE_ENGINE.replace_all(&rule.title, &replacements),
            help: TEMPLATE_ENGINE.replace_all(&rule.help, &replacements),
            label,
            visible,
            context,
            top_context: top_context(tree.root_node(), node),
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
pub struct Rule {
    /// Name such as `[missing-foobar]`
    pub name: String,
    /// Template description of problem
    pub title: String,
    /// Severity of problem
    pub severity: Severity,
    /// Template of instructions to address the issue
    pub help: String,
    /// URL with more information
    pub url: String,
    /// Template describing the matching error range
    pub label: Option<String>,
    /// Describes context ranges (applied to first one)
    pub context_label: Option<String>,
    /// Optional automatic fix
    pub fix: Option<Fix>,
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

/// rule fix types
pub enum Fix {
    Static(String),
}

/// Look up rule by pattern index
#[must_use]
pub fn rule(index: usize) -> &'static Rule {
    RULES.get(index).expect("rule should exist")
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
        match child.kind() {
            "method_declaration"
            | "variable_declarator"
            | "constructor_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration" => {
                if let Some(name) = child.child_by_field_name("name")
                    && name.start_position().row != error_node.start_position().row
                {
                    range = Some(name.range());
                }
            }
            _ => {}
        }
        node = child;
    }
    range
}

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

/// array of rules indexed by patterns of `QUERY`
static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    let count = QUERY.pattern_count();
    let mut rules = Vec::with_capacity(count);
    for index in 0..count {
        let mut name: Option<&str> = None;
        let mut title: Option<&str> = None;
        let mut severity: Option<Severity> = None;
        let mut help: Option<&str> = None;
        let mut label: Option<&str> = None;
        let mut context_label: Option<&str> = None;
        let mut fix_arg: Option<&str> = None;
        let mut fix_kind: Option<&str> = None;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "diagnostic.name" => {
                    name = value;
                }
                "diagnostic.title" => {
                    title = value;
                }
                "diagnostic.severity" => {
                    severity = match value {
                        Some("error") => Some(Severity::Error),
                        Some("warn") => Some(Severity::Warn),
                        Some("info") => Some(Severity::Info),
                        Some("hint") => Some(Severity::Hint),
                        _ => {
                            panic!("invalid severity");
                        }
                    }
                }
                "diagnostic.help" => {
                    help = value;
                }
                "diagnostic.label" => {
                    label = value;
                }
                "diagnostic.context.label" => {
                    context_label = value;
                }
                "diagnostic.fix.kind" => {
                    fix_kind = value;
                }
                "diagnostic.fix.arg" => {
                    fix_arg = value;
                }
                _ => panic!("{key}: unknown metadata key"),
            }
        }
        let fix = match fix_kind {
            Some("static") => Some(Fix::Static(
                fix_arg.expect("static fix should have an arg").into(),
            )),
            Some(other) => panic!("{other}: unknown fix type"),
            None => None,
        };
        rules.push(Rule {
            name: name.expect("pattern should have a name").into(),
            title: title.expect("pattern should have a title").into(),
            severity: severity.expect("pattern should have a severity"),
            help: help.expect("pattern should have a help").into(),
            label: label.map(str::to_owned),
            context_label: context_label.map(str::to_owned),
            url: format!(
                "https://github.com/rmuir/pegon/wiki/diagnostics#{}",
                name.expect("pattern should have a name")
            ),
            fix,
        });
    }
    rules
});

/// index of the `@error` capture
static ERROR_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "error"));

/// index of the `@context` capture
static CONTEXT_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "context"));

/// index of the `@visible` capture
static VISIBLE_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "visible"));

/// simple error templating engine
static TEMPLATE_ENGINE: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::builder()
        .kind(AhoCorasickKind::DFA.into())
        .build(["{node.text}", "{node.kind}"])
        .expect("dfa should build")
});
