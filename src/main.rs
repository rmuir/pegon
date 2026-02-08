use std::{fs, process::ExitCode, sync::LazyLock};

use ignore::{WalkBuilder, WalkState, types::TypesBuilder};
use tree_sitter::{Node, StreamingIterator};

static JAVA_ERROR_QUERY: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
    tree_sitter::Query::new(
        &tree_sitter_java::LANGUAGE.into(),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/queries/java/lint.scm"
        )),
    )
    .unwrap()
});

fn format(tree: &tree_sitter::Tree, entry: ignore::DirEntry, data: Vec<u8>) {
    if tree.root_node().has_error() {
        println!("error found: {:?}", entry.path());
    }
    if let Ok(text) = String::from_utf8(data) {
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&JAVA_ERROR_QUERY, tree.root_node(), text.as_bytes());
        while let Some(hit) = matches.next() {
            for capture in hit.captures {
                println!("error: {:?}", capture.node.range())
            }
        }
    }
}

fn main() -> ExitCode {
    let mut typesbuilder = TypesBuilder::new();
    // TODO: the default types for java are crazy and include jsp and properties
    // i guess we could format those?
    typesbuilder.add("java", "*.java").unwrap();
    typesbuilder.select("java");
    let matcher = typesbuilder.build().unwrap();
    let mut walkbuilder = WalkBuilder::new("/home/rmuir/workspace/lucene");
    walkbuilder.types(matcher);
    walkbuilder.build_parallel().run(|| {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();

        Box::new(move |result| {
            match result {
                Ok(entry) => {
                    if let Some(filetype) = entry.file_type()
                        && filetype.is_file()
                    {
                        let data = fs::read(entry.path()).unwrap();
                        let hash = blake3::hash(data.as_slice());
                        let res = hash.to_hex().to_string();
                        if res == "foobar" {
                            println!("bogus: {}", res);
                        }
                        parser.reset();
                        let tree = parser.parse(&data, None).unwrap();
                        format(&tree, entry, data);
                    }
                }
                Err(err) => println!("error: {}", err),
            }
            WalkState::Continue
        })
    });
    ExitCode::SUCCESS
}
