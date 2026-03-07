// Build script that generates man pages for the CLI using clap_mangen.
// It includes the same derive-based CLI definitions from src/cli.rs so that
// the Command layout is a single source of truth.

use core::error::Error;
use std::env;
use std::fs;
use std::path::PathBuf;

// Include the CLI definitions directly so this build script (a separate crate)
// can use the same clap derive types. Requires `clap` in [build-dependencies].
mod cli {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli.rs"));
}

fn main() -> Result<(), Box<dyn Error>> {
    // Always rerun if CLI file changes
    println!("cargo:rerun-if-changed=src/cli.rs");

    // Determine output directory: use target/man under the workspace for convenience.
    // Using OUT_DIR is also fine; target/man is easier to discover.
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = manifest_dir.join("target").join("man");
    fs::create_dir_all(&out_dir)?;

    // Build the clap::Command from the derive type.
    let cmd = <cli::Cli as clap::CommandFactory>::command();

    // Generate a man page for the root and all subcommands recursively.
    // clap_mangen::generate_to will walk the subcommands if you pass the root Command.
    clap_mangen::generate_to(cmd, &out_dir)?;

    println!("cargo:warning=Generated man pages to {}", out_dir.display());

    Ok(())
}
