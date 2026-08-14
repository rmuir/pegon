//! Shared support code

use std::sync::LazyLock;

use tree_sitter::Language;

pub mod diagnostics;
pub mod fix;
pub mod index;
pub mod organize_imports;
pub mod queries;

/// Tree-sitter grammar in use
pub static LANGUAGE: LazyLock<Language> =
    LazyLock::new(|| tree_sitter_java_orchard::LANGUAGE.into());
