pub mod lint;

use std::{
    fs,
    process::ExitCode,
    sync::atomic::{AtomicU32, Ordering},
};

use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder, types::TypesBuilder};

use crate::lint::Linter;

static COUNT: AtomicU32 = AtomicU32::new(0);

fn main() -> ExitCode {
    let mut typesbuilder = TypesBuilder::new();
    // TODO: the default types for java are crazy and include JSP and properties
    // i guess we could format those?
    typesbuilder.add("java", "*.java").unwrap();
    typesbuilder.select("java");
    let matcher = typesbuilder.build().unwrap();
    let mut overrides = OverrideBuilder::new("/home/rmuir/workspace/lucene");
    // JFlex-generated code with escaped DFA
    overrides.add("!**/ClassicTokenizerImpl.java").unwrap();
    overrides.add("!**/HTMLStripCharFilter.java").unwrap();
    overrides.add("!**/TestJapaneseAnalyzer.java").unwrap();
    overrides.add("!**/StandardTokenizerImpl.java").unwrap();
    overrides
        .add("!**/UAX29URLEmailTokenizerImpl.java")
        .unwrap();
    overrides.add("!**/WikipediaTokenizerImpl.java").unwrap();
    overrides
        .add("!**/WordBreakTestUnicode_12_1_0.java")
        .unwrap();
    let mut builder = WalkBuilder::new("/home/rmuir/workspace/lucene");
    builder.types(matcher);
    builder.overrides(overrides.build().unwrap());

    builder.build_parallel().run(|| {
        let mut linter = Linter::new();

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
                        let errors = linter.lint(entry.path(), data);
                        if errors > 0 {
                            COUNT.fetch_add(errors, Ordering::Relaxed);
                        }
                    }
                }
                Err(err) => println!("error: {}", err),
            }
            WalkState::Continue
        })
    });
    let violations = COUNT.load(Ordering::Relaxed);
    if violations > 0 {
        println!("Found {violations} diagnostics");
        ExitCode::FAILURE
    } else {
        println!("All checks passed!");
        ExitCode::SUCCESS
    }
}
