use core::net::Ipv4Addr;
use core::sync::atomic::{AtomicUsize, Ordering};

use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context as _, Error, bail};
use clap::Parser as _;
use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder, types::TypesBuilder};
use lsp_server::Connection;
use tree_sitter::Parser;

use pegon::cli;
use pegon::diagnostics;
use pegon::lsp;

static FILES: AtomicUsize = AtomicUsize::new(0);
static ERRORS: AtomicUsize = AtomicUsize::new(0);
static INTERNAL_ERRORS: AtomicUsize = AtomicUsize::new(0);

fn check_file(parser: &mut Parser, path: &Path, concise: bool) -> Result<(), Error> {
    let data = fs::read(path)?;
    parser.reset();
    let tree = parser
        .parse(&data, None)
        .context("parser should be setup")?;
    let result = diagnostics::lint(&tree, &data)?;
    if !result.is_empty() {
        ERRORS.fetch_add(result.len(), Ordering::Relaxed);
        cli::console::render(path, &data, result, concise)?;
    }
    FILES.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

fn check(inputs: &[PathBuf], concise: bool) -> Result<(), Error> {
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
            .set_language(&pegon::LANGUAGE.into())
            .expect("parser should be included in the binary");

        Box::new(move |result| {
            match result {
                Ok(entry) => {
                    let shouldcheck = entry.file_type().is_none_or(|filetype| !filetype.is_dir());
                    let path = if entry.is_stdin() {
                        // TODO
                        Path::new("/dev/stdin")
                    } else {
                        entry.path()
                    };

                    if shouldcheck && let Err(error) = check_file(&mut parser, path, concise) {
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
    let options = cli::Cli::parse();
    match &options.command {
        cli::Commands::Check {
            files,
            fix: _,
            output_format,
        } => check(files, *output_format == cli::OutputFormat::Concise),
        cli::Commands::Server { socket: None, .. } => {
            let (connection, iothreads) = Connection::stdio();
            let result = lsp::start(connection);
            iothreads.join()?;
            result
        }
        cli::Commands::Server {
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
