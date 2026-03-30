use anyhow::{Error, Result};
use lsp_server::Connection;

use crate::lsp::client::Client;
use crate::lsp::server::Server;

mod client;
mod diagnostics;
mod document_symbols;
mod server;
mod sync;

/// Start lsp server with provided connection
///
/// # Errors
///
/// This function will return an error if something bad happens
pub fn start(connection: Connection) -> Result<(), Error> {
    // get the client capabilities
    let (id, params) = connection.initialize_start()?;
    let client = Client::new(serde_json::from_value(params)?);
    let server = Server::new(connection, &client, id)?;
    server.main_loop(&client)
}
