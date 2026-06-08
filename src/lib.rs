//! Test
//!

mod cli;
/// Shared support code
mod diagnostics;
/// Language Server functionality
mod lsp;

pub use cli::main;
pub use lsp::run_server;

/// Tree-sitter grammar in use
const LANGUAGE: tree_sitter_language::LanguageFn = tree_sitter_java_orchard::LANGUAGE;
