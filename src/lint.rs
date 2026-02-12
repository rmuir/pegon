use annotate_snippets::{
    Annotation, AnnotationKind, Level, Renderer, Snippet, renderer::DecorStyle,
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

static JAVA_CONTEXT_CAPTURE: LazyLock<u32> =
    LazyLock::new(|| JAVA_ERROR_QUERY.capture_index_for_name("visible").unwrap());

static RENDERER: Renderer = Renderer::styled().decor_style(DecorStyle::Unicode);

fn ts_context(node: &Node) -> Option<Range<usize>> {
    let mut parent = node.parent();
    while let Some(p) = parent {
        match p.kind() {
            "method_declaration" | "constructor_declaration" | "class_definition" => {
                if let Some(body) = p.child_by_field_name("body") {
                    return Some(p.range().start_byte..body.range().start_byte);
                }
            }
            _ => {}
        }
        parent = p.parent();
    }
    None
}

pub struct Linter {
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

    pub fn lint(&mut self, path: &Path, data: Vec<u8>) {
        self.parser.reset();
        let tree = self.parser.parse(&data, None).unwrap();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&JAVA_ERROR_QUERY, tree.root_node(), data.as_slice());
        while let Some(hit) = matches.next() {
            let props = JAVA_ERROR_QUERY.property_settings(hit.pattern_index);
            let mut prop_name: Option<Box<str>> = None;
            let mut prop_title: Option<Box<str>> = None;
            let mut prop_label: Option<Box<str>> = None;
            let mut prop_severity: Option<Box<str>> = None;
            let mut prop_help: Option<Box<str>> = None;
            for prop in props {
                let name = &*prop.key;
                if name == "name" {
                    prop_name = prop.value.clone();
                } else if name == "title" {
                    prop_title = prop.value.clone();
                } else if name == "label" {
                    prop_label = prop.value.clone();
                } else if name == "severity" {
                    prop_severity = prop.value.clone();
                } else if name == "help" {
                    prop_help = prop.value.clone();
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
                prop_label.unwrap().to_string()
            };

            let mut annotations: Vec<Annotation> = Vec::new();
            annotations.push(
                AnnotationKind::Primary
                    .span(node.byte_range())
                    .label(label)
                    .highlight_source(true),
            );

            for context in hit.nodes_for_capture_index(*JAVA_CONTEXT_CAPTURE) {
                annotations.push(AnnotationKind::Visible.span(context.byte_range()));
            }

            if let Some(ctx) = ts_context(&node) {
                annotations.push(AnnotationKind::Visible.span(ctx.start..ctx.end));
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
            let report = &[level
                .primary_title(prop_title.unwrap().to_string())
                .id(name)
                .id_url(prop_url)
                .element(
                    Snippet::source(source)
                        .path(path.to_str())
                        .annotations(annotations),
                )
                .element(Level::HELP.message(prop_help.unwrap().to_string()))];
            anstream::println!("{}\n", RENDERER.render(report))
        }
    }
}
