use std::{path::Path, sync::LazyLock};
use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet, renderer::DecorStyle};
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
    if tree.root_node().has_error() {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&JAVA_ERROR_QUERY, tree.root_node(), data.as_slice());
        let source = str::from_utf8(data.as_slice()).unwrap();
        let renderer = Renderer::styled().decor_style(DecorStyle::Unicode);
        while let Some(hit) = matches.next() {
            for capture in hit.captures {
                let report = &[Level::ERROR
                    .primary_title("error")
                    .element(
                        Snippet::source(source)
                        .line_start(capture.node.range().start_point.row)
                        .path(path.to_str())
                        .annotation(AnnotationKind::Primary.span(capture.node.byte_range()).label("some more info"))
                )];
                anstream::println!("{}", renderer.render(report))
            }
        }
    }
}
