pub mod cli;
pub mod console;
pub mod lint;
pub mod lsp;

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use anyhow::Error;
use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder, types::TypesBuilder};

use crate::cli::{Commands, parse};
use crate::lint::lint;

static FILES: AtomicUsize = AtomicUsize::new(0);
static ERRORS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static INTERNAL_ERRORS: AtomicUsize = AtomicUsize::new(0);

fn check(files: &[PathBuf]) -> Result<(), Error> {
    let start_time = Instant::now();
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
    let mut builder = WalkBuilder::new(paths.pop().unwrap_or_else(|| PathBuf::from(".")));
    for remaining in paths {
        builder.add(remaining);
    }
    builder.types(matcher);
    builder.overrides(overrides.build()?);

    // TODO: use parallelvisitor builder
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
                        BYTES.fetch_add(data.len(), Ordering::Relaxed);
                        let hash = blake3::hash(data.as_slice());
                        let res = hash.to_hex().to_string();
                        if res == "foobar" {
                            println!("bogus: {res}");
                        }
                        parser.reset();
                        let tree = parser.parse(&data, None).unwrap();
                        let result = lint(&tree, &data);
                        match result {
                            Ok(errors) => {
                                FILES.fetch_add(1, Ordering::Relaxed);
                                if !errors.is_empty() {
                                    ERRORS.fetch_add(errors.len(), Ordering::Relaxed);
                                    console::render(entry.path(), &data, errors).unwrap(); // TODO
                                }
                            }
                            Err(error) => {
                                eprintln!(
                                    "internal error processing {}: {}",
                                    entry.path().to_string_lossy(),
                                    error
                                );
                                INTERNAL_ERRORS.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("internal error: {err}");
                    INTERNAL_ERRORS.fetch_add(1, Ordering::Relaxed);
                }
            }
            WalkState::Continue
        })
    });

    let errors = ERRORS.load(Ordering::Relaxed);
    let files = FILES.load(Ordering::Relaxed);
    let bytes = BYTES.load(Ordering::Relaxed);
    let elapsed = start_time.elapsed();
    let millis = elapsed.as_millis();
    #[allow(clippy::cast_precision_loss)]
    let speed = (bytes as f64 / 1_000_000.0) / elapsed.as_secs_f64();

    if errors > 0 {
        Err(anyhow::anyhow!(
            "Found {errors} problems across {files} java files in {millis} ms ({speed:.1} MB/s)"
        ))
    } else if files == 0 {
        Err(anyhow::anyhow!("Found no java files to check"))
    } else {
        println!(
            "Success: no problems found across {files} java files in {millis} ms ({speed:.1} MB/s)"
        );
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    let cli = parse();
    match &cli.command {
        Commands::Check { files, fix: _ } => check(files),
        Commands::Format { files: _, check: _ } => todo!(),
        Commands::Server => lsp::main(),
    }
}
