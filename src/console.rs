use annotate_snippets::{
    Annotation, AnnotationKind, Group, Level, Patch, Renderer, Snippet,
    renderer::{Ansi256Color, DecorStyle, Style},
};
use anyhow::Error;
use core::fmt::{Display, Formatter};
use std::path::Path;

use crate::lint::{Lint, Severity, rule};

static GREY: Style = Ansi256Color(247).on_default();
static RENDERER: Renderer = Renderer::styled()
    .decor_style(DecorStyle::Unicode)
    .context(GREY)
    .line_num(GREY);

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

pub fn render(path: &Path, data: &[u8], errors: Vec<Lint>) -> Result<(), Error> {
    if errors.is_empty() {
        return Ok(());
    }
    let source = str::from_utf8(data)?;
    for diagnostic in errors {
        let mut annotations: Vec<Annotation> = Vec::new();
        let rule = rule(diagnostic.rule_id);

        // primary error annotation: as precise of a range as possible
        annotations.push(
            AnnotationKind::Primary
                .span(diagnostic.range.start_byte..diagnostic.range.end_byte)
                .label(diagnostic.label)
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
                .id_url(&rule.url)
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
        anstream::println!("{}\n", RENDERER.render(&report));
    }
    Ok(())
}
