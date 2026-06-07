use core::net::Ipv4Addr;

use crate::cli::diagnostics::check;
use crate::cli::parser::Cli;
use crate::cli::parser::Commands::{Check, Server};
use crate::cli::parser::OutputFormat::Concise;
use anyhow::Error;
use clap::Parser as _;
use lsp_server::Connection;

use pegon::{cli, lsp};

fn main() -> Result<(), Error> {
    let options = Cli::parse();
    match &options.command {
        Check {
            files,
            fix: _,
            output_format,
        } => check(files, *output_format == Concise),
        Server { socket: None, .. } => {
            let (connection, iothreads) = Connection::stdio();
            let result = lsp::start(connection);
            iothreads.join()?;
            result
        }
        Server {
            socket: Some(port), ..
        } => {
            let addr = (Ipv4Addr::LOCALHOST, *port);
            let (connection, iothreads) = Connection::listen(addr)?;
            let result = lsp::start(connection);
            iothreads.join()?;
            result
        }
    }
}
