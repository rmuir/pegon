pub mod formatting;

use std::{fs, process::ExitCode};

use ignore::{WalkBuilder, WalkState, types::TypesBuilder};

fn main() -> ExitCode {
    let mut typesbuilder = TypesBuilder::new();
    // TODO: the default types for java are crazy and include jsp and properties
    // i guess we could format those?
    typesbuilder.add("java", "*.java").unwrap();
    typesbuilder.select("java");
    let matcher = typesbuilder.build().unwrap();
    let mut builder = WalkBuilder::new("/home/rmuir/workspace/lucene");
    builder.types(matcher);
    builder.build_parallel().run(|| {
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
                        formatting::format(&tree, entry.path(), data);
                    }
                }
                Err(err) => println!("error: {}", err),
            }
            WalkState::Continue
        })
    });
    ExitCode::SUCCESS
}
