//! CLI "format" command
use anyhow::Result;
use std::path::PathBuf;

use anyhow::{Context as _, Error, bail};
use core::sync::atomic::AtomicBool;
use crossbeam_channel::{SendError, Sender};

use ignore::{DirEntry, WalkBuilder, WalkState, types::TypesBuilder};
use std::{
    fs,
    io::{Read as _, Write as _},
    time::Instant,
};
use tree_sitter::Parser;

use crate::support::formatting;

#[derive(Clone, Copy, Default)]
struct Stats {
    files: usize,
    error_count: usize,
    fix_count: usize,
}

impl Stats {
    const fn add_file(&mut self, count: usize) {
        self.files = self.files.saturating_add(count);
    }

    const fn add_problem(&mut self) {
        self.error_count = self.error_count.saturating_add(1);
    }

    const fn add(&mut self, other: Self) {
        self.add_file(other.files);
        self.error_count = self.error_count.saturating_add(other.error_count);
        self.fix_count = self.fix_count.saturating_add(other.fix_count);
    }
}

struct Worker {
    verify: bool,
    parser: Parser,
    stats_sender: Sender<Stats>,
    stats: Stats,
}

impl Worker {
    fn new(verify: bool, stats_sender: Sender<Stats>) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&crate::support::LANGUAGE)
            .expect("parser should be included in the binary");
        Self {
            verify,
            parser,
            stats_sender,
            stats: Stats::default(),
        }
    }

    fn visit(&mut self, result: Result<ignore::DirEntry, ignore::Error>) -> WalkState {
        match result {
            Ok(entry) => {
                let shouldcheck = entry.file_type().is_none_or(|filetype| !filetype.is_dir());
                if shouldcheck && let Err(error) = self.handle_file(&entry) {
                    if error.downcast_ref::<SendError<String>>().is_some() {
                        return WalkState::Quit;
                    }
                    let filename = entry.path().to_string_lossy();
                    eprintln!("internal error: {filename} {error}");
                    self.stats.add_problem();
                }
                WalkState::Continue
            }
            Err(err) => {
                eprintln!("file error: {err}");
                self.stats.add_problem();
                WalkState::Skip
            }
        }
    }

    /// Reads file (or stdin) contents
    fn read_bytes(entry: &DirEntry) -> Result<Vec<u8>, Error> {
        if entry.is_stdin() {
            let mut buffer = vec![];
            std::io::stdin().read_to_end(&mut buffer)?;
            Ok(buffer)
        } else {
            Ok(fs::read(entry.path())?)
        }
    }

    /// Reads file (or stdout) contents
    fn write_bytes(entry: &DirEntry, data: &[u8]) -> Result<(), Error> {
        if entry.is_stdin() {
            Ok(std::io::stdout().write_all(data)?)
        } else {
            Ok(fs::write(entry.path(), data)?)
        }
    }

    /// apply formatting to a file
    fn handle_file(&mut self, entry: &DirEntry) -> Result<(), Error> {
        let data = Self::read_bytes(entry)?;
        let mut buffer = Vec::with_capacity(data.len());
        self.parser.reset();
        let tree = self
            .parser
            .parse(&data, None)
            .context("parser should be setup")?;
        // differentiate these from real errors
        if tree.root_node().has_error() {
            return Ok(());
        }
        formatting::format(&tree, &data, &mut buffer, &AtomicBool::new(false))?;
        if data != buffer {
            if self.verify {
                self.parser.reset();
                let tree2 = self
                    .parser
                    .parse(&buffer, None)
                    .context("parser should be setup")?;
                let mut buffer2 = Vec::with_capacity(buffer.len());
                formatting::format(&tree2, &buffer, &mut buffer2, &AtomicBool::new(false))
                    .context("verify: parsing check failed")?;
                if buffer != buffer2 {
                    bail!("verify: idempotency check failed");
                }
            }
            self.stats.fix_count = self.stats.fix_count.saturating_add(1);
            Self::write_bytes(entry, &buffer)?;
        }
        self.stats.add_file(1);
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        _ = self.stats_sender.send(self.stats);
    }
}

/// Formats the set of files
///
/// # Errors
///
/// Returns an error if any files had problems, or if internal errors were encountered.
pub fn format(inputs: &[PathBuf], verify: bool) -> Result<(), Error> {
    let start_time = Instant::now();
    let mut typesbuilder = TypesBuilder::new();
    // TODO: the default types for java are crazy and include JSP and properties
    // i guess we could format those?
    typesbuilder.add("java", "*.java")?;
    typesbuilder.select("java");
    let matcher = typesbuilder.build()?;

    // create overrides to ignore generated files
    // paths passed on cmdline (e.g. pre-commit) must be explicitly filtered with it.
    let default_roots = [PathBuf::from(".")];
    let roots = if inputs.is_empty() {
        &default_roots
    } else {
        inputs
    };
    let overrides = super::generated::generated_files(roots.first().context("not empty")?)?;
    let mut builder = WalkBuilder::from_iter(roots.iter().filter(|item| {
        overrides.as_ref().is_none_or(|overrides| {
            !matches!(
                overrides.matched(item, item.is_dir()),
                ignore::Match::Ignore(_)
            )
        })
    }));
    builder.types(matcher);
    if let Some(overrides) = overrides {
        builder.overrides(overrides);
    }

    let (stats_tx, stats_rx) = crossbeam_channel::unbounded();
    builder.build_parallel().run(|| {
        let mut worker = Worker::new(verify, stats_tx.clone());
        Box::new(move |result| worker.visit(result))
    });

    // write stats
    drop(stats_tx);
    let mut stats = Stats::default();
    for result in stats_rx {
        stats.add(result);
    }

    let files = stats.files;
    let fix_count = stats.fix_count;
    let err_count = stats.error_count;

    let elapsed = start_time.elapsed();
    let millis = elapsed.as_millis();

    if files == 0 {
        if err_count > 0 {
            bail!("Found no java files to check [{err_count} errors]");
        }
        bail!("Found no java files to check");
    }
    if err_count > 0 {
        if fix_count > 0 {
            bail!("{files} files left unchanged in {millis} ms [{err_count} errors]");
        }
        bail!("{files} files formatted in {millis} ms [{fix_count} changed, {err_count} errors]");
    }

    if fix_count > 0 {
        eprintln!("Success: {files} files formatted in {millis} ms [{fix_count} changed]");
    } else {
        eprintln!("Success: {files} files left unchanged in {millis} ms");
    }
    Ok(())
}
