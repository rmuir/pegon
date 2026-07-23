//! CLI "analyze" command
use anyhow::Error;

use std::{path::PathBuf, time::Instant};

/// Analyze the set of files
///
/// # Errors
///
/// Returns an error if any files had problems, or if internal errors were encountered.
pub fn analyze(inputs: &[PathBuf]) -> Result<(), Error> {
    let start_time = Instant::now();
    let index = crate::support::index::index(inputs)?;
    let elapsed = start_time.elapsed();
    let millis = elapsed.as_millis();
    eprintln!("Success: analyzed in {millis} ms");
    serde_json::to_writer_pretty(std::io::stdout(), &index)?;
    Ok(())
}
