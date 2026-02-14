use annotate_snippets::{
    Annotation, AnnotationKind, Group, Level, Patch, Renderer, Snippet,
    renderer::{DecorStyle, Style},
};
use std::{ops::Range, path::Path, sync::LazyLock};
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

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

static JAVA_ERROR_CAPTURE: LazyLock<u32> =
    LazyLock::new(|| JAVA_ERROR_QUERY.capture_index_for_name("error").unwrap());

static JAVA_VISIBLE_CAPTURE: LazyLock<u32> =
    LazyLock::new(|| JAVA_ERROR_QUERY.capture_index_for_name("visible").unwrap());

static RENDERER: Renderer = Renderer::styled()
    .decor_style(DecorStyle::Unicode)
    .line_num(Style::new().dimmed());

/// simplified version of nvim-treesitter-context
/// <https://github.com/nvim-treesitter/nvim-treesitter-context>
fn top_context(error_node: &Node) -> Option<Range<usize>> {
    let mut parent = error_node.parent();
    while let Some(node) = parent {
        match node.kind() {
            "method_declaration"
            | "constructor_declaration"
            | "class_declaration"
            | "record_declaration" => {
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

    pub fn lint(&mut self, path: &Path, data: Vec<u8>) -> u32 {
        self.parser.reset();
        let mut errors = 0;
        let tree = self.parser.parse(&data, None).unwrap();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&JAVA_ERROR_QUERY, tree.root_node(), data.as_slice());
        while let Some(hit) = matches.next() {
            errors += 1;
            let props = JAVA_ERROR_QUERY.property_settings(hit.pattern_index);
            let mut prop_name: Option<Box<str>> = None;
            let mut prop_title: Option<Box<str>> = None;
            let mut prop_severity: Option<Box<str>> = None;
            let mut prop_label: Option<Box<str>> = None;
            let mut prop_note: Option<Box<str>> = None;
            let mut prop_fix: Option<Box<str>> = None;
            for prop in props {
                let name = &*prop.key;
                if name == "name" {
                    prop_name = prop.value.clone();
                } else if name == "title" {
                    prop_title = prop.value.clone();
                } else if name == "severity" {
                    prop_severity = prop.value.clone();
                } else if name == "label" {
                    prop_label = prop.value.clone();
                } else if name == "note" {
                    prop_note = prop.value.clone();
                } else if name == "fix" {
                    prop_fix = prop.value.clone();
                }
            }
            let name = prop_name.unwrap().to_string();
            let prop_url = format!("https://github.com/rmuir/pegon/wiki/lints#{}", name);

            let node = hit
                .nodes_for_capture_index(*JAVA_ERROR_CAPTURE)
                .next()
                .unwrap();

            let label = if node.is_missing() {
                format!("missing {} here", node.kind())
            } else {
                prop_label.unwrap_or_default().to_string()
            };

            let mut annotations: Vec<Annotation> = Vec::new();

            // primary error annotation: as precise of a range as possible
            annotations.push(AnnotationKind::Primary.span(node.byte_range()).label(label));

            // node context: explicitly marked visible in the query
            for visible in hit.nodes_for_capture_index(*JAVA_VISIBLE_CAPTURE) {
                annotations.push(AnnotationKind::Visible.span(visible.byte_range()));
            }

            // top context: e.g. what function are you in
            if let Some(ctx) = top_context(&node) {
                annotations.push(AnnotationKind::Visible.span(ctx));
            }

            let source = str::from_utf8(data.as_slice()).unwrap();

            let severity = prop_severity.unwrap().to_string();
            let level = match severity.as_str() {
                "error" => Level::ERROR,
                "warning" => Level::WARNING,
                "info" => Level::INFO,
                "hint" => Level::NOTE,
                _ => Level::ERROR,
            };
            let mut report = Vec::new();
            report.push(
                level
                    .primary_title(prop_title.unwrap().to_string())
                    .id(name)
                    .id_url(prop_url)
                    .element(
                        Snippet::source(source)
                            .path(path.to_str())
                            .annotations(annotations),
                    ),
            );
            if let Some(fix) = prop_fix {
                report.push(
                    Level::HELP
                        .secondary_title(prop_note.unwrap().to_string())
                        .element(
                            Snippet::source(source)
                                .patch(Patch::new(node.byte_range(), fix.to_string())),
                        ),
                );
            } else {
                report.push(Group::with_title(
                    Level::HELP.secondary_title(prop_note.unwrap().to_string()),
                ));
            }
            anstream::println!("{}\n", RENDERER.render(&report))
        }
        errors
    }
}
