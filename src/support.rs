//! Shared support code

pub mod diagnostics;
pub mod queries;

/// Tree-sitter grammar in use
pub const LANGUAGE: tree_sitter_language::LanguageFn = tree_sitter_java_orchard::LANGUAGE;
