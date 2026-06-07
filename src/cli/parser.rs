use clap::{
    Parser, Subcommand, ValueEnum,
    builder::styling::{AnsiColor, Styles},
};
use std::path::PathBuf;

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
        /// Use standard I/O streams [default]
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
