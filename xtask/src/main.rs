use clap::CommandFactory as _;
use clap::ValueEnum as _;
use core::error::Error;
use pegon::cli::Cli;
use std::{env, fs};

fn main() -> Result<(), Box<dyn Error>> {
    help()?;
    manpages()?;
    completions()?;
    Ok(())
}

/// output help to `$CWD/docs/README.md`
fn help() -> Result<(), Box<dyn Error>> {
    let mut command = Cli::command().flatten_help(true).disable_colored_help(true);
    let help = format!("```text\n{}\n```\n", command.render_long_help());
    let out_dir = env::current_dir()?.join("docs");
    fs::write(out_dir.join("README.md"), help)?;
    println!("Generated help to {}", out_dir.display());
    Ok(())
}

/// output manual pages to `$CWD/docs/man`
fn manpages() -> Result<(), Box<dyn Error>> {
    let out_dir = env::current_dir()?.join("docs").join("man");
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)?;
    }
    fs::create_dir_all(&out_dir)?;
    clap_mangen::generate_to(Cli::command(), &out_dir)?;
    println!("Generated man pages to {}", out_dir.display());
    Ok(())
}

/// output completions to `$CWD/contrib/completions`
fn completions() -> Result<(), Box<dyn Error>> {
    let out_dir = env::current_dir()?.join("contrib").join("completions");
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)?;
    }
    fs::create_dir_all(&out_dir)?;
    let mut command = Cli::command();
    let name = command.get_name().to_owned();
    for &shell in clap_complete::Shell::value_variants() {
        clap_complete::generate_to(shell, &mut command, name.as_str(), &out_dir)?;
    }
    println!("Generated shell completions to {}", out_dir.display());
    Ok(())
}
