use annotate_snippets::{
    Annotation, AnnotationKind, Group, Level, Patch, Renderer, Snippet,
    renderer::{Ansi256Color, DecorStyle, Style},
};
use anyhow::Error;
use std::{
    fmt::{Display, Formatter},
    path::Path,
};

use crate::lint::{Lint, Severity, rule};

static GREY: Style = Ansi256Color(247).on_default();
static RENDERER: Renderer = Renderer::styled()
    .decor_style(DecorStyle::Unicode)
    .context(GREY)
    .line_num(GREY);

impl Display for Severity {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::Error => write!(formatter, "error"),
            Self::Warn => write!(formatter, "warn"),
            Self::Info => write!(formatter, "info"),
            Self::Hint => write!(formatter, "hint"),
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

pub(crate) fn render(path: &Path, data: &[u8], errors: Vec<Lint>) -> Result<(), Error> {
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
                .span(diagnostic.range.clone())
                .label(diagnostic.label),
        );

        // only write context label a single time, colors will coordinate
        let mut label_written = false;

        // explicitly marked context in the query
        for context in diagnostic.context {
            if label_written {
                annotations.push(AnnotationKind::Context.span(context));
            } else {
                annotations.push(
                    AnnotationKind::Context
                        .span(context)
                        .label(rule.context_label.clone()),
                );
                label_written = true;
            }
        }

        // explicitly marked visible in the query
        for visible in diagnostic.visible {
            annotations.push(AnnotationKind::Visible.span(visible));
        }

        // top context: e.g. what function are you in
        if let Some(ctx) = diagnostic.top_context {
            annotations.push(AnnotationKind::Visible.span(ctx));
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
                    .element(Snippet::source(source).patch(Patch::new(diagnostic.range, fix))),
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
