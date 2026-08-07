use tree_sitter::QueryMatch;

use crate::java_constants::NODE_KIND_COUNT;
use crate::java_queries::Predicate;

/// allows matching against predicates generated from build.rs
pub trait PredicateMatch {
    /// true if the hit matches the predicate
    fn matches(&self, hit: &QueryMatch, data: &[u8]) -> bool;
}

impl PredicateMatch for Predicate {
    fn matches(&self, hit: &QueryMatch, data: &[u8]) -> bool {
        match self {
            Self::LessThan(left, right) => {
                let node1 = hit
                    .nodes_for_capture_index(*left)
                    .next()
                    .expect("valid capture");
                let node2 = hit
                    .nodes_for_capture_index(*right)
                    .next()
                    .expect("valid capture");
                data.get(node1.byte_range()) < data.get(node2.byte_range())
            }
            Self::EndOfLine(capture) => {
                let node = hit
                    .nodes_for_capture_index(*capture)
                    .next()
                    .expect("valid capture");
                let position = node.end_byte();
                data.get(position).copied().unwrap_or(b'\n') == b'\n'
            }
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
