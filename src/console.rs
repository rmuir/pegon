use annotate_snippets::{
    Annotation, AnnotationKind, Group, Level, Patch, Renderer, Snippet,
    renderer::{DecorStyle, Style},
};
use anyhow::Error;
use std::path::Path;

use crate::lint::Lint;

static RENDERER: Renderer = Renderer::styled()
    .decor_style(DecorStyle::Unicode)
    .context(Style::new().dimmed())
    .line_num(Style::new().dimmed());

pub(crate) fn render(path: &Path, data: Vec<u8>, errors: Vec<Lint>) -> Result<(), Error> {
    if errors.is_empty() {
        return Ok(());
    }
    let source = str::from_utf8(&data)?;
    for diagnostic in errors {
        let mut annotations: Vec<Annotation> = Vec::new();

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
                        .label(diagnostic.context_label.clone()),
                );
                label_written = true
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

        let level = match diagnostic.severity.as_str() {
            "warn" => Level::WARNING,
            "info" => Level::INFO,
            "hint" => Level::HELP,
            _ => Level::ERROR,
        };

        let mut report = Vec::new();
        report.push(
            level
                .with_name(diagnostic.severity)
                .primary_title(diagnostic.title)
                .id(diagnostic.name)
                .id_url(diagnostic.url)
                .element(
                    Snippet::source(source)
                        .path(path.to_str())
                        .annotations(annotations),
                ),
        );
        if let Some(fix) = diagnostic.fix {
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
        anstream::println!("{}\n", RENDERER.render(&report))
    }
    Ok(())
}
