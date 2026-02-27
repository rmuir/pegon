pub mod cli;
pub mod console;
pub mod lint;
pub mod lsp;

use core::net::Ipv4Addr;
use core::sync::atomic::{AtomicUsize, Ordering};

use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context as _, Error, bail};
use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder, types::TypesBuilder};
use lsp_server::Connection;
use tree_sitter::Parser;

use crate::cli::{Commands, parse};
use crate::lint::lint;

static FILES: AtomicUsize = AtomicUsize::new(0);
static ERRORS: AtomicUsize = AtomicUsize::new(0);
static INTERNAL_ERRORS: AtomicUsize = AtomicUsize::new(0);

fn check_file(parser: &mut Parser, path: &Path) -> Result<(), Error> {
    let data = fs::read(path)?;
    let hash = blake3::hash(data.as_slice());
    #[expect(unused_variables, reason = "TODO: needs cache impl")]
    let res = hash.to_hex().to_string();
    parser.reset();
    let tree = parser
        .parse(&data, None)
        .context("parser should be setup")?;
    let result = lint(&tree, &data)?;
    if !result.is_empty() {
        ERRORS.fetch_add(result.len(), Ordering::Relaxed);
        console::render(path, &data, result)?;
    }
    FILES.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

fn check(inputs: &[PathBuf]) -> Result<(), Error> {
    let start_time = Instant::now();
    let mut paths = inputs.to_vec();
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
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .expect("parser should be included in the binary");

        Box::new(move |result| {
            match result {
                Ok(entry) => {
                    if let Some(filetype) = entry.file_type()
                        && !filetype.is_dir()
                        && let Err(error) = check_file(&mut parser, entry.path())
                    {
                        let filename = entry.path().to_string_lossy();
                        eprintln!("internal error: {filename} {error}");
                        INTERNAL_ERRORS.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(err) => {
                    eprintln!("file error: {err}");
                    INTERNAL_ERRORS.fetch_add(1, Ordering::Relaxed);
                }
            }
            WalkState::Continue
        })
    });

    let errors = ERRORS.load(Ordering::Relaxed);
    let files = FILES.load(Ordering::Relaxed);
    let elapsed = start_time.elapsed();
    let millis = elapsed.as_millis();

    if errors > 0 {
        bail!("Found {errors} problems across {files} java files in {millis} ms");
    } else if files == 0 {
        bail!("Found no java files to check");
    }
    println!("Success: No problems found across {files} java files in {millis} ms");
    Ok(())
}

fn main() -> Result<(), Error> {
    let cli = parse();
    match &cli.command {
        Commands::Check { files, fix: _ } => check(files),
        Commands::Format { files: _, check: _ } => todo!(),
        Commands::Server { socket: None, .. } => {
            let (connection, iothreads) = Connection::stdio();
            let result = lsp::start(connection);
            iothreads.join()?;
            result
        }
        Commands::Server {
            socket: Some(port), ..
        } => {
            let addr = (Ipv4Addr::LOCALHOST, *port);
            let (connection, iothreads) = Connection::listen(addr)?;
            let result = lsp::start(connection);
            iothreads.join()?;
            result
        }
    }
}
