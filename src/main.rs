pub mod cli;
pub mod lint;

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

use anyhow::Error;
use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder, types::TypesBuilder};

use crate::cli::{Commands, parse};
use crate::lint::Linter;

static COUNT: AtomicU32 = AtomicU32::new(0);

fn lint(files: &[PathBuf]) -> Result<(), Error> {
    let mut paths = files.to_vec();
    let mut typesbuilder = TypesBuilder::new();
    // TODO: the default types for java are crazy and include JSP and properties
    // i guess we could format those?
    typesbuilder.add("java", "*.java")?;
    typesbuilder.select("java");
    let matcher = typesbuilder.build()?;
    let mut overrides = OverrideBuilder::new("/home/rmuir/workspace/lucene");
    // JFlex-generated code with escaped DFA
    overrides.add("!**/ClassicTokenizerImpl.java")?;
    overrides.add("!**/HTMLStripCharFilter.java")?;
    overrides.add("!**/TestJapaneseAnalyzer.java")?;
    overrides.add("!**/StandardTokenizerImpl.java")?;
    overrides.add("!**/UAX29URLEmailTokenizerImpl.java")?;
    overrides.add("!**/WikipediaTokenizerImpl.java")?;
    overrides.add("!**/WordBreakTestUnicode_12_1_0.java")?;
    let mut builder = WalkBuilder::new(paths.pop().unwrap_or(PathBuf::from(".")));
    for remaining in paths {
        builder.add(remaining);
    }
    builder.types(matcher);
    builder.overrides(overrides.build()?);

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
                        let errors = linter.lint(entry.path(), data).unwrap();
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
        Err(anyhow::anyhow!("Found {} diagnostics", violations))
    } else {
        println!("All checks passed!");
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    let cli = parse();
    match &cli.command {
        Commands::Check { files, fix: _ } => lint(files),
        Commands::Format { files: _, check: _ } => todo!(),
    }
}
