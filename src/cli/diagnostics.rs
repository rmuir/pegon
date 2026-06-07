use annotate_snippets::{
    Annotation, AnnotationKind, Group, Level, Patch, Renderer, Snippet,
    renderer::{Ansi256Color, DecorStyle, Style},
};
use anyhow::{Context as _, Error, bail};
use core::fmt::{Display, Formatter};

use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder, types::TypesBuilder};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    time::Instant,
};
use tree_sitter::Parser;

use crate::diagnostics::{self, Diagnostic, Severity, rule};

/// grey color used for context and line numbers
static GREY: Style = Ansi256Color(247).on_default();

/// cargo-style output
static FULL: Renderer = Renderer::styled()
    .decor_style(DecorStyle::Unicode)
    .context(GREY)
    .line_num(GREY);

/// gcc-style output
static CONCISE: Renderer = Renderer::plain().short_message(true);

/// display severity levels
impl Display for Severity {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match *self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Hint => write!(f, "hint"),
        }
    }
}

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

/// Render any diagnostics to the console
fn render(path: &Path, data: &[u8], errors: Vec<Diagnostic>, concise: bool) -> Result<(), Error> {
    if errors.is_empty() {
        return Ok(());
    }
    let source = str::from_utf8(data)?;
    for diagnostic in errors {
        let mut annotations: Vec<Annotation> = Vec::new();
        let rule = rule(diagnostic.rule_id);

        let label = if concise { None } else { diagnostic.label };
        let id_url = if concise { "" } else { &rule.url };

        // primary error annotation: as precise of a range as possible
        annotations.push(
            AnnotationKind::Primary
                .span(diagnostic.range.start_byte..diagnostic.range.end_byte)
                .label(label)
                .highlight_source(true),
        );

        // explicitly marked context in the query
        if let Some(context) = diagnostic.context {
            annotations.push(
                AnnotationKind::Context
                    .span(context.start_byte..context.end_byte)
                    .label(rule.context_label.clone()),
            );
        }

        // explicitly marked visible in the query
        if let Some(visible) = diagnostic.visible {
            annotations.push(AnnotationKind::Visible.span(visible.start_byte..visible.end_byte));
        }

        // top context: e.g. what function are you in
        if let Some(ctx) = diagnostic.top_context {
            annotations.push(AnnotationKind::Visible.span(ctx.start_byte..ctx.end_byte));
        }

        let level: Level = rule.severity.into();

        let mut report = Vec::new();
        report.push(
            level
                .with_name(rule.severity.to_string())
                .primary_title(diagnostic.title)
                .id(&rule.name)
                .id_url(id_url)
                .element(
                    Snippet::source(source)
                        .path(path.to_str())
                        .annotations(annotations),
                ),
        );
        if let Some(fix) = &rule.fix {
            report.push(
                Level::NOTE
                    .with_name("help")
                    .secondary_title(diagnostic.help)
                    .element(Snippet::source(source).patch(Patch::new(
                        diagnostic.range.start_byte..diagnostic.range.end_byte,
                        fix,
                    ))),
            );
        } else {
            report.push(Group::with_title(
                Level::NOTE
                    .with_name("help")
                    .secondary_title(diagnostic.help),
            ));
        }
        if concise {
            anstream::println!("{}", CONCISE.render(&report));
        } else {
            anstream::println!("{}\n", FULL.render(&report));
        }
    }
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
        self.files = self.files.checked_add(count).expect("no overflow");
    }
    const fn add_problem(&mut self, severity: Severity) {
        match severity {
            Severity::Error => {
                self.error_count = self.error_count.checked_add(1).expect("no overflow");
            }
            Severity::Warn => {
                self.warning_count = self.warning_count.checked_add(1).expect("no overflow");
            }
            Severity::Info => {
                self.info_count = self.info_count.checked_add(1).expect("no overflow");
            }
            Severity::Hint => {
                self.hint_count = self.hint_count.checked_add(1).expect("no overflow");
            }
        }
    }
    const fn add(&mut self, other: Self) {
        self.add_file(other.files);
        self.error_count = self
            .error_count
            .checked_add(other.error_count)
            .expect("no overflow");
        self.warning_count = self
            .warning_count
            .checked_add(other.warning_count)
            .expect("no overflow");
        self.info_count = self
            .info_count
            .checked_add(other.info_count)
            .expect("no overflow");
        self.hint_count = self
            .hint_count
            .checked_add(other.hint_count)
            .expect("no overflow");
    }

    fn problem_count(&self) -> usize {
        (|| -> _ {
            self.error_count
                .checked_add(self.warning_count)?
                .checked_add(self.info_count)?
                .checked_add(self.hint_count)
        })()
        .expect("should not overflow")
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
            .set_language(&crate::LANGUAGE.into())
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
            }
            Err(err) => {
                eprintln!("file error: {err}");
                self.stats.add_problem(Severity::Error);
            }
        }
        WalkState::Continue
    }

    fn check_file(&mut self, path: &Path) -> Result<(), Error> {
        let data = fs::read(path)?;
        self.parser.reset();
        let tree = self
            .parser
            .parse(&data, None)
            .context("parser should be setup")?;
        let result = diagnostics::lint(&tree, &data)?;
        if !result.is_empty() {
            for item in result.iter().as_ref() {
                self.stats.add_problem(rule(item.rule_id).severity);
            }
            render(path, &data, result, self.concise)?;
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
    } else if files == 0 {
        bail!("Found no java files to check");
    }
    println!("Success: No problems found across {files} java files in {millis} ms");
    Ok(())
}
