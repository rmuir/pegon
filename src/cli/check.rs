//! CLI "check" command
use annotate_snippets::{
    AnnotationKind, Group, Level, Patch, Renderer, Snippet,
    renderer::{Ansi256Color, DecorStyle, Style},
};
use anyhow::{Context as _, Error, bail};
use core::fmt::{Display, Formatter};
use core::sync::atomic::AtomicBool;

use ignore::{WalkBuilder, WalkState, types::TypesBuilder};
use std::{
    fs,
    io::{BufWriter, Write as _},
    path::{Path, PathBuf},
    sync::{Arc, mpsc::Sender},
    time::Instant,
};
use tree_sitter::Parser;

use crate::support::diagnostics::{self, Diagnostic, Fix, Severity, rule};

/// grey color used for context and line numbers
static GREY: Style = Ansi256Color(247).on_default();

/// cargo-style output
static FULL: Renderer = Renderer::styled()
    .decor_style(DecorStyle::Unicode)
    .context(GREY)
    .line_num(GREY);

/// gcc-style output
static CONCISE: Renderer = Renderer::plain().short_message(true);

/// map severity levels to annotate-snippets severities
impl From<Severity> for Level<'_> {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Error => Self::ERROR,
            Severity::Warn => Self::WARNING,
            Severity::Info => Self::INFO,
            Severity::Hint => Self::HELP,
        }
    }
}

/// Render some diagnostics to the console
fn render(path: &Path, data: &[u8], errors: &[Diagnostic], concise: bool) -> Result<(), Error> {
    let filename = path.to_str();
    let source = str::from_utf8(data)?;
    let lock = anstream::stdout().lock();
    let mut writer = BufWriter::new(lock);
    for diagnostic in errors {
        let rule = rule(diagnostic.rule_id);
        let id_url = if concise { "" } else { &rule.url };
        let label = if concise {
            None
        } else {
            diagnostic.label.as_ref()
        };

        let annotations = [
            // top context: e.g. what function are you in
            diagnostic
                .top_context
                .map(|ctx| AnnotationKind::Visible.span(ctx.start_byte..ctx.end_byte)),
            // primary error annotation: as precise of a range as possible
            Some(
                AnnotationKind::Primary
                    .span(diagnostic.range.start_byte..diagnostic.range.end_byte)
                    .label(label)
                    .highlight_source(true),
            ),
            // explicitly marked context in the query
            diagnostic.context.map(|context| {
                AnnotationKind::Context
                    .span(context.start_byte..context.end_byte)
                    .label(rule.context_label.as_ref())
            }),
            // explicitly marked visible in the query
            diagnostic
                .visible
                .map(|visible| AnnotationKind::Visible.span(visible.start_byte..visible.end_byte)),
        ];

        let level: Level = rule.severity.into();

        let report = [
            level
                .with_name(rule.severity.as_str())
                .primary_title(&diagnostic.title)
                .id(&rule.name)
                .id_url(id_url)
                .element(
                    Snippet::source(source)
                        .path(filename)
                        .annotations(annotations.into_iter().flatten()),
                ),
            match &rule.fix {
                Some(Fix::Static(replacement)) => Level::NOTE
                    .with_name("help")
                    .secondary_title(&diagnostic.help)
                    .element(Snippet::source(source).patch(Patch::new(
                        diagnostic.range.start_byte..diagnostic.range.end_byte,
                        replacement,
                    ))),
                _ => Group::with_title(
                    Level::NOTE
                        .with_name("help")
                        .secondary_title(&diagnostic.help),
                ),
            },
        ];
        if concise {
            writeln!(writer, "{}", CONCISE.render(&report))?;
        } else {
            writeln!(writer, "{}\n", FULL.render(&report))?;
        }
    }
    writer.flush()?;
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Stats {
    files: usize,
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    hint_count: usize,
}

impl Stats {
    const fn add_file(&mut self, count: usize) {
        self.files = self.files.saturating_add(count);
    }
    const fn add_problem(&mut self, severity: Severity) {
        match severity {
            Severity::Error => self.error_count = self.error_count.saturating_add(1),
            Severity::Warn => self.warning_count = self.warning_count.saturating_add(1),
            Severity::Info => self.info_count = self.info_count.saturating_add(1),
            Severity::Hint => self.hint_count = self.hint_count.saturating_add(1),
        }
    }
    const fn add(&mut self, other: Self) {
        self.add_file(other.files);
        self.error_count = self.error_count.saturating_add(other.error_count);
        self.warning_count = self.warning_count.saturating_add(other.warning_count);
        self.info_count = self.info_count.saturating_add(other.info_count);
        self.hint_count = self.hint_count.saturating_add(other.hint_count);
    }

    const fn problem_count(&self) -> usize {
        self.error_count
            .saturating_add(self.warning_count)
            .saturating_add(self.info_count)
            .saturating_add(self.hint_count)
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Error:{} Warning:{} Info:{} Hint:{}",
            self.error_count, self.warning_count, self.info_count, self.hint_count
        )
    }
}

struct Worker {
    concise: bool,
    parser: Parser,
    sender: Sender<Stats>,
    stats: Stats,
}

impl Worker {
    fn new(concise: bool, sender: Sender<Stats>) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&crate::support::language())
            .expect("parser should be included in the binary");
        Self {
            concise,
            parser,
            sender,
            stats: Stats::default(),
        }
    }

    fn visit(&mut self, result: Result<ignore::DirEntry, ignore::Error>) -> WalkState {
        match result {
            Ok(entry) => {
                let shouldcheck = entry.file_type().is_none_or(|filetype| !filetype.is_dir());
                let path = if entry.is_stdin() {
                    // TODO
                    Path::new("/dev/stdin")
                } else {
                    entry.path()
                };

                if shouldcheck && let Err(error) = self.check_file(path) {
                    let filename = entry.path().to_string_lossy();
                    eprintln!("internal error: {filename} {error}");
                    self.stats.add_problem(Severity::Error);
                }
                WalkState::Continue
            }
            Err(err) => {
                eprintln!("file error: {err}");
                self.stats.add_problem(Severity::Error);
                WalkState::Skip
            }
        }
    }

    fn check_file(&mut self, path: &Path) -> Result<(), Error> {
        let data = fs::read(path)?;
        self.parser.reset();
        let tree = self
            .parser
            .parse(&data, None)
            .context("parser should be setup")?;
        let result = diagnostics::lint(&tree, &data, &Arc::new(AtomicBool::new(false)), true)?;
        if !result.is_empty() {
            for item in result.iter().as_ref() {
                self.stats.add_problem(rule(item.rule_id).severity);
            }
            render(path, &data, &result, self.concise)?;
        }
        self.stats.add_file(1);
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        _ = self.sender.send(self.stats);
    }
}

/// Check the set of files
///
/// # Errors
///
/// Returns an error if any files had problems, or if internal errors were encountered.
pub fn check(inputs: &[PathBuf], concise: bool) -> Result<(), Error> {
    let start_time = Instant::now();
    let mut paths = inputs.to_vec();
    let mut typesbuilder = TypesBuilder::new();
    // TODO: the default types for java are crazy and include JSP and properties
    // i guess we could format those?
    typesbuilder.add("java", "*.java")?;
    typesbuilder.select("java");
    let matcher = typesbuilder.build()?;
    let first_path = paths.pop().unwrap_or_else(|| PathBuf::from("."));
    let mut builder = WalkBuilder::new(first_path.as_path());
    for remaining in paths {
        builder.add(remaining);
    }
    builder.types(matcher);
    if let Some(overrides) = super::generated::generated_files(first_path.as_path())? {
        builder.overrides(overrides);
    }

    let (tx, rx) = std::sync::mpsc::channel();
    builder.build_parallel().run(|| {
        let mut worker = Worker::new(concise, tx.clone());
        Box::new(move |result| worker.visit(result))
    });
    drop(tx);

    let mut stats = Stats::default();
    for result in rx {
        stats.add(result);
    }

    let files = stats.files;
    let problem_count = stats.problem_count();

    let elapsed = start_time.elapsed();
    let millis = elapsed.as_millis();

    if problem_count > 0 {
        bail!("Found {problem_count} problems across {files} java files in {millis} ms [{stats}]");
    }
    if files == 0 {
        bail!("Found no java files to check");
    }
    println!("Success: No problems found across {files} java files in {millis} ms");
    Ok(())
}
