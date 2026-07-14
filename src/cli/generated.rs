//! Hacks for ignoring generated files

use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use anyhow::Error;
use ignore::overrides::{Override, OverrideBuilder};

/// gitattributes (unlike gitignore) does not support negation etc
/// but you can set attributes to false
pub fn generated_files(path: &Path) -> Result<Option<Override>, Error> {
    if let Some(attributes) = gitattributes(path)
        && let Some(parent) = attributes.parent()
    {
        let mut overrides = OverrideBuilder::new(parent);
        for line in read_to_string(attributes)?.lines() {
            if !line.starts_with('#')
                && let Some(index) = line.find(char::is_whitespace)
                && let Some(name) = line.get(0..index)
                && let Some(rest) = line.get(index..)
                && rest.contains("linguist-generated")
                && !rest.contains("linguist-generated=false")
            {
                overrides.add(format!("!{name}").as_str())?;
            }
        }
        return Ok(Some(overrides.build()?));
    }
    Ok(None)
}

/// locate gitattributes file
fn gitattributes(path: &Path) -> Option<PathBuf> {
    let mut dir = path;
    loop {
        let candidate = dir.join(".gitattributes");
        if candidate.is_file() {
            return Some(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            _ => break,
        }
    }
    None
}
