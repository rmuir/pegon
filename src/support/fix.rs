//! Fixes for diagnostic issues
//!
//! It works on byte ranges so it can be eventually efficient in bulk
use std::ops::Range;
use std::str::from_utf8;
use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use regex::{Captures, Regex};
use tree_sitter::Tree;

use crate::support::diagnostics::Diagnostic;

/// Automatic Fix for an issue
pub enum Fix {
    /// Replace whitespace in range with escapes
    EscapeWhitespace,
    /// Replace newlines in range with spaces
    LineUnwrap,
    /// Sort imports according to google style
    OrganizeImports,
    /// Replace range with static string
    Static(&'static str),
    /// Transform range to uppercase
    ToUpper,
}

impl Fix {
    /// Generate an [`Edit`] if possible to fix the issue
    pub fn generate(&self, range: Range<usize>, tree: &Tree, data: &[u8]) -> Result<Option<Edit>> {
        Ok(match self {
            Self::EscapeWhitespace => Some(Edit {
                range: range.clone(),
                replacement: escape_whitespace(to_text(data, range)?),
            }),
            Self::LineUnwrap => Some(Edit {
                range: range.clone(),
                replacement: to_text(data, range)?.replace(['\r', '\n'], " "),
            }),
            Self::OrganizeImports => super::organize_imports::organize(tree, data)?,
            Self::Static(replacement) => Some(Edit {
                range,
                replacement: (*replacement).into(),
            }),
            Self::ToUpper => Some(Edit {
                range: range.clone(),
                replacement: to_text(data, range)?.to_uppercase(),
            }),
        })
    }

    /// Generate a list of backwards-sorted edits from a list of diagnostics
    pub fn batch(diagnostics: &[Diagnostic], tree: &Tree, data: &[u8]) -> Result<Vec<Edit>> {
        let edits: Result<Vec<Edit>> = diagnostics
            .iter()
            .filter_map(|diagnostic| {
                diagnostic
                    .pattern()
                    .fix
                    .as_ref()
                    .map(|fix| fix.generate(diagnostic.range(), tree, data).transpose())
            })
            .flatten()
            .collect();
        let mut edits = edits?;
        // sort the edits backwards
        edits.sort_unstable_by(|right, left| {
            left.range
                .start
                .cmp(&right.range.start)
                .then_with(|| left.range.end.cmp(&right.range.end))
        });
        Ok(edits)
    }
}

/// Edit to fix an issue
#[derive(PartialEq, Eq)]
pub struct Edit {
    /// Byte range to replace
    pub range: Range<usize>,
    /// New contents
    pub replacement: String,
}

impl Edit {
    /// half-open range intersection
    ///
    /// come on rust, get it together
    pub const fn intersects(left: &Range<usize>, right: &Range<usize>) -> bool {
        left.start < right.end && right.start < left.end
    }
}

/// matches whitespace that should not exist in any string (single or multi-line)
static WHITESPACE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\s&&[^\u0020\r\n]]").expect("whitespace regex should compile"));

/// escapes whitespace into java-escapes (UTF-16)
///
/// tab and form-feed should be converted into their special escape
fn escape_whitespace(old_text: &str) -> String {
    WHITESPACE_REGEX
        .replace_all(old_text, |caps: &Captures| {
            let codepoint = caps[0].chars().next().expect("capture should exist") as u32;
            match codepoint {
                0x9 => "\\t".into(),
                0xC => "\\f".into(),
                bmp if codepoint <= 0xFFFF => format!("\\u{bmp:04X}"),
                _ => {
                    let (high, low) = to_surrogates(codepoint).expect("valid unicode");
                    format!("\\u{high:04X}\\u{low:04X}")
                }
            }
        })
        .into()
}

const SURROGATE_HIGH_START: u32 = 0xD800;
const SURROGATE_LOW_START: u32 = 0xDC00;

/// split codepoint into surrogate pair for java
fn to_surrogates(codepoint: u32) -> Result<(u16, u16)> {
    let surrogate_offset = codepoint.checked_sub(0x10000).context("supplementary")?;
    let high = SURROGATE_HIGH_START
        .checked_add(surrogate_offset >> 10)
        .context("valid high")?
        .try_into()?;
    let low = SURROGATE_LOW_START
        .checked_add(surrogate_offset & 0x3FF)
        .context("valid low")?
        .try_into()?;
    Ok((high, low))
}

fn to_text(data: &[u8], range: Range<usize>) -> Result<&str> {
    Ok(from_utf8(data.get(range).context("valid range")?)?)
}
