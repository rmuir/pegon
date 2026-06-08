#![cfg_attr(doc, doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")))]

mod cli;
mod lsp;
mod support;

pub use cli::main;
pub use lsp::run_server;
