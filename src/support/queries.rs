use anyhow::{Context as _, Result, bail};
use tree_sitter::{Query, QueryMatch, QueryPredicateArg};

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

/// Returns id of the capture, or panics if it doesn't exist in the query
pub fn capture_id(query: &Query, name: &str) -> u32 {
    query
        .capture_index_for_name(name)
        .unwrap_or_else(|| panic!("{name} capture should exist"))
}

/// maximum size of the set.
///
/// the grammar is pinned and will fail tests if its too small.
const KIND_SET_WORDS: usize = 6;

/// A bitset for efficiently matching node kinds
#[derive(Copy, Clone)]
pub struct KindSet {
    words: [u64; KIND_SET_WORDS],
}

#[expect(clippy::indexing_slicing, reason = "bounds are checked")]
impl KindSet {
    /// create set from list of node kind names
    pub fn new(kinds: &[&str]) -> Self {
        let mut words: [u64; KIND_SET_WORDS] = [0; KIND_SET_WORDS];
        let lang = super::language();
        debug_assert!(lang.node_kind_count() <= words.len() << 6);
        for kind in kinds {
            let id = lang.id_for_node_kind(kind, true);
            let index = id as usize >> 6;
            let bit = id & 0x3F;
            words[index] |= 1 << bit;
        }
        Self { words }
    }

    /// true if the named node's kind is in the set
    pub const fn contains(self, kind_id: u16) -> bool {
        let index = kind_id as usize >> 6;
        let bit = kind_id & 0x3F;
        index < self.words.len() && (self.words[index] & (1 << bit)) != 0
    }
}
