use anyhow::{Context as _, Result, bail};
use tree_sitter::{QueryMatch, QueryPredicateArg};

use crate::java_constants::NODE_KIND_COUNT;

/// Implement matching for custom predicates
pub fn custom_predicate(
    hit: &QueryMatch,
    data: &[u8],
    operator: &str,
    args: &[QueryPredicateArg],
) -> Result<bool> {
    match operator {
        "lt?" => match args {
            [
                QueryPredicateArg::Capture(left),
                QueryPredicateArg::Capture(right),
            ] => {
                let node1 = hit
                    .nodes_for_capture_index(*left)
                    .next()
                    .context("valid capture")?;
                let node2 = hit
                    .nodes_for_capture_index(*right)
                    .next()
                    .context("valid capture")?;
                let bytes1 = data.get(node1.byte_range()).context("valid range")?;
                let bytes2 = data.get(node2.byte_range()).context("valid range")?;
                Ok(bytes1 < bytes2)
            }
            _ => bail!("invalid predicate arguments"),
        },
        "eol?" => match args {
            [QueryPredicateArg::Capture(capture)] => {
                let node = hit
                    .nodes_for_capture_index(*capture)
                    .next()
                    .context("valid capture")?;
                let position = node.end_byte();
                if position == data.len() {
                    Ok(true)
                } else {
                    Ok(*data.get(position).context("valid range")? == b'\n')
                }
            }
            _ => bail!("invalid predicate arguments"),
        },
        _ => {
            bail!("invalid predicate {operator}");
        }
    }
}

/// maximum size of the set.
const KIND_SET_WORDS: usize = NODE_KIND_COUNT.div_ceil(64);

/// A bitset for efficiently matching node kinds
pub struct KindSet {
    words: [u64; KIND_SET_WORDS],
}

#[expect(clippy::indexing_slicing, reason = "bounds are checked")]
#[expect(clippy::arithmetic_side_effects, reason = "not possible")]
impl KindSet {
    /// create set from list of node kinds
    pub const fn new(kinds: &[u16]) -> Self {
        let mut words: [u64; KIND_SET_WORDS] = [0; KIND_SET_WORDS];
        let mut kind = 0;

        while kind < kinds.len() {
            let id = kinds[kind] as usize;
            debug_assert!(id < NODE_KIND_COUNT);
            let index = id >> 6;
            let bit = id & 0x3F;
            words[index] |= 1 << bit;
            kind += 1;
        }
        Self { words }
    }

    /// true if the named node's kind is in the set
    pub const fn contains(&self, kind_id: u16) -> bool {
        let index = kind_id as usize >> 6;
        let bit = kind_id & 0x3F;
        index < self.words.len() && (self.words[index] & (1 << bit)) != 0
    }
}
