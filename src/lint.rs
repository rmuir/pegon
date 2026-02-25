use aho_corasick::{AhoCorasick, AhoCorasickKind};
use anyhow::{Context, Error};
use std::{ops::Range, sync::LazyLock};
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

/// Single diagnostic result
#[derive(Hash)]
pub(crate) struct Lint {
    /// Matched rule
    pub(crate) rule_id: usize,
    /// Primary matching error node range
    pub(crate) range: Range<usize>,
    /// Formatted title of problem
    pub(crate) title: String,
    /// Formatted instructions to address the issue
    pub(crate) help: String,
    /// Formatted Text describing the matching error range
    pub(crate) label: Option<String>,
    /// Ranges that provide additional information
    pub(crate) context: Vec<Range<usize>>,

    // CLI only features that can't translate to LSP
    /// Ranges that should be visible
    pub(crate) visible: Vec<Range<usize>>,
    /// Computed top context (e.g. what function you are in)
    pub(crate) top_context: Option<Range<usize>>,
}

/// Runs lint queries against a parse tree, returning any lints found
pub(crate) fn lint(tree: &Tree, data: &[u8]) -> Result<Vec<Lint>, Error> {
    let has_error = tree.root_node().has_error();
    let mut lints = Vec::new();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&QUERY, tree.root_node(), data);
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
        let mut visible = Vec::new();
        for visible_node in hit.nodes_for_capture_index(*VISIBLE_CAPTURE) {
            visible.push(visible_node.byte_range());
        }

        // explicitly marked context in the query
        let mut context = Vec::new();
        for context_node in hit.nodes_for_capture_index(*CONTEXT_CAPTURE) {
            context.push(context_node.byte_range());
        }

        lints.push(Lint {
            rule_id: hit.pattern_index,
            range: node.byte_range(),
            title: TEMPLATE_ENGINE.replace_all(&rule.title, &replacements),
            help: TEMPLATE_ENGINE.replace_all(&rule.help, &replacements),
            label,
            visible,
            context,
            top_context: top_context(&node),
        });
        // stop linting the document at the first ERROR or MISSING node
        // alerts to the issue, but prevents annoying cascade
        if has_error && hit.pattern_index < 2 {
            break;
        }
    }
    Ok(lints)
}

// single rule (compiled pattern)
pub(crate) struct Rule {
    /// Name such as `[missing-foobar]`
    pub(crate) name: String,
    /// Template description of problem
    pub(crate) title: String,
    /// Severity of problem
    pub(crate) severity: Severity,
    /// Template of instructions to address the issue
    pub(crate) help: String,
    /// URL with more information
    pub(crate) url: String,
    /// Template describing the matching error range
    pub(crate) label: Option<String>,
    /// Describes context ranges (applied to first one)
    pub(crate) context_label: Option<String>,
    /// Optional automatic fix
    pub(crate) fix: Option<String>,
}

/// rule severity
#[derive(Copy, Clone)]
pub(crate) enum Severity {
    /// Serious problem that must be addressed (e.g. invalid code)
    Error,
    /// Problem that should definitely be addressed
    Warn,
    /// Minor problem
    Info,
    /// Nitpick that can be automatically fixed
    Hint,
}

// Look up rule by pattern index
pub(crate) fn rule(index: usize) -> &'static Rule {
    &RULES[index]
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
fn top_context(error_node: &Node) -> Option<Range<usize>> {
    let mut parent = error_node.parent();
    while let Some(node) = parent {
        match node.kind() {
            "method_declaration"
            | "variable_declarator"
            | "constructor_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration" => {
                // keep traversing upwards until we find a node not on the same line.
                if let Some(name) = node.child_by_field_name("name")
                    && name.start_position().row != error_node.start_position().row
                {
                    return Some(name.byte_range());
                }
            }
            _ => {}
        }
        parent = node.parent();
    }
    None
}

/// compiled query that matches all lint rules
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &tree_sitter_java::LANGUAGE.into(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/lint.scm"
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
        let mut fix: Option<&str> = None;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "name" => {
                    name = value;
                }
                "title" => {
                    title = value;
                }
                "severity" => {
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
                "help" => {
                    help = value;
                }
                "label" => {
                    label = value;
                }
                "context.label" => {
                    context_label = value;
                }
                "fix" => {
                    fix = value;
                }
                _ => {}
            }
        }
        rules.push(Rule {
            name: name.expect("pattern should have a name").to_string(),
            title: title.expect("pattern should have a title").to_string(),
            severity: severity.expect("pattern should have a severity"),
            help: help.expect("pattern should have a help").to_string(),
            label: label.map(ToString::to_string),
            context_label: context_label.map(ToString::to_string),
            fix: fix.map(ToString::to_string),
            url: format!(
                "https://github.com/rmuir/pegon/wiki/lints#{}",
                name.expect("pattern should have a name")
            ),
        });
    }
    rules
});

/// index of the `@error` capture
static ERROR_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("error")
        .expect("error capture should exist")
});

/// index of the `@context` capture
static CONTEXT_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("context")
        .expect("context capture should exist")
});

/// index of the `@visible` capture
static VISIBLE_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("visible")
        .expect("visible capture should exist")
});

/// simple error templating engine
static TEMPLATE_ENGINE: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::builder()
        .kind(AhoCorasickKind::DFA.into())
        .build(["{node.text}", "{node.kind}"])
        .expect("dfa should build")
});
