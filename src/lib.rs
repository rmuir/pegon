//! Test
//!

pub mod cli;
/// Shared support code
mod diagnostics;
/// Language Server functionality
pub mod lsp;

/// Tree-sitter grammar in use
const LANGUAGE: tree_sitter_language::LanguageFn = tree_sitter_java_orchard::LANGUAGE;
