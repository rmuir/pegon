//! Index of the workspace sources
//!
//! Currently only handles java source files.
//! Does not use tree-sitter parser, instead just parses minimally

use std::{
    mem::take,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Error};
use bstr::ByteSlice as _;
use crossbeam_channel::Sender;
use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder, types::TypesBuilder};
use regex_automata::{
    dfa::onepass::{Cache, DFA},
    util::captures::Captures,
};
use rustc_hash::FxHashMap;

#[derive(Default)]
pub struct Index {
    /// fully qualified name -> path name
    names: FxHashMap<String, String>,
}

/// per thread worker
struct Worker<'scope> {
    parser: &'scope DFA,
    sender: Sender<Index>,
    cache: Cache,
    captures: Captures,
    index: Index,
}

impl<'scope> Worker<'scope> {
    fn new(parser: &'scope DFA, sender: Sender<Index>) -> Self {
        Self {
            parser,
            sender,
            cache: parser.create_cache(),
            captures: parser.create_captures(),
            index: Index::default(),
        }
    }

    /// visit an individual file being walked
    fn visit(&mut self, result: Result<ignore::DirEntry, ignore::Error>) -> WalkState {
        match result {
            Ok(entry) => {
                let shouldcheck = entry.file_type().is_none_or(|filetype| !filetype.is_dir());
                let path = entry.path();
                if shouldcheck && let Err(error) = self.analyze(path) {
                    let filename = path.to_string_lossy();
                    eprintln!("internal error: {filename} {error}");
                }
                WalkState::Continue
            }
            Err(err) => {
                eprintln!("file error: {err}");
                WalkState::Skip
            }
        }
    }

    /// parse the package declaration and combine with the filename
    ///
    /// doesn't do anything for files without package declarations
    fn analyze(&mut self, path: &Path) -> Result<(), Error> {
        let bytes = std::fs::read(path)?;
        for line in bytes.lines() {
            self.parser
                .captures(&mut self.cache, line, &mut self.captures);
            if self.captures.is_match()
                && let Some(span) = self.captures.get_group(1)
            {
                let slice = line.get(span.start..span.end).context("should exist")?;
                let package = str::from_utf8(slice)?;
                let class = path
                    .file_stem()
                    .context("should be a file")?
                    .to_string_lossy();
                let path = path.to_string_lossy().into_owned();
                self.index.names.insert(format!("{package}.{class}"), path);
                break; // currently, we don't want anything else from this file
            }
        }
        Ok(())
    }
}

/// send our sub-index back to be merged
impl Drop for Worker<'_> {
    fn drop(&mut self) {
        let index = take(&mut self.index);
        _ = self.sender.send(index);
    }
}

// parses a package declaration only, captures the actual package name
// there can't be any annotations, we ignore package-info.java explicitly
// it can't be line-wrapped according to google style
const PACKAGE_DECLARATION: &str = r"(?-u)^\s*package\s+([a-zA-Z0-9_.]+)\s*;";

/// index a workspace.
///
/// it might have multiple paths if we parse .classpath or something
#[expect(unused, reason = "not yet")]
pub fn index(inputs: &[PathBuf]) -> Result<Index, Error> {
    let mut typesbuilder = TypesBuilder::new();
    typesbuilder.add("java", "*.java")?;
    typesbuilder.select("java");

    // ignore package-info.java files as they have nothing to offer
    // though, if everyone consistently used them, this whole thing would be faster...
    let mut overridesbuilder = OverrideBuilder::new("/");
    overridesbuilder.add("!**/package-info.java")?;

    let mut builder = WalkBuilder::from_iter(inputs.iter());
    builder.types(typesbuilder.build()?);
    builder.overrides(overridesbuilder.build()?);

    let parser = DFA::new(PACKAGE_DECLARATION)?;

    let (tx, rx) = crossbeam_channel::unbounded();
    builder.build_parallel().run(|| {
        let mut worker = Worker::new(&parser, tx.clone());
        Box::new(move |result| worker.visit(result))
    });

    drop(tx);
    let mut index = Index::default();
    for shard in rx {
        index.names.extend(shard.names);
    }
    Ok(index)
}
