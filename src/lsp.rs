use anyhow::{Error, Result};
use crossbeam_channel::SendError;
use lsp_server::{Connection, ErrorCode, Message, RequestId, Response, ResponseError};
use lsp_types::InitializeParams;
use serde::Serialize;

use crate::lsp::client::Client;
use crate::lsp::server::Server;

mod client;
mod diagnostics;
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
    let init_params: InitializeParams = serde_json::from_value(params)?;

    let server = Server { connection };
    let client = Client::new(init_params);
    let result = serde_json::json!(server.initialize(&client));

    server.connection.initialize_finish(id, result)?;
    server.main_loop(&client)?;
    Ok(())
}

/// sends an LSP notification to the client
fn notify<N>(conn: &Connection, params: N::Params) -> Result<(), SendError<Message>>
where
    N: lsp_types::notification::Notification,
    N::Params: Serialize,
{
    let notification = lsp_server::Notification::new(N::METHOD.to_owned(), params);
    conn.sender.send(Message::Notification(notification))?;
    Ok(())
}

/// responds successfully to an LSP client request
fn respond<T: serde::Serialize>(conn: &Connection, id: RequestId, result: &T) -> Result<()> {
    // TODO: tighten the types up like notify(), but lsp types get complex here
    let resp = Response {
        id,
        result: Some(serde_json::to_value(result)?),
        error: None,
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}

/// responds unsuccessfully to an LSP client request
fn error(
    conn: &Connection,
    id: RequestId,
    code: ErrorCode,
    msg: &str,
) -> Result<(), SendError<Message>> {
    let resp = Response {
        id,
        result: None,
        error: Some(ResponseError {
            code: code as i32,
            message: msg.into(),
            data: None,
        }),
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}
