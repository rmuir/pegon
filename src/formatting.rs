use annotate_snippets::{
    Annotation, AnnotationKind, Level, Renderer, Snippet, renderer::DecorStyle,
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

pub fn format(tree: &Tree, path: &Path, data: Vec<u8>) {
    if true || tree.root_node().has_error() {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&JAVA_ERROR_QUERY, tree.root_node(), data.as_slice());
        let source = str::from_utf8(data.as_slice()).unwrap();
        let renderer = Renderer::styled().decor_style(DecorStyle::Unicode);
        while let Some(hit) = matches.next() {
            for capture in hit.captures {
                let mut annotations: Vec<Annotation> = Vec::new();
                annotations.push(
                    AnnotationKind::Primary
                        .span(capture.node.byte_range())
                        .label("label")
                        .highlight_source(true)
                );
                if let Some(parent) = capture.node.parent() {
                    let range = parent.byte_range();
                    let end = min(range.end, capture.node.byte_range().end);
                    annotations.push(
                        AnnotationKind::Visible
                            .span(range.start ..end)
                    );
                }

                let report = &[Level::ERROR
                    .with_name(Some("name"))
                    .primary_title("title")
                    .id("ECODE")
                    .id_url("https://some-description.somewhere.org/codes/ECODE.html")
                    .element(
                        Snippet::source(source)
                            .line_start(capture.node.range().start_point.row)
                            .path(path.to_str())
                            .annotations(annotations)
                    )];
                anstream::println!("{}", renderer.render(report))
            }
        }
    }
}
