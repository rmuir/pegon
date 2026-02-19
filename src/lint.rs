use aho_corasick::{AhoCorasick, AhoCorasickKind};
use anyhow::Error;
use std::{ops::Range, sync::LazyLock};
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

/// Returns optional range of "top context" for the node.
/// This is typically the containing method or class declaration.
///
/// To minimize the output, only the range containing the name is returned.
///
/// Simplified version of nvim-treesitter-context
/// <https://github.com/nvim-treesitter/nvim-treesitter-context>
///
/// For example, returns the range associated with line `167`:
/// ```text
///     ╭▸ TestIndexWriterOnDiskFull.java:174:9
///     │
/// 167 │   public void testAddIndexOnDiskFull() throws IOException {
///     ‡
/// 174 │     int START_COUNT = 57;
///     │         ━━━━━━━━━━━ Uppercase
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
        parent = node.parent()
    }
    None
}

pub(crate) enum Severity {
    Error,
    Warn,
    Info,
    Hint,
}

pub(crate) struct Rule {
    /// Name of matching lint
    pub(crate) name: String,
    /// Title of lint
    pub(crate) title: String,
    /// Severity of problem
    pub(crate) severity: Severity,
    /// instructions to address the issue
    pub(crate) help: String,
    /// url with more information
    pub(crate) url: String,
    /// Text describing the matching error range
    pub(crate) label: Option<String>,
    /// describes context ranges (applied to first one)
    pub(crate) context_label: Option<String>,
    /// optional automatic fix
    pub(crate) fix: Option<String>,
}

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
    /// ranges that provide additional information
    pub(crate) context: Vec<Range<usize>>,

    // CLI only features that can't translate to LSP
    /// ranges that should be visible
    pub(crate) visible: Vec<Range<usize>>,
    /// computed top context (e.g. what function you are in)
    pub(crate) top_context: Option<Range<usize>>,
}

static JAVA_ERROR_QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &tree_sitter_java::LANGUAGE.into(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/lint.scm"
        )),
    )
    .unwrap()
});

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    let count = JAVA_ERROR_QUERY.pattern_count();
    let mut rules = Vec::with_capacity(count);
    for index in 0..count {
        let mut name: Option<String> = None;
        let mut title: Option<String> = None;
        let mut severity: Option<Severity> = None;
        let mut help: Option<String> = None;
        let mut label: Option<String> = None;
        let mut context_label: Option<String> = None;
        let mut fix: Option<String> = None;
        let props = JAVA_ERROR_QUERY.property_settings(index);
        for prop in props {
            let value = prop.value.clone().unwrap().to_string();
            match &*prop.key {
                "name" => {
                    name = Some(value);
                }
                "title" => {
                    title = Some(value);
                }
                "severity" => {
                    severity = match value.as_str() {
                        "error" => Some(Severity::Error),
                        "warn" => Some(Severity::Warn),
                        "info" => Some(Severity::Info),
                        "hint" => Some(Severity::Hint),
                        _ => {
                            panic!("invalid severity");
                        }
                    }
                }
                "help" => {
                    help = Some(value);
                }
                "label" => {
                    label = Some(value);
                }
                "context.label" => {
                    context_label = Some(value);
                }
                "fix" => {
                    fix = Some(value);
                }
                _ => {}
            }
        }
        rules.push(Rule {
            name: name.clone().unwrap(),
            title: title.unwrap(),
            severity: severity.unwrap(),
            help: help.unwrap(),
            label,
            context_label,
            fix,
            url: format!(
                "https://github.com/rmuir/pegon/wiki/lints#{}",
                name.unwrap()
            ),
        });
    }
    rules
});

static JAVA_ERROR_CAPTURE: LazyLock<u32> =
    LazyLock::new(|| JAVA_ERROR_QUERY.capture_index_for_name("error").unwrap());

static JAVA_CONTEXT_CAPTURE: LazyLock<u32> =
    LazyLock::new(|| JAVA_ERROR_QUERY.capture_index_for_name("context").unwrap());

static JAVA_VISIBLE_CAPTURE: LazyLock<u32> =
    LazyLock::new(|| JAVA_ERROR_QUERY.capture_index_for_name("visible").unwrap());

static TEMPLATE_ENGINE: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::builder()
        .kind(AhoCorasickKind::DFA.into())
        .build(["{node.text}", "{node.kind}"])
        .unwrap()
});

pub(crate) fn rule(index: usize) -> &'static Rule {
    &RULES[index]
}

pub(crate) struct Linter {
    parser: tree_sitter::Parser,
}

impl Linter {
    pub fn new() -> Self {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();
        Linter { parser }
    }

    pub fn lint(&mut self, data: &Vec<u8>) -> Result<Vec<Lint>, Error> {
        self.parser.reset();
        let tree = self.parser.parse(data, None).unwrap();
        if tree.root_node().has_error() {
            return Err(anyhow::anyhow!("syntax error"));
        }
        let mut lints = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&JAVA_ERROR_QUERY, tree.root_node(), data.as_slice());
        while let Some(hit) = matches.next() {
            let rule = rule(hit.pattern_index);

            let node = hit
                .nodes_for_capture_index(*JAVA_ERROR_CAPTURE)
                .next()
                .unwrap();

            let replacements = [node.utf8_text(data)?, node.kind()];
            let label = rule
                .label
                .as_ref()
                .map(|value| TEMPLATE_ENGINE.replace_all(value, &replacements));

            // explicitly marked visible in the query
            let mut visible = Vec::new();
            for visible_node in hit.nodes_for_capture_index(*JAVA_VISIBLE_CAPTURE) {
                visible.push(visible_node.byte_range());
            }

            // explicitly marked context in the query
            let mut context = Vec::new();
            for context_node in hit.nodes_for_capture_index(*JAVA_CONTEXT_CAPTURE) {
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
        }
        Ok(lints)
    }
}
