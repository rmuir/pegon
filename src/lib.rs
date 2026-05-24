pub mod cli;
pub mod console;
pub mod diagnostics;
pub mod lsp;

pub const LANGUAGE: tree_sitter_language::LanguageFn = tree_sitter_java_orchard::LANGUAGE;
