#![cfg_attr(doc, doc = include_str!("../README.md"))]

mod cli;
mod lsp;
mod support;

pub use cli::main;
pub use lsp::run_server;
