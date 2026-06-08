//! Command line interface

use core::net::Ipv4Addr;
use std::path::PathBuf;

mod diagnostics;

use anyhow::Error;
use clap::{
    Parser, Subcommand, ValueEnum,
    builder::styling::{AnsiColor, Styles},
};

use lsp_server::Connection;

/// Fast linter for the Google Java Style.
///
/// More information
#[derive(Parser)]
#[command(author, version)]
#[command(arg_required_else_help = true)]
#[command(disable_help_subcommand = true)]
#[command(styles = CLI_STYLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run pegon on the given files or directories.
    ///
    /// More information
    Check {
        /// List of files or directories to check
        ///
        /// Use `-` for standard input. [default: CWD]
        files: Vec<PathBuf>,

        /// Apply fixes to resolve lint violations.
        #[arg(long)]
        fix: bool,

        /// Diagnostic error format
        #[arg(long, value_enum, id = "FMT", default_value_t = OutputFormat::Full)]
        output_format: OutputFormat,
    },

    /// Run the language server
    #[group(required = false, multiple = false)]
    Server {
        /// Use standard I/O streams (default)
        #[arg(long)]
        stdio: bool,

        /// Listen on loopback TCP socket
        #[arg(long, id = "PORT")]
        socket: Option<u16>,
    },
}

#[derive(ValueEnum, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    /// Cargo-style format
    Full,
    /// Grep-style format
    Concise,
}

const CLI_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

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
            fix: _,
            output_format,
        } => diagnostics::check(files, *output_format == OutputFormat::Concise),
        Commands::Server { socket: None, .. } => {
            let (connection, iothreads) = Connection::stdio();
            let result = crate::lsp::start(connection);
            iothreads.join()?;
            result
        }
        Commands::Server {
            socket: Some(port), ..
        } => {
            let addr = (Ipv4Addr::LOCALHOST, *port);
            let (connection, iothreads) = Connection::listen(addr)?;
            let result = crate::lsp::start(connection);
            iothreads.join()?;
            result
        }
    }
}
