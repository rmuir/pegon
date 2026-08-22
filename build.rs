//! Generates some tree-sitter related code at build-time
//!
//! Otherwise, the processing must be done at runtime and there is subpar
//! LSP support and maintenance. This generates constants for various IDs
//! from the grammar and queries. It provides additional compile-time "safety"
//! since things such as captures don't require string references, and benefit
//! from unused clippy lints and such.
use std::{
    env::var,
    error::Error,
    ffi::OsStr,
    fs::{DirEntry, read_dir, read_to_string, write},
    ops::Not as _,
    path::Path,
};

use indoc::{formatdoc, indoc};
use tree_sitter::{Language, Query, QueryPredicateArg};

/// build script that regenerates output if the queries files change
fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=queries/java");
    let out = var("OUT_DIR")?;
    let out_dir = Path::new(&out);

    let language: Language = tree_sitter_java_orchard::LANGUAGE.into();

    // create language constants
    write(out_dir.join("java_constants.rs"), constants(&language)?)?;

    // create query constants
    let cargo_manifest = var("CARGO_MANIFEST_DIR")?;
    let queries_dir = Path::new(&cargo_manifest).join("queries").join("java");
    write(
        out_dir.join("java_queries.rs"),
        queries(&language, &queries_dir)?,
    )?;

    Ok(())
}

/// generate constants from the grammar
fn constants(language: &Language) -> Result<String, Box<dyn Error>> {
    let node_kind_count = language.node_kind_count();

    // counts for sizing
    let mut doc = formatdoc! {r"
        /// Number of distinct node types in the language
        pub const NODE_KIND_COUNT: usize = {node_kind_count};
    "};

    doc.push_str(&node_kinds(language)?);
    doc.push_str(&fields(language)?);

    Ok(doc)
}

/// generate node kind IDs from the grammar
fn node_kinds(language: &Language) -> Result<String, Box<dyn Error>> {
    // node kind ids
    let mut doc = indoc! {r#"
        /// Node kind IDs in the language
        #[expect(unused, reason = "not all are used")]
        pub mod kinds {
    "#}
    .to_owned();
    for kind in 0..language.node_kind_count() {
        let id: u16 = kind.try_into()?;
        if !language.node_kind_is_named(id) || !language.node_kind_is_visible(id) {
            continue;
        }
        let name = language
            .node_kind_for_id(id)
            .ok_or("kind should have a name")?;
        let upper_name = name.to_ascii_uppercase();
        doc.push_str(&formatdoc! {r"
            /// Node kind ID for `{name}`
            pub const {upper_name}: u16 = {id};
        "});
    }
    doc.push_str(indoc! {"
        }
    "});
    Ok(doc)
}

/// generate field IDs from the grammar
fn fields(language: &Language) -> Result<String, Box<dyn Error>> {
    // field ids
    let mut doc = indoc! {r#"
        /// Field IDs in the language
        #[expect(unused, reason = "not all are used")]
        pub mod fields {
    "#}
    .to_owned();
    for kind in 0..language.field_count() {
        let id: u16 = kind.try_into()?;
        if let Some(name) = language.field_name_for_id(id) {
            let upper_name = name.to_ascii_uppercase();
            doc.push_str(&formatdoc! {r"
                /// Field ID for `{name}`
                pub const {upper_name}: u16 = {id};
            "});
        }
    }
    doc.push_str(indoc! {"
        }
    "});
    Ok(doc)
}

/// generate constants from each tree-sitter query
fn queries(language: &Language, queries_dir: &Path) -> Result<String, Box<dyn Error>> {
    let mut doc = indoc! {r"
        /// Custom predicates used by queries
        pub enum Predicate {
            /// Compares the text of the captures in codepoint order
            LessThan(u32, u32),
            /// True if the node precedes end of line or end of file
            EndOfLine(u32),
            /// True if the node's text doesn't equal the filename
            NotEqualsFileName(u32),
            /// True if the node has no children
            Terminal(u32),
        }
    "}
    .to_owned();
    let mut entries: Vec<_> = read_dir(queries_dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(DirEntry::path);
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) == Some("scm") {
            let name = path
                .file_stem()
                .ok_or("file should have name")?
                .to_str()
                .ok_or("valid unicode")?;
            doc.push_str(&formatdoc! {r"
                /// Query constants for `{name}.scm`
                pub mod {name} {{
            "});
            let text = read_to_string(&path)?;
            let query = Query::new(language, &text)?;
            doc.push_str(&query_captures(&query));
            if let Some(predicates) = query_predicates(&query) {
                doc.push_str(&predicates);
            }
            if let Some(properties) = query_properties(&query) {
                doc.push_str(&properties);
            }
            doc.push_str(indoc! {"
                }
            "});
        }
    }
    Ok(doc)
}

/// generate capture indexes for a query
fn query_captures(query: &Query) -> String {
    let mut doc = indoc! {r"
        /// Capture indexes in the query
        pub mod captures {
    "}
    .to_owned();

    for (index, name) in query.capture_names().iter().enumerate() {
        if name.starts_with('_') {
            continue;
        }
        let upper_name = name.to_ascii_uppercase().replace('.', "_");
        doc.push_str(&formatdoc! {r"
            /// Capture index for `@{name}`
            pub const {upper_name}: u32 = {index};
        "});
    }
    doc.push_str(indoc! {"
        }
    "});
    doc
}

/// generate arrays of custom predicates indexed by pattern
fn query_predicates(query: &Query) -> Option<String> {
    let mut doc = indoc! {r"
        /// Custom predicates by pattern in the query
        pub mod predicates {
    "}
    .to_owned();

    let mut by_query = vec![];
    let mut by_pattern = vec![];
    for pattern in 0..query.pattern_count() {
        let start_index: u8 = by_query.len().try_into().expect("no overflow");
        for predicate in query.general_predicates(pattern) {
            let operator: &str = &predicate.operator;
            let args: &[QueryPredicateArg] = &predicate.args;
            by_query.push(match operator {
                "lt?" => match args {
                    [
                        QueryPredicateArg::Capture(left),
                        QueryPredicateArg::Capture(right),
                    ] => {
                        formatdoc! {"
                            Predicate::LessThan({left}, {right}),
                        "}
                    }
                    _ => panic!("invalid predicate arguments"),
                },
                "eol?" => match args {
                    [QueryPredicateArg::Capture(capture)] => {
                        formatdoc! {"
                            Predicate::EndOfLine({capture}),
                        "}
                    }
                    _ => panic!("invalid predicate arguments"),
                },
                "not-eq-filename?" => match args {
                    [QueryPredicateArg::Capture(capture)] => {
                        formatdoc! {"
                            Predicate::NotEqualsFileName({capture}),
                        "}
                    }
                    _ => panic!("invalid predicate arguments"),
                },
                "terminal?" => match args {
                    [QueryPredicateArg::Capture(capture)] => {
                        formatdoc! {"
                            Predicate::Terminal({capture}),
                        "}
                    }
                    _ => panic!("invalid predicate arguments"),
                },
                _ => {
                    panic!("invalid predicate {operator}");
                }
            });
        }
        let end_index: u8 = by_query.len().try_into().expect("no overflow");
        by_pattern.push(start_index..end_index);
    }

    if !by_query.is_empty() {
        // output array for the query
        let count = by_query.len();
        doc.push_str(&formatdoc! {"
            use core::ops::Range;
            use crate::java_queries::Predicate;

            /// All predicates used by the query
            pub const PREDICATES: [Predicate; {count}] = [
        "});
        for predicate in &by_query {
            doc.push_str(predicate);
        }
        doc.push_str(&formatdoc! {"
            ];
        "});

        // output array by pattern
        let pattern_count = by_pattern.len();
        doc.push_str(&formatdoc! {"
            /// Range of indices into `PREDICATES` indexed by pattern
            pub const PREDICATES_BY_PATTERN: [Range<u8>; {pattern_count}] = [
        "});
        for range in &by_pattern {
            let start = range.start;
            let end = range.end;
            doc.push_str(&formatdoc! {"
                {start}..{end},
            "});
        }
        doc.push_str(&formatdoc! {"
            ];
        "});

        // end module
        doc.push_str(indoc! {"
            }
        "});
    }

    by_query.is_empty().not().then_some(doc)
}

/// generate arrays of custom predicates indexed by pattern
#[expect(clippy::unwrap_in_result, reason = "panic is desired in such case")]
fn query_properties(query: &Query) -> Option<String> {
    let mut doc = indoc! {r"
        /// Custom properties by pattern in the query
        pub mod properties {
    "}
    .to_owned();

    let mut by_query = vec![];
    let mut by_pattern = vec![];
    for pattern in 0..query.pattern_count() {
        let start_index = by_query.len();
        for property in query.property_settings(pattern) {
            let key: String = property
                .key
                .chars()
                .flat_map(char::escape_default)
                .collect();
            let value: String = property
                .value
                .as_ref()
                .expect("property should have value")
                .chars()
                .flat_map(char::escape_default)
                .collect();
            by_query.push(formatdoc! {r#"
                ("{key}", "{value}"),
            "#});
        }
        let end_index = by_query.len();
        by_pattern.push(start_index..end_index);
    }

    if !by_query.is_empty() {
        // output array for the query
        let count = by_query.len();
        doc.push_str(&formatdoc! {"
            use core::ops::Range;

            /// All properties used by the query
            pub const PROPERTIES: [(&str, &str); {count}] = [
        "});
        for property in &by_query {
            doc.push_str(property);
        }
        doc.push_str(&formatdoc! {"
            ];
        "});

        // output array by pattern
        let pattern_count = by_pattern.len();
        doc.push_str(&formatdoc! {"
            /// Pattern count in the query
            pub const PATTERN_COUNT: usize = {pattern_count};

            /// Range of indices into `PROPERTIES` indexed by pattern
            pub const PROPERTIES_BY_PATTERN: [Range<usize>; {pattern_count}] = [
        "});
        for range in &by_pattern {
            let start = range.start;
            let end = range.end;
            doc.push_str(&formatdoc! {"
                {start}..{end},
            "});
        }
        doc.push_str(&formatdoc! {"
            ];
        "});

        // end module
        doc.push_str(indoc! {"
            }
        "});
    }

    by_query.is_empty().not().then_some(doc)
}
