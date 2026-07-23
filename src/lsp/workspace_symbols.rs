use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use gen_lsp_types::{
    BaseSymbolInformation, Location, Position, Range, SymbolKind, WorkspaceSymbol,
    WorkspaceSymbolLocation, WorkspaceSymbolParams,
};

use crate::lsp::server::AllWorkspaces;

use super::Client;

/// Response limit.
///
/// similar to rust-analyzer and clangd limit, returning all the symbols
/// can totally overwhelm a slower client.
static LIMIT: usize = 128;

#[expect(clippy::unnecessary_wraps, reason = "simple start")]
#[expect(clippy::iter_over_hash_type, reason = "unclear sorting helps")]
pub fn request(
    _client: &Client,
    workspaces: &AllWorkspaces,
    params: &WorkspaceSymbolParams,
    cancel: &Arc<AtomicBool>,
) -> Result<Vec<WorkspaceSymbol>> {
    let query = &params.query;
    let mut response = Vec::with_capacity(128);
    let mut counter: u64 = 0;
    for (_name, index) in workspaces {
        for (symbol, path) in &index.names {
            if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("java"))
                && let Some((package, class)) = symbol.rsplit_once('.')
            {
                counter = counter.wrapping_add(1);
                if counter.is_multiple_of(128) && cancel.load(Ordering::Relaxed) {
                    return Ok(vec![]);
                }
                if lsp_match(query, class) {
                    response.push(WorkspaceSymbol {
                        location: WorkspaceSymbolLocation::Location(Location {
                            uri: super::server::path_to_uri(&path.to_string_lossy()),
                            range: Range {
                                start: Position::new(0, 0),
                                end: Position::new(0, 0),
                            },
                        }),
                        data: None,
                        base_symbol_information: BaseSymbolInformation {
                            name: class.into(),
                            kind: SymbolKind::File,
                            tags: None,
                            container_name: Some(package.into()),
                        },
                    });
                    if response.len() == LIMIT {
                        return Ok(response);
                    }
                }
            }
        }
    }
    Ok(response)
}

/// Attempts to match as recommended by the LSP spec
///
/// > A good rule of thumb is to match case-insensitive and to simply check that the
/// > characters of *query* appear in their order in a candidate symbol.
/// > Servers shouldn't use prefix, substring, or similar strict matching.
fn lsp_match(query: &str, word: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let mut query_chars = query.chars().flat_map(char::to_lowercase);
    let mut current_char = query_chars.next();

    for word_char in word.chars().flat_map(char::to_lowercase) {
        match current_char {
            Some(ch) => {
                if word_char == ch {
                    current_char = query_chars.next();
                }
            }
            None => {
                return true;
            }
        }
    }
    current_char.is_none()
}
