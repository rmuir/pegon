use clap::{Parser, Subcommand, builder::styling::AnsiColor, builder::styling::Styles};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about, long_about = None, version)]
#[command(arg_required_else_help = true)]
#[command(propagate_version = true)]
#[command(styles = CLI_STYLES)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Run pegon on the given files or directories.
    Check {
        /// List of files or directories to check, or `-` to read from stdin
        files: Vec<PathBuf>,

        /// Apply fixes to resolve lint violations.
        #[arg(long, short)]
        fix: bool,
    },

    /// Run the pegon formatter on the given files or directories.
    Format {
        /// List of files or directories to format, or `-` to read from stdin
        files: Vec<PathBuf>,

        /// Avoid writing any formatted files back; instead, exit with a non-zero status code if any
        /// files would be modified, and zero otherwise.
        #[arg(long, short)]
        check: bool,
    },

    /// Run the language server
    Server,
}

const CLI_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

pub(crate) fn parse() -> Cli {
    Cli::parse()
}
