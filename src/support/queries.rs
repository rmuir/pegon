use std::path::Path;

use tree_sitter::QueryMatch;

use crate::java_constants::NODE_KIND_COUNT;
use crate::java_queries::Predicate;

/// allows matching against predicates generated from build.rs
pub trait PredicateMatch {
    /// true if the hit matches the predicate
    fn matches(&self, hit: &QueryMatch, data: &[u8], path: Option<&Path>) -> bool;
}

impl PredicateMatch for Predicate {
    fn matches(&self, hit: &QueryMatch, data: &[u8], path: Option<&Path>) -> bool {
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
                let byte = data.get(position).copied().unwrap_or(b'\n');
                byte == b'\n' || byte == b'\r'
            }
            Self::NotEqualsFileName(capture) => {
                let Some(path) = path else { return false };
                let Some(path) = path.file_stem() else {
                    return false;
                };
                let node = hit
                    .nodes_for_capture_index(*capture)
                    .next()
                    .expect("valid capture");
                Some(path.as_encoded_bytes()) != data.get(node.byte_range())
            }
        }
    }
}

/// Create array element at a time from const fn
macro_rules! const_array_from_fn {
    ($f:expr, $n:expr) => {{
        let mut array = [const { ::core::mem::MaybeUninit::uninit() }; $n];
        let mut index = 0;
        while index < $n {
            array[index].write($f(index));
            index += 1;
        }
        // SAFETY: entire array was initialized above
        unsafe { ::core::mem::transmute::<_, [_; $n]>(array) }
    }};
}

pub(crate) use const_array_from_fn;

/// parse a boolean from a string
pub const fn to_bool_const(string: &str) -> bool {
    match string.as_bytes() {
        b"true" => true,
        b"false" => false,
        _ => panic!("invalid boolean"),
    }
}

/// parse an integer from a string
#[expect(clippy::indexing_slicing, reason = "compile time")]
#[expect(clippy::arithmetic_side_effects, reason = "compile time")]
#[expect(clippy::cast_possible_wrap, reason = "compile time")]
pub const fn to_i8_const(string: &str) -> i8 {
    let bytes = string.as_bytes();
    let mut sign: i8 = 1;
    let mut mag: i8 = 0;
    let mut index = 0;
    if bytes[0] == b'-' {
        sign = -1;
        index += 1;
    } else if bytes[0] == b'+' {
        index += 1;
    }
    while index < bytes.len() {
        let byte = bytes[index];
        assert!(b'0' <= byte && byte <= b'9', "invalid digit");
        mag = mag * 10 + (byte - b'0') as i8;
        index += 1;
    }
    sign * mag
}

// slow linear search at compile-time until rust lets us bsearch
#[expect(clippy::indexing_slicing, reason = "bounds are checked")]
#[expect(clippy::arithmetic_side_effects, reason = "not possible")]
pub const fn const_table_search(slice: &[&str], target: &str) -> usize {
    let target = target.as_bytes();
    let mut index = 0;
    'iteration: while index < slice.len() {
        let val = slice[index].as_bytes();
        if target.len() == val.len() {
            let mut offset = 0;
            while offset < val.len() {
                if target[offset] != val[offset] {
                    index += 1;
                    continue 'iteration;
                }
                offset += 1;
            }
            return index;
        }
        index += 1;
    }
    panic!("did not find element in table");
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
