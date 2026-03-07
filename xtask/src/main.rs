use clap::CommandFactory as _;
use core::error::Error;
use pegon::cli::Cli;
use std::{env, fs};

fn main() -> Result<(), Box<dyn Error>> {
    // put manpages in CWD/target/man
    let out_dir = env::current_dir()?.join("target").join("man");
    fs::create_dir_all(&out_dir)?;
    clap_mangen::generate_to(Cli::command(), &out_dir)?;
    println!("Generated man pages to {}", out_dir.display());
    Ok(())
}
