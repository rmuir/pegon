pub mod cli;
mod diagnostics;
pub mod lsp;

pub const LANGUAGE: tree_sitter_language::LanguageFn = tree_sitter_java_orchard::LANGUAGE;
