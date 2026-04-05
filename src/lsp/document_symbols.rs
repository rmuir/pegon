use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use ls_types::{
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, Location, SymbolInformation,
    SymbolKind, SymbolTag, Uri,
};
use tree_sitter::{Query, QueryCursor, Range, StreamingIterator as _};

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

// internal representation
struct Symbol {
    name: String,
    kind: SymbolKind,
    detail: Option<String>,
    deprecated: bool,
    #[expect(unused, reason = "TODO")]
    flags: u16,
    range: Range,
    selection_range: Range,
    children: Vec<usize>,
}

impl Symbol {
    fn encode(&self, client: &Client, doc: &Document, symbols: &Vec<Self>) -> DocumentSymbol {
        let children: Vec<DocumentSymbol> = self
            .children
            .iter()
            .map(|index| {
                symbols
                    .get(*index)
                    .expect("valid index")
                    .encode(client, doc, symbols)
            })
            .collect();
        DocumentSymbol {
            name: self.name.clone(),
            kind: self.kind,
            detail: self.detail.clone(),
            tags: self.deprecated.then(|| vec![SymbolTag::DEPRECATED]),
            #[expect(deprecated, reason = "unavoidable")]
            deprecated: None,
            range: client
                .encode_range(&self.range, &doc.line_index)
                .expect("valid range"),
            selection_range: client
                .encode_range(&self.selection_range, &doc.line_index)
                .expect("valid range"),
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
        }
    }
}

fn nested(client: &Client, doc: &Document) -> Result<Vec<DocumentSymbol>> {
    let bytes = doc.text.as_bytes();
    let mut symbols = Vec::new();
    let mut roots = Vec::new();
    let mut stack: Vec<(usize, Range)> = Vec::new();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&QUERY, doc.tree.root_node(), bytes);
    while let Some(hit) = matches.next() {
        let pattern = pattern(hit.pattern_index);
        let range = hit
            .nodes_for_capture_index(*RANGE_CAPTURE)
            .next()
            .context("range capture should exist")?;
        let bounds = range.range();
        while stack
            .pop_if(|parent| bounds.start_byte >= parent.1.end_byte)
            .is_some()
        {}
        let selection = hit
            .nodes_for_capture_index(*SELECTION_CAPTURE)
            .next()
            .context("selection capture should exist")?;
        let detail = hit.nodes_for_capture_index(*DETAIL_CAPTURE).next();
        let deprecated = hit.nodes_for_capture_index(*DEPRECATED_CAPTURE).next();
        let mut flags: u16 = 0;
        let modifiers = hit.nodes_for_capture_index(*MODIFIER_CAPTURE);
        for modifier in modifiers {
            flags |= match modifier.utf8_text(bytes)? {
                "public" => access_flags::ACC_PUBLIC,
                "protected" => access_flags::ACC_PROTECTED,
                "private" => access_flags::ACC_PRIVATE,
                "abstract" => access_flags::ACC_ABSTRACT,
                "static" => access_flags::ACC_STATIC,
                "final" => access_flags::ACC_FINAL,
                "strictfp" => access_flags::ACC_STRICT,
                "synchronized" => access_flags::ACC_SYNCHRONIZED,
                "native" => access_flags::ACC_NATIVE,
                "transient" => access_flags::ACC_TRANSIENT,
                "volatile" => access_flags::ACC_VOLATILE,
                _ => 0,
            }
        }
        let mut name = selection.utf8_text(bytes)?.to_owned();
        if let Some(detail) = detail {
            name.push_str(detail.utf8_text(bytes)?.trim());
        }
        let symbol = Symbol {
            name,
            flags,
            detail: None,
            kind: pattern.kind,
            deprecated: deprecated.is_some(),
            range: bounds,
            selection_range: selection.range(),
            children: vec![],
        };

        // add new symbol
        let index = symbols.len();
        symbols.push(symbol);

        if let Some(parent) = stack.last()
            && bounds.start_byte >= parent.1.start_byte
            && bounds.end_byte <= parent.1.end_byte
        {
            let node: &mut Symbol = symbols.get_mut(parent.0).expect("valid index");
            node.children.push(index);
        } else {
            roots.push(index);
        }
        stack.push((index, bounds));
    }
    let mut result: Vec<DocumentSymbol> = Vec::new();
    for index in roots {
        let symbol = symbols.get(index).expect("valid index");
        result.push(symbol.encode(client, doc, &symbols));
    }
    Ok(result)
}

mod access_flags {
    /// public class, field, method, or inner class
    pub const ACC_PUBLIC: u16 = 0x0001;
    /// private field, method, or inner class
    pub const ACC_PRIVATE: u16 = 0x0002;
    /// protected field, method, or inner class
    pub const ACC_PROTECTED: u16 = 0x0004;
    /// static field, method, or inner class
    pub const ACC_STATIC: u16 = 0x0008;
    /// final class, field, method, inner class, parameter
    pub const ACC_FINAL: u16 = 0x0010;
    /// synchronized method
    pub const ACC_SYNCHRONIZED: u16 = 0x0020;
    /// volatile field
    pub const ACC_VOLATILE: u16 = 0x0040;
    /// transient field
    pub const ACC_TRANSIENT: u16 = 0x0080;
    /// native method
    pub const ACC_NATIVE: u16 = 0x0100;
    /// interface class, inner class
    #[expect(unused, reason = "TODO")]
    pub const ACC_INTERFACE: u16 = 0x0200;
    /// abstract class, method, inner class
    pub const ACC_ABSTRACT: u16 = 0x0400;
    /// strictfp method
    pub const ACC_STRICT: u16 = 0x0800;
    /// annotation class, inner class
    #[expect(unused, reason = "TODO")]
    pub const ACC_ANNOTATION: u16 = 0x2000;
    /// enum class, field, inner class
    #[expect(unused, reason = "TODO")]
    pub const ACC_ENUM: u16 = 0x4000;
    /// module class
    #[expect(unused, reason = "TODO")]
    pub const ACC_MODULE: u16 = 0x8000;
    /// mandated parameter
    #[expect(unused, reason = "TODO")]
    pub const ACC_MANDATED: u16 = 0x8000;
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

static MODIFIER_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("modifier")
        .expect("modifier capture should exist")
});

static DETAIL_CAPTURE: LazyLock<u32> = LazyLock::new(|| {
    QUERY
        .capture_index_for_name("detail")
        .expect("detail capture should exist")
});
