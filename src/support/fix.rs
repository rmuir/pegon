//! Fixes for diagnostic issues
//!
//! It works on byte ranges so it can be eventually efficient in bulk
use std::ops::Range;
use std::str::from_utf8;

use anyhow::{Context as _, Result, bail};
use regex::{Captures, Regex};
use tree_sitter::Tree;

/// Automatic Fix for an issue
pub enum Fix {
    /// Replace whitespace in range with escapes
    EscapeWhitespace,
    /// Replace newlines in range with spaces
    LineUnwrap,
    /// Sort imports according to google style
    OrganizeImports,
    /// Replace range with static string
    Static(String),
    /// Transform range to uppercase
    ToUpper,
}

impl Fix {
    /// Generate an [`Edit`] if possible to fix the issue
    pub fn generate(&self, range: Range<usize>, _tree: &Tree, data: &[u8]) -> Result<Option<Edit>> {
        let old_text = from_utf8(data.get(range.clone()).context("valid range")?)?;
        Ok(match self {
            Self::EscapeWhitespace => Some(Edit {
                range,
                replacement: escape_whitespace(old_text)?,
            }),
            Self::LineUnwrap => Some(Edit {
                range,
                replacement: old_text.replace('\n', " "),
            }),
            Self::OrganizeImports => bail!("not yet"),
            Self::Static(replacement) => Some(Edit {
                range,
                replacement: replacement.clone(),
            }),
            Self::ToUpper => Some(Edit {
                range,
                replacement: old_text.to_uppercase(),
            }),
        })
    }
}

/// Edit to fix an issue
pub struct Edit {
    /// Byte range to replace
    pub range: Range<usize>,
    /// New contents
    pub replacement: String,
}

/// escapes whitespace into java-escapes (UTF-16)
///
/// tab and form-feed should be converted into their special escape
fn escape_whitespace(old_text: &str) -> Result<String> {
    // FIXME: compile this / make it more efficient
    let re = Regex::new(r"[\s&&[^\u0020\r\n]]")?;
    let result = re.replace_all(old_text, |caps: &Captures| {
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
    });
    Ok(result.into())
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
