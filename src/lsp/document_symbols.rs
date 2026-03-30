use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use ls_types::{
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, Location, SymbolInformation,
    SymbolKind, SymbolTag, Uri,
};
use tree_sitter::{Query, QueryCursor, StreamingIterator as _};

use crate::lsp::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &DocumentSymbolParams,
) -> Result<DocumentSymbolResponse> {
    let symbols = nested(client, doc)?;
    if client.supports_hierarchical_symbols() {
        Ok(DocumentSymbolResponse::Nested(symbols))
    } else {
        let mut flat: Vec<SymbolInformation> = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            flatten(&mut flat, client, &params.text_document.uri, &symbol, None);
        }
        Ok(DocumentSymbolResponse::Flat(flat))
    }
}

fn flatten(
    flat: &mut Vec<SymbolInformation>,
    client: &Client,
    uri: &Uri,
    symbol: &DocumentSymbol,
    parent: Option<&DocumentSymbol>,
) {
    flat.push(SymbolInformation {
        name: symbol.name.clone(),
        kind: symbol.kind,
        tags: if client.supports_tags() {
            symbol.tags.clone()
        } else {
            None
        },
        #[expect(deprecated, reason = "unavoidable")]
        deprecated: symbol.deprecated,
        location: Location {
            uri: uri.clone(),
            range: symbol.range,
        },
        container_name: parent.map(|node| node.name.clone()),
    });
    if let Some(children) = symbol.children.as_ref() {
        for child in children {
            flatten(flat, client, uri, child, Some(symbol));
        }
    }
}

fn nested(client: &Client, doc: &Document) -> Result<Vec<DocumentSymbol>> {
    let bytes = doc.text.as_bytes();
    let mut symbols = Vec::new();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&QUERY, doc.tree.root_node(), bytes);
    while let Some(hit) = matches.next() {
        let pattern = pattern(hit.pattern_index);
        let range = hit
            .nodes_for_capture_index(*RANGE_CAPTURE)
            .next()
            .context("range capture should exist")?;
        let bounds = range.range();
        let selection = hit
            .nodes_for_capture_index(*SELECTION_CAPTURE)
            .next()
            .context("selection capture should exist")?;
        let detail = hit.nodes_for_capture_index(*DETAIL_CAPTURE).next();
        let deprecated = hit.nodes_for_capture_index(*DEPRECATED_CAPTURE).next();
        let mut name = selection.utf8_text(bytes)?.to_owned();
        if let Some(detail) = detail {
            name.push_str(detail.utf8_text(bytes)?);
        }
        let symbol = DocumentSymbol {
            name,
            detail: None,
            kind: pattern.kind,
            tags: deprecated.is_some().then(|| vec![SymbolTag::DEPRECATED]),
            #[expect(deprecated, reason = "unavoidable")]
            deprecated: None,
            range: client
                .encode_range(&bounds, &doc.line_index)
                .expect("can encode range"),
            selection_range: client
                .encode_range(&selection.range(), &doc.line_index)
                .expect("can encode range"),
            children: None,
        };
        symbols.push(symbol);
    }
    Ok(symbols)
}

/// single compiled pattern
struct Pattern {
    /// kind of symbol
    kind: SymbolKind,
}

// Look up rule by pattern index
#[must_use]
fn pattern(index: usize) -> &'static Pattern {
    PATTERNS.get(index).expect("pattern should exist")
}

/// compiled query that matches all symbol patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::LANGUAGE.into(),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/queries/java/symbols.scm"
        )),
    )
    .expect("query should compile")
});

/// array of rules indexed by patterns of `QUERY`
static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    let count = QUERY.pattern_count();
    let mut patterns = Vec::with_capacity(count);
    for index in 0..count {
        let mut kind: Option<&str> = None;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            #[expect(clippy::single_match, reason = "TODO")]
            match key {
                "kind" => {
                    kind = value;
                }
                _ => {}
            }
        }
        patterns.push(Pattern {
            kind: kind
                .expect("pattern should have a kind")
                .try_into()
                .expect("should map to lsp kind"),
        });
    }
    patterns
});

static RANGE_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("range")
        .expect("range capture should exist")
});

static SELECTION_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("selection")
        .expect("selection capture should exist")
});

static DEPRECATED_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("deprecated")
        .expect("deprecated capture should exist")
});

static DETAIL_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("detail")
        .expect("detail capture should exist")
});
