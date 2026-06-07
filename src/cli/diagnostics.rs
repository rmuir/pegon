use annotate_snippets::{
    Annotation, AnnotationKind, Group, Level, Patch, Renderer, Snippet,
    renderer::{Ansi256Color, DecorStyle, Style},
};
use anyhow::{Context as _, Error, bail};
use core::fmt::{Display, Formatter};
use core::sync::atomic::{AtomicUsize, Ordering};

use ignore::{WalkBuilder, WalkState, overrides::OverrideBuilder, types::TypesBuilder};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use tree_sitter::Parser;

use crate::{
    cli,
    diagnostics::{self, Diagnostic, Severity, rule},
};

static GREY: Style = Ansi256Color(247).on_default();
static FULL: Renderer = Renderer::styled()
    .decor_style(DecorStyle::Unicode)
    .context(GREY)
    .line_num(GREY);

static CONCISE: Renderer = Renderer::plain().short_message(true);

impl Display for Severity {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match *self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Hint => write!(f, "note"),
        }
    }
}

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
        cli::diagnostics::render(path, &data, result, concise)?;
    }
    FILES.fetch_add(1, Ordering::Relaxed);
    Ok(())
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

    // TODO: use parallelvisitor builder
    builder.build_parallel().run(|| {
        let mut parser = Parser::new();
        parser
            .set_language(&crate::LANGUAGE.into())
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
