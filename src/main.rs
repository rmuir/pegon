pub mod lint;

use std::{fs, process::ExitCode};

use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder, types::TypesBuilder};

fn main() -> ExitCode {
    let mut typesbuilder = TypesBuilder::new();
    // TODO: the default types for java are crazy and include jsp and properties
    // i guess we could format those?
    typesbuilder.add("java", "*.java").unwrap();
    typesbuilder.select("java");
    let matcher = typesbuilder.build().unwrap();
    let mut overrides = OverrideBuilder::new("/home/rmuir/workspace/lucene");
    // jflex-generated code with escaped DFA
    overrides.add("!**/ClassicTokenizerImpl.java").unwrap();
    overrides.add("!**/HTMLStripCharFilter.java").unwrap();
    overrides.add("!**/StandardTokenizerImpl.java").unwrap();
    overrides.add("!**/UAX29URLEmailTokenizerImpl.java").unwrap();
    overrides.add("!**/WikipediaTokenizerImpl.java").unwrap();
    let mut builder = WalkBuilder::new("/home/rmuir/workspace/lucene");
    builder.types(matcher);
    builder.overrides(overrides.build().unwrap());
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
                        lint::lint_document(&tree, entry.path(), data);
                    }
                }
                Err(err) => println!("error: {}", err),
            }
            WalkState::Continue
        })
    });
    ExitCode::SUCCESS
}
