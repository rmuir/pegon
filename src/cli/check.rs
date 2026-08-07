//! CLI "check" command
use annotate_snippets::{
    AnnotationKind, Group, Level, Renderer, Snippet,
    renderer::{Ansi256Color, DecorStyle, Style},
};
use anyhow::{Context as _, Error, bail};
use core::fmt::{Display, Formatter};
use core::sync::atomic::AtomicBool;
use crossbeam_channel::{SendError, Sender};

use ignore::{WalkBuilder, WalkState, types::TypesBuilder};
use std::{
    fs,
    io::{BufWriter, Write as _},
    path::{Path, PathBuf},
    time::Instant,
};
use tree_sitter::Parser;

use crate::support::{
    diagnostics::{self, Diagnostic, Severity, rule},
    fix::{Edit, Fix},
};

/// grey color used for context and line numbers
static GREY: Style = Ansi256Color(247).on_default();

/// cargo-style output
static FULL: Renderer = Renderer::styled()
    .decor_style(DecorStyle::Unicode)
    .context(GREY)
    .line_num(GREY);

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

#[derive(Clone, Copy, Default)]
struct Stats {
    files: usize,
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    hint_count: usize,
    fix_count: usize,
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
        self.fix_count = self.fix_count.saturating_add(other.fix_count);
    }

    const fn problem_count(&self) -> usize {
        self.error_count
            .saturating_add(self.warning_count)
            .saturating_add(self.info_count)
            .saturating_add(self.hint_count)
    }

    const fn fix_count(&self) -> usize {
        self.fix_count
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
    fix: bool,
    parser: Parser,
    sender: Sender<String>,
    stats_sender: Sender<Stats>,
    stats: Stats,
}

impl Worker {
    fn new(concise: bool, fix: bool, sender: Sender<String>, stats_sender: Sender<Stats>) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&crate::support::language())
            .expect("parser should be included in the binary");
        Self {
            concise,
            fix,
            parser,
            sender,
            stats_sender,
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

                if shouldcheck && let Err(error) = self.handle_file(path) {
                    if error.downcast_ref::<SendError<String>>().is_some() {
                        return WalkState::Quit;
                    }
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

    fn handle_file(&mut self, path: &Path) -> Result<(), Error> {
        if self.fix {
            self.fix_file(path)
        } else {
            self.check_file(path)
        }
    }

    /// check a file without applying fixes
    fn check_file(&mut self, path: &Path) -> Result<(), Error> {
        let data = fs::read(path)?;
        self.parser.reset();
        let tree = self
            .parser
            .parse(&data, None)
            .context("parser should be setup")?;
        let diagnostics = diagnostics::lint(&tree, &data, &AtomicBool::new(false), !self.concise)?;
        self.finish_file(path, &data, &diagnostics)
    }

    /// check and apply autofixes for a file
    ///
    /// apply edits in backwards order.
    /// iteratively re-parse and re-diagnose if anything was fixed or if any fixes intersect
    fn fix_file(&mut self, path: &Path) -> Result<(), Error> {
        // TODO: incremental tree-sitter parsing for the loop?
        let mut data = fs::read(path)?;
        let mut fix_count: usize = 0;
        let mut diagnostics = vec![];
        // place a bound on iterations
        for _ in 1..10 {
            self.parser.reset();
            let tree = self
                .parser
                .parse(&data, None)
                .context("parser should be setup")?;
            diagnostics = diagnostics::lint(&tree, &data, &AtomicBool::new(false), !self.concise)?;

            // we're done if file has no problems
            if diagnostics.is_empty() {
                break;
            }

            let mut edits = Fix::batch(&diagnostics, &tree, &data)?;

            // we're done if there are no edits
            if edits.is_empty() {
                break;
            }

            // do we have fixes for all our problems?
            let mut all_fixed = edits.len() == diagnostics.len();

            // deduplicate edits (can happen easily for e.g. out of order imports)
            edits.dedup();

            // apply the edits in memory
            let mut previous = None;
            for edit in edits {
                // if we intersect with previous edit, we'll iterate again
                if previous
                    .as_ref()
                    .is_some_and(|previous| Edit::intersects(&edit.range, previous))
                {
                    all_fixed = false;
                    continue;
                }
                data.splice(edit.range.clone(), edit.replacement.into_bytes());
                previous = Some(edit.range);
                fix_count = fix_count.saturating_add(1);
            }
            // stop iterating if we've fixed it all, there's no reason to re-parse
            if all_fixed {
                diagnostics = vec![];
                break;
            }
        }
        // if we applied any edits, write the buffer back
        if fix_count > 0 {
            self.stats.fix_count = self.stats.fix_count.saturating_add(fix_count);
            fs::write(path, &data)?;
        }
        self.finish_file(path, &data, &diagnostics)
    }

    // accumulates stats and writes output for a file
    fn finish_file(
        &mut self,
        path: &Path,
        data: &[u8],
        result: &[Diagnostic],
    ) -> Result<(), Error> {
        self.stats.add_file(1);
        if !result.is_empty() {
            for item in result.iter().as_ref() {
                self.stats.add_problem(rule(item.rule_id).severity);
            }
            if self.concise {
                self.render_concise(path, result)?;
            } else {
                self.render_full(path, data, result)?;
            }
        }
        Ok(())
    }

    /// Render some diagnostics to the console
    #[expect(clippy::arithmetic_side_effects, reason = "TODO")]
    fn render_full(&self, path: &Path, data: &[u8], errors: &[Diagnostic]) -> Result<(), Error> {
        let filename = path.to_str();
        let source = str::from_utf8(data)?;
        for diagnostic in errors {
            let rule = rule(diagnostic.rule_id);
            let id_url = &rule.url;
            let label = diagnostic.label.as_ref();
            let bounds = diagnostic.bounds(source);
            let offset = bounds.range.start;

            let annotations = [
                // top context: e.g. what function are you in
                diagnostic.top_context.map(|ctx| {
                    AnnotationKind::Visible.span(ctx.start_byte - offset..ctx.end_byte - offset)
                }),
                // primary error annotation: as precise of a range as possible
                Some(
                    AnnotationKind::Primary
                        .span(
                            diagnostic.range.start_byte - offset
                                ..diagnostic.range.end_byte - offset,
                        )
                        .label(label)
                        .highlight_source(true),
                ),
                // explicitly marked context in the query
                diagnostic.context.map(|context| {
                    AnnotationKind::Context
                        .span(context.start_byte - offset..context.end_byte - offset)
                        .label(rule.context_label.as_ref())
                }),
                // explicitly marked visible in the query
                diagnostic.visible.map(|visible| {
                    AnnotationKind::Visible
                        .span(visible.start_byte - offset..visible.end_byte - offset)
                }),
            ];

            let level: Level = rule.severity.into();
            // just show if a fix is available, the diffs can get enormous
            let help_name = if rule.fix.is_some() {
                "help (fix available)"
            } else {
                "help"
            };

            let report = [
                level
                    .with_name(rule.severity.as_str())
                    .primary_title(&diagnostic.title)
                    .id(&rule.name)
                    .id_url(id_url)
                    .element(
                        Snippet::source(source.get(bounds.range).context("valid bounds")?)
                            .path(filename)
                            .line_start(bounds.line_start + 1)
                            .annotations(annotations.into_iter().flatten()),
                    ),
                Group::with_title(
                    Level::NOTE
                        .with_name(help_name)
                        .secondary_title(&diagnostic.help),
                ),
            ];
            let mut message = FULL.render(&report);
            message.push_str("\n\n");
            self.sender.send(message)?;
        }
        Ok(())
    }

    // fastest and easier to not use annotate-snippets for this
    fn render_concise(&self, path: &Path, errors: &[Diagnostic]) -> Result<(), Error> {
        let filename = path.to_string_lossy();
        for diagnostic in errors {
            let rule = rule(diagnostic.rule_id);
            let line = diagnostic
                .range
                .start_point
                .row
                .checked_add(1)
                .context("no overflow")?;
            let column = diagnostic
                .range
                .start_point
                .column
                .checked_add(1)
                .context("no overflow")?;
            let severity = rule.severity.as_str();
            let title = &diagnostic.title;
            let id = &rule.name;
            let message = format!("{filename}:{line}:{column}: {severity}[{id}]: {title}\n");
            self.sender.send(message)?;
        }
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        _ = self.stats_sender.send(self.stats);
    }
}

/// Check the set of files
///
/// # Errors
///
/// Returns an error if any files had problems, or if internal errors were encountered.
pub fn check(inputs: &[PathBuf], concise: bool, fix: bool) -> Result<(), Error> {
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

    // buffer diagnostics with crossbeam so the threads don't lock each other on printing
    let (tx, rx) = crossbeam_channel::bounded::<String>(1024);
    let messages = std::thread::spawn(move || -> Result<(), Error> {
        let mut writer = BufWriter::new(anstream::stdout().lock());
        for diagnostic in rx {
            writer.write_all(diagnostic.as_bytes())?;
        }
        Ok(())
    });

    let (stats_tx, stats_rx) = crossbeam_channel::unbounded();
    builder.build_parallel().run(|| {
        let mut worker = Worker::new(concise, fix, tx.clone(), stats_tx.clone());
        Box::new(move |result| worker.visit(result))
    });

    // finish writing diagnostics
    drop(tx);
    messages.join().map_err(|err| {
        drop(err); // not worth the trouble
        anyhow::anyhow!("message thread panicked")
    })??;

    // write stats
    drop(stats_tx);
    let mut stats = Stats::default();
    for result in stats_rx {
        stats.add(result);
    }

    let files = stats.files;
    let problem_count = stats.problem_count();
    let fix_count = stats.fix_count();

    let elapsed = start_time.elapsed();
    let millis = elapsed.as_millis();

    if problem_count > 0 && fix_count > 0 {
        bail!(
            "{problem_count} problems remain in {files} files / {millis} ms [{stats}] [{fix_count} fixed]"
        );
    }
    if problem_count > 0 {
        bail!("Found {problem_count} problems in {files} files / {millis} ms [{stats}]");
    }
    if files == 0 {
        bail!("Found no java files to check");
    }
    if fix_count > 0 {
        eprintln!("Success: No problems remain in {files} files / {millis} ms [{fix_count} fixed]");
    } else {
        eprintln!("Success: No problems found in {files} files / {millis} ms");
    }
    Ok(())
}
