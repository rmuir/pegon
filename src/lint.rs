use annotate_snippets::{
    Annotation, AnnotationKind, Level, Renderer, Snippet, renderer::DecorStyle
};
use std::{cmp::min, path::Path, sync::LazyLock};
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

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

static RENDERER: Renderer = Renderer::styled().decor_style(DecorStyle::Unicode);

pub fn lint_document(tree: &Tree, path: &Path, data: Vec<u8>) {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&JAVA_ERROR_QUERY, tree.root_node(), data.as_slice());
    while let Some(hit) = matches.next() {

        let props = JAVA_ERROR_QUERY.property_settings(hit.pattern_index);
        let mut prop_name: Option<Box<str>> = None;
        let mut prop_title: Option<Box<str>> = None;
        let mut prop_label: Option<Box<str>> = None;
        let mut prop_severity: Option<Box<str>> = None;
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
            }
        }
        let name = prop_name.unwrap().to_string();
        let prop_url = format!("https://github.com/rmuir/pegon/wiki/lints#{}", name);

        let error_capture = JAVA_ERROR_QUERY.capture_index_for_name("error").unwrap();
        let node = hit.nodes_for_capture_index(error_capture).next().unwrap();

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

        // TODO: use "real" context
        if let Some(parent) = node.parent() {
            let range = parent.byte_range();
            let end = min(range.end, node.byte_range().end);
            annotations.push(AnnotationKind::Visible.span(range.start..end));
        }

        let source = str::from_utf8(data.as_slice()).unwrap();
        let severity = prop_severity.unwrap().to_string();
        let level = match severity.as_str() {
            "error" => Level::ERROR,
            "warning" => Level::WARNING,
            "info" => Level::INFO,
            "hint" => Level::NOTE,
            _ => Level::ERROR
        };
        let report = &[level
            .primary_title(prop_title.unwrap().to_string())
            .id(name)
            .id_url(prop_url)
            .element(
                Snippet::source(source)
                    //.line_start(node.range().start_point.row)
                    .path(path.to_str())
                    .annotations(annotations),
            )];
        anstream::println!("{}", RENDERER.render(report))
    }
}
