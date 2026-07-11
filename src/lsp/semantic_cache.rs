#![allow(clippy::allow_attributes_without_reason)]
#![allow(unused)]
use anyhow::{Context as _, Error};
use gen_lsp_types::SemanticTokensEdit;

/// byte-level diff of semantic tokens
/// see rust-analyzer implementation for inspiration
fn diff(old: &[u32], new: &[u32]) -> Result<Vec<SemanticTokensEdit>, Error> {
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
        Ok(vec![SemanticTokensEdit {
            start: common_prefix.try_into()?,
            delete_count: old.len().try_into()?,
            data: Some(new.into()),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_insert_at_end() {
        let before = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let after = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits,
            vec![SemanticTokensEdit {
                start: 10,
                delete_count: 0,
                data: Some(vec![11, 12, 13, 14, 15])
            }]
        );
    }

    #[test]
    fn diff_insert_at_beginning() {
        let before = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let after = [11, 12, 13, 14, 15, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits,
            vec![SemanticTokensEdit {
                start: 0,
                delete_count: 0,
                data: Some(vec![11, 12, 13, 14, 15])
            }]
        );
    }

    #[test]
    fn diff_insert_in_middle() {
        let before = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let after = [
            1, 2, 3, 4, 5, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 6, 7, 8, 9, 10,
        ];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits,
            vec![SemanticTokensEdit {
                start: 5,
                delete_count: 0,
                data: Some(vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100])
            }]
        );
    }

    #[test]
    fn diff_remove_from_end() {
        let before = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let after = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits,
            vec![SemanticTokensEdit {
                start: 10,
                delete_count: 5,
                data: Some(vec![])
            }]
        );
    }

    #[test]
    fn diff_remove_from_beginning() {
        let before = [11, 12, 13, 14, 15, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let after = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits,
            vec![SemanticTokensEdit {
                start: 0,
                delete_count: 5,
                data: Some(vec![])
            }]
        );
    }

    #[test]
    fn diff_remove_from_middle() {
        let before = [
            1, 2, 3, 4, 5, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 6, 7, 8, 9, 10,
        ];
        let after = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let edits = diff(&before, &after).unwrap();
        assert_eq!(
            edits,
            vec![SemanticTokensEdit {
                start: 5,
                delete_count: 10,
                data: Some(vec![])
            }]
        );
    }
}
