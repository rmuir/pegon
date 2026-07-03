//! Shared support code

pub mod diagnostics;
pub mod queries;
pub mod scopes;

/// Tree-sitter grammar in use
pub fn language() -> tree_sitter::Language {
    tree_sitter_java_orchard::LANGUAGE.into()
}
