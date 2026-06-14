//! Command line interface

use core::net::Ipv4Addr;

mod check;
mod parser;

use anyhow::Error;
use clap::Parser as _;
use lsp_server::Connection;

use crate::cli::parser::{Cli, Commands, OutputFormat};

/// CLI entrypoint
///
/// # Errors
///
/// Returns error if checking found problems, or if the server did
/// non exit gracefully.
pub fn main() -> Result<(), Error> {
    let options = Cli::parse();
    match &options.command {
        Commands::Check {
            files,
            output_format,
            ..
        } => check::check(files, *output_format == OutputFormat::Concise),
        Commands::Server { socket: None, .. } => {
            let (connection, iothreads) = Connection::stdio();
            let result = crate::lsp::run_server(connection);
            iothreads.join()?;
            result
        }
        Commands::Server {
            socket: Some(port), ..
        } => {
            let addr = (Ipv4Addr::LOCALHOST, *port);
            let (connection, iothreads) = Connection::listen(addr)?;
            let result = crate::lsp::run_server(connection);
            iothreads.join()?;
            result
        }
    }
}
