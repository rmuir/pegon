#![allow(clippy::allow_attributes_without_reason)]
#![allow(unused)]
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use anyhow::{Context as _, Error};
use gen_lsp_types::{SemanticToken, SemanticTokensEdit};

#[derive(Default)]
pub struct Cache(Mutex<VecDeque<CacheEntry>>);

struct CacheEntry {
    tokens: Vec<SemanticToken>,
    result_id: String,
}

const HISTORY_LEN: usize = 16;

impl Cache {
    /// Pushes a new cache entry
    pub fn push(&self, data: &[SemanticToken]) -> String {
        let integers: Vec<u32> = data.iter().copied().flat_map(<[u32; 5]>::from).collect();
        let bytes: Vec<u8> = integers
            .iter()
            .flat_map(|integer| integer.to_le_bytes())
            .collect();

        let result_id = blake3::hash(&bytes).to_string();
        let tokens = Vec::from(data);
        let mut cache = self.0.lock().expect("poisoned");
        if cache.len() > HISTORY_LEN {
            cache.pop_back();
        }
        cache.push_front(CacheEntry {
            result_id: result_id.clone(),
            tokens,
        });
        result_id
    }

    /// Looks for a cached entry whose id matches `previous_result_id`.
    pub fn delta(
        &self,
        previous_result_id: &str,
        data: &[SemanticToken],
    ) -> Option<Vec<SemanticTokensEdit>> {
        let previous = {
            self.0
                .lock()
                .expect("poisoned")
                .iter()
                .find(|entry| entry.result_id == previous_result_id)
                .map(|entry| entry.tokens.clone())
        }?;
        diff(&previous, data).ok()
    }
}

/// byte-level diff of semantic tokens
/// see rust-analyzer implementation for inspiration
fn diff(old: &[SemanticToken], new: &[SemanticToken]) -> Result<Vec<SemanticTokensEdit>, Error> {
    // common prefix shared by old and new
    let common_prefix = old
        .iter()
        .zip(new)
        .take_while(|(old_word, new_word)| old_word == new_word)
        .count();

    let (_, old) = old.split_at_checked(common_prefix).context("valid slice")?;
    let (_, new) = new.split_at_checked(common_prefix).context("valid slice")?;

    // common suffix shared by old and new
    let common_suffix = new
        .iter()
        .rev()
        .zip(old.iter().rev())
        .take_while(|(old_word, new_word)| old_word == new_word)
        .count();

    let old_limit = old
        .len()
        .checked_sub(common_suffix)
        .context("no overflow")?;

    let new_limit = new
        .len()
        .checked_sub(common_suffix)
        .context("no overflow")?;

    let (old, _) = old.split_at_checked(old_limit).context("valid slice")?;
    let (new, _) = new.split_at_checked(new_limit).context("valid slice")?;

    if old.is_empty() && new.is_empty() {
        Ok(vec![])
    } else {
        let data = new.iter().copied().flat_map(<[u32; 5]>::from).collect();
        Ok(vec![SemanticTokensEdit {
            start: common_prefix
                .checked_mul(5)
                .context("no overflow")?
                .try_into()?,
            delete_count: old
                .len()
                .checked_mul(5)
                .context("no overflow")?
                .try_into()?,
            data: Some(data),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from(token: (u32, u32, u32, u32, u32)) -> SemanticToken {
        SemanticToken {
            delta_line: token.0,
            delta_start: token.1,
            length: token.2,
            token_type: token.3,
            token_modifiers_bitset: token.4,
        }
    }

    #[test]
    fn diff_insert_at_end() {
        let before = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];
        let after = [
            from((1, 2, 3, 4, 5)),
            from((6, 7, 8, 9, 10)),
            from((11, 12, 13, 14, 15)),
        ];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 10,
                delete_count: 0,
                data: Some(vec![11, 12, 13, 14, 15])
            }
        );
    }

    #[test]
    fn diff_insert_at_beginning() {
        let before = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];
        let after = [
            from((11, 12, 13, 14, 15)),
            from((1, 2, 3, 4, 5)),
            from((6, 7, 8, 9, 10)),
        ];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 0,
                delete_count: 0,
                data: Some(vec![11, 12, 13, 14, 15])
            }
        );
    }

    #[test]
    fn diff_insert_in_middle() {
        let before = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];
        let after = [
            from((1, 2, 3, 4, 5)),
            from((10, 20, 30, 40, 50)),
            from((60, 70, 80, 90, 100)),
            from((6, 7, 8, 9, 10)),
        ];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 5,
                delete_count: 0,
                data: Some(vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100])
            }
        );
    }

    #[test]
    fn diff_remove_from_end() {
        let before = [
            from((1, 2, 3, 4, 5)),
            from((6, 7, 8, 9, 10)),
            from((11, 12, 13, 14, 15)),
        ];
        let after = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 10,
                delete_count: 5,
                data: Some(vec![])
            }
        );
    }

    #[test]
    fn diff_remove_from_beginning() {
        let before = [
            from((11, 12, 13, 14, 15)),
            from((1, 2, 3, 4, 5)),
            from((6, 7, 8, 9, 10)),
        ];
        let after = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 0,
                delete_count: 5,
                data: Some(vec![])
            }
        );
    }

    #[test]
    fn diff_remove_from_middle() {
        let before = [
            from((1, 2, 3, 4, 5)),
            from((10, 20, 30, 40, 50)),
            from((60, 70, 80, 90, 100)),
            from((6, 7, 8, 9, 10)),
        ];
        let after = [from((1, 2, 3, 4, 5)), from((6, 7, 8, 9, 10))];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits[0],
            SemanticTokensEdit {
                start: 5,
                delete_count: 10,
                data: Some(vec![])
            }
        );
    }
}
