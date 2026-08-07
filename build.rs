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
    fs::{read_dir, read_to_string, write},
    path::Path,
};

use indoc::{formatdoc, indoc};
use tree_sitter::{Language, Query};

/// build script that regenerates output if the queries files change
fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=src/queries");
    let out = var("OUT_DIR")?;
    let out_dir = Path::new(&out);

    let language: Language = tree_sitter_java_orchard::LANGUAGE.into();

    // create language constants
    write(
        out_dir.join("java_constants.rs"),
        java_constants(&language)?,
    )?;

    // create query constants
    let cargo_manifest = var("CARGO_MANIFEST_DIR")?;
    let queries_dir = Path::new(&cargo_manifest).join("queries").join("java");
    write(out_dir.join("java_queries.rs"), java_queries(&queries_dir)?)?;

    Ok(())
}

/// generate constants from the grammar
fn java_constants(language: &Language) -> Result<String, Box<dyn Error>> {
    let node_kind_count = language.node_kind_count();

    // counts for sizing
    let mut doc = formatdoc! {r"
        /// Number of distinct node types in the language
        pub const NODE_KIND_COUNT: usize = {node_kind_count};
    "};

    doc.push_str(node_kinds(language)?.as_str());
    doc.push_str(fields(language)?.as_str());

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
        doc.push_str(
            formatdoc! {r"
            /// Node kind ID for `{name}`
            pub const {upper_name}: u16 = {id};
        "}
            .as_str(),
        );
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
            doc.push_str(
                formatdoc! {r"
                    /// Field ID for `{name}`
                    pub const {upper_name}: u16 = {id};
                "}
                .as_str(),
            );
        }
    }
    doc.push_str(indoc! {"
        }
    "});
    Ok(doc)
}

/// generate constants from each tree-sitter query
fn java_queries(queries_dir: &Path) -> Result<String, Box<dyn Error>> {
    let language: Language = tree_sitter_java_orchard::LANGUAGE.into();
    let mut doc = String::new();
    for entry in read_dir(queries_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) == Some("scm") {
            let name = path
                .file_stem()
                .ok_or("file should have name")?
                .to_str()
                .ok_or("valid unicode")?;
            doc.push_str(
                formatdoc! {r"
                    /// Query constants for `{name}.scm`
                    pub mod {name} {{
                "}
                .as_str(),
            );
            let text = read_to_string(&path)?;
            let query = Query::new(&language, &text)?;
            doc.push_str(query_captures(&query).as_str());
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
        doc.push_str(
            formatdoc! {r"
            /// Capture index for `@{name}`
            pub const {upper_name}: u32 = {index};
        "}
            .as_str(),
        );
    }
    doc.push_str(indoc! {"
        }
    "});
    doc
}
