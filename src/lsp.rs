use std::sync::Arc;

use anyhow::{Error, Result};
use lsp_server::Connection;

use client::Client;
use server::Server;

mod client;
mod code_action;
mod diagnostics;
mod document_symbols;
mod folding_range;
mod hover;
mod initialize;
mod selection_range;
mod server;
mod sync;

/// Run lsp server with provided connection
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
