//! Language Server
use std::sync::Arc;

use anyhow::{Error, Result};
use lsp_server::Connection;

use client::Client;
use server::Server;

mod client;
mod code_action;
mod definition;
mod diagnostics;
mod document_highlight;
mod document_symbols;
mod folding_range;
mod hover;
mod initialize;
mod inlay_hints;
mod locals;
mod selection_range;
mod semantic_cache;
mod semantic_tokens;
mod server;
mod sync;
#[cfg(test)]
mod test_client;

/// Run LSP server
///
/// # Arguments
///
/// * `connection` - JSON-RPC connection (e.g. stdio, memory, socket)
///
/// # Errors
///
/// This function will return an error if the server does not
/// terminate in a graceful way: e.g. if the client disconnects.
pub fn run_server(connection: Connection) -> Result<(), Error> {
    // get the client capabilities
    let (id, params) = connection.initialize_start()?;
    let client = Client::new(serde_json::from_value(params)?);
    let server = Server::new(connection, &client, id)?;
    server.main_loop(&Arc::new(client))
}
