use clap::{
    Parser, Subcommand, ValueEnum,
    builder::styling::{AnsiColor, Styles},
};
use std::path::PathBuf;

/// A slightly fast Java linter and code formatter, written in Rust.
///
/// More sentence
#[derive(Parser)]
#[command(name = "pegon", author, version)]
#[command(arg_required_else_help = true)]
#[command(propagate_version = true)]
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
        /// List of files or directories to check, or `-` to read from stdin
        files: Vec<PathBuf>,

        /// Apply fixes to resolve lint violations.
        #[arg(long)]
        fix: bool,

        /// Diagnostic output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Full)]
        output_format: OutputFormat,
    },

    /// Run the pegon formatter on the given files or directories.
    Format {
        /// List of files or directories to format, or `-` to read from stdin
        files: Vec<PathBuf>,

        /// Avoid writing any formatted files back; instead, exit with a non-zero status code if any
        /// files would be modified, and zero otherwise.
        #[arg(long)]
        check: bool,
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
    Full,
    Concise,
}

const CLI_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());
