use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};

use std::sync::{Arc, LazyLock};

use anyhow::{Context as _, Result};
use gen_lsp_types::{
    BaseSymbolInformation, DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, Location,
    SymbolInformation, SymbolKind, SymbolTag, Uri,
};
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, Range, StreamingIterator as _,
};

use crate::support::queries::capture_id;

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &DocumentSymbolParams,
    cancel_token: &Arc<AtomicBool>,
) -> Result<DocumentSymbolResponse> {
    let symbols = nested(client, doc, cancel_token)?;
    if client.supports_hierarchical_symbols() {
        Ok(DocumentSymbolResponse::DocumentSymbolList(symbols))
    } else {
        let mut flat: Vec<SymbolInformation> = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            flatten(&mut flat, client, &params.text_document.uri, &symbol, None);
        }
        Ok(DocumentSymbolResponse::SymbolInformationList(flat))
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
        base_symbol_information: BaseSymbolInformation {
            name: symbol.name.clone(),
            kind: symbol.kind,
            tags: if client.supports_tags() {
                symbol.tags.clone()
            } else {
                None
            },
            container_name: parent.map(|node| node.name.clone()),
        },
        #[expect(deprecated, reason = "unavoidable")]
        deprecated: symbol.deprecated,
        location: Location::new(uri.clone(), symbol.range),
    });
    if let Some(children) = symbol.children.as_ref() {
        for child in children {
            flatten(flat, client, uri, child, Some(symbol));
        }
    }
}

/// internal representation
struct Symbol {
    name: String,
    kind: SymbolKind,
    detail: Option<String>,
    deprecated: bool,
    range: Range,
    selection_range: Range,
    children: Vec<usize>,
}

impl Symbol {
    fn encode(
        &self,
        client: &Client,
        doc: &Document,
        symbols: &Vec<Self>,
    ) -> Result<DocumentSymbol> {
        let subtree: Result<Vec<DocumentSymbol>> = self
            .children
            .iter()
            .map(|index| {
                symbols
                    .get(*index)
                    .expect("valid index")
                    .encode(client, doc, symbols)
            })
            .collect();
        let children = subtree?;
        Ok(DocumentSymbol {
            name: self.name.clone(),
            kind: self.kind,
            detail: self.detail.clone(),
            tags: self.deprecated.then(|| vec![SymbolTag::Deprecated]),
            #[expect(deprecated, reason = "unavoidable")]
            deprecated: None,
            range: client
                .encode_range(&self.range, &doc.line_index)
                .context("valid range")?,
            selection_range: client
                .encode_range(&self.selection_range, &doc.line_index)
                .context("valid range")?,
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
        })
    }
}

fn nested(
    client: &Client,
    doc: &Document,
    cancel_token: &Arc<AtomicBool>,
) -> Result<Vec<DocumentSymbol>> {
    let bytes = doc.text.as_bytes();
    let mut symbols = Vec::new();
    let mut roots = Vec::new();
    let mut stack: Vec<(usize, Range)> = Vec::new();
    let mut cursor = QueryCursor::new();

    // this callback MUST be a separate let-binding. do *NOT* factor into anonymous closure!
    let mut cancellation = |_: &QueryCursorState| {
        if cancel_token.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };

    let mut matches = cursor.matches_with_options(
        &QUERY,
        doc.tree.root_node(),
        bytes,
        QueryCursorOptions::new().progress_callback(&mut cancellation),
    );
    while let Some(hit) = matches.next() {
        let pattern = pattern(hit.pattern_index);
        let node = hit
            .nodes_for_capture_index(*RANGE_CAPTURE)
            .next()
            .context("range capture should exist")?;
        let range = node.range();
        while stack
            .pop_if(|parent| range.start_byte >= parent.1.end_byte)
            .is_some()
        {}
        let selection = hit
            .nodes_for_capture_index(*SELECTION_CAPTURE)
            .next()
            .context("selection capture should exist")?;
        let detail = hit.nodes_for_capture_index(*DETAIL_CAPTURE).next();
        let mut deprecated = false;
        for marker in hit.nodes_for_capture_index(*MARKER_CAPTURE) {
            deprecated |= marker.utf8_text(bytes)? == "Deprecated";
        }
        let mut name = selection.utf8_text(bytes)?.to_owned();
        let mut first_param = true;
        for signature in hit.nodes_for_capture_index(*SIGNATURE_CAPTURE) {
            if signature.is_named() && signature.kind_id() != *DIMENSIONS_KIND {
                if !first_param {
                    name.push(',');
                }
                first_param = false;
            }
            name.push_str(signature.utf8_text(bytes)?);
        }
        let symbol = Symbol {
            name,
            kind: pattern.kind,
            detail: if let Some(detail) = detail {
                Some(detail.utf8_text(bytes)?.trim().into())
            } else {
                None
            },
            deprecated,
            range,
            selection_range: selection.range(),
            children: vec![],
        };

        // add new symbol
        let index = symbols.len();
        symbols.push(symbol);

        if let Some(parent) = stack.last()
            && range.start_byte >= parent.1.start_byte
            && range.end_byte <= parent.1.end_byte
        {
            let parent_symbol = symbols.get_mut(parent.0).context("valid index")?;
            parent_symbol.children.push(index);
        } else {
            roots.push(index);
        }
        stack.push((index, range));
    }
    let mut result = Vec::new();
    for index in roots {
        let symbol = symbols.get(index).context("valid index")?;
        result.push(symbol.encode(client, doc, &symbols)?);
    }
    Ok(result)
}

/// single compiled pattern
struct Pattern {
    /// kind of symbol
    kind: SymbolKind,
}

/// Look up rule by pattern index
#[must_use]
fn pattern(index: usize) -> &'static Pattern {
    PATTERNS.get(index).expect("pattern should exist")
}

/// compiled query that matches all symbol patterns
static QUERY: LazyLock<Query> = LazyLock::new(|| {
    Query::new(
        &crate::support::language(),
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
        let mut kind: Option<SymbolKind> = None;
        let props = QUERY.property_settings(index);
        for prop in props {
            let key = prop.key.as_ref();
            let value = prop.value.as_deref();
            match key {
                "symbol.kind" => {
                    let code = value
                        .expect("kind should have a value")
                        .parse::<u32>()
                        .expect("kind should be an integer");
                    kind = Some(
                        SymbolKind::try_from(code).expect("kind should be a valid SymbolKind"),
                    );
                }
                _ => panic!("{key}: unknown metadata key"),
            }
        }
        patterns.push(Pattern {
            kind: kind.expect("pattern should have a kind"),
        });
    }
    patterns
});

static RANGE_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "range"));

static SELECTION_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "selection"));

static MARKER_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "marker"));

static SIGNATURE_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "signature"));

static DETAIL_CAPTURE: LazyLock<u32> = LazyLock::new(|| capture_id(&QUERY, "detail"));

static DIMENSIONS_KIND: LazyLock<u16> = LazyLock::new(|| {
    let lang = crate::support::language();
    lang.id_for_node_kind("dimensions", true)
});
