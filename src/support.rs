//! Shared support code

pub mod diagnostics;
pub mod index;
pub mod queries;

/// Tree-sitter grammar in use
pub fn language() -> tree_sitter::Language {
    tree_sitter_java_orchard::LANGUAGE.into()
}
