use core::ops::ControlFlow;
use core::sync::atomic::{AtomicBool, Ordering};

use std::sync::LazyLock;

use anyhow::{Context as _, Result};
use gen_lsp_types::{
    BaseSymbolInformation, DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, Location,
    SymbolInformation, SymbolKind, SymbolTag, Uri,
};
use tree_sitter::{
    Query, QueryCursor, QueryCursorOptions, QueryCursorState, Range, StreamingIterator as _,
};

use crate::java_constants::kinds;
use crate::java_queries::symbols::captures;
use crate::java_queries::symbols::properties::{PATTERN_COUNT, PROPERTIES, PROPERTIES_BY_PATTERN};
use crate::support::queries::const_array_from_fn;

use super::{Client, server::Document};

pub fn request(
    client: &Client,
    doc: &Document,
    params: &DocumentSymbolParams,
    cancel: &AtomicBool,
) -> Result<Option<DocumentSymbolResponse>> {
    let symbols = nested(client, doc, cancel)?;
    if client.supports_hierarchical_symbols() {
        Ok(Some(DocumentSymbolResponse::DocumentSymbolList(symbols)))
    } else {
        let mut flat: Vec<SymbolInformation> = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            flatten(&mut flat, client, &params.text_document.uri, &symbol, None);
        }
        Ok(Some(DocumentSymbolResponse::SymbolInformationList(flat)))
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

fn nested(client: &Client, doc: &Document, cancel: &AtomicBool) -> Result<Vec<DocumentSymbol>> {
    let bytes = doc.text.as_bytes();
    let mut symbols = Vec::with_capacity(16);
    let mut roots = Vec::with_capacity(16);
    let mut stack: Vec<(usize, Range)> = Vec::with_capacity(16);
    let mut cursor = QueryCursor::new();

    // this callback MUST be a separate let-binding. do *NOT* factor into anonymous closure!
    let mut cancellation = |_: &QueryCursorState| {
        if cancel.load(Ordering::Relaxed) {
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
            .nodes_for_capture_index(captures::RANGE)
            .next()
            .context("range capture should exist")?;
        let range = node.range();
        while stack
            .pop_if(|parent| range.start_byte >= parent.1.end_byte)
            .is_some()
        {}
        let selection = hit
            .nodes_for_capture_index(captures::SELECTION)
            .next()
            .context("selection capture should exist")?;
        let detail = hit.nodes_for_capture_index(captures::DETAIL).next();
        let mut deprecated = false;
        for marker in hit.nodes_for_capture_index(captures::MARKER) {
            deprecated |= marker.utf8_text(bytes)? == "Deprecated";
        }
        let mut name = selection.utf8_text(bytes)?.to_owned();
        let mut first_param = true;
        for signature in hit.nodes_for_capture_index(captures::SIGNATURE) {
            if signature.is_named() && signature.kind_id() != kinds::DIMENSIONS {
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
    let mut result = Vec::with_capacity(roots.len());
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
#[expect(clippy::indexing_slicing, reason = "compile time safety")]
const fn pattern(index: usize) -> &'static Pattern {
    &PATTERNS[index]
}

/// array of pattern metadata from `QUERY` by index
#[expect(clippy::indexing_slicing, reason = "compile time safety")]
#[expect(clippy::arithmetic_side_effects, reason = "compile time safety")]
const PATTERNS: [Pattern; PATTERN_COUNT] = const_array_from_fn!(to_pattern, PATTERN_COUNT);

#[expect(clippy::indexing_slicing, reason = "compile time safety")]
#[expect(clippy::arithmetic_side_effects, reason = "compile time safety")]
const fn to_pattern(pattern: usize) -> Pattern {
    let range = &PROPERTIES_BY_PATTERN[pattern];
    let mut index = range.start;
    let mut kind: Option<SymbolKind> = None;
    while index < range.end {
        let property = PROPERTIES[index];
        match property.0.as_bytes() {
            b"symbol.kind" => kind = Some(to_kind(property.1)),
            _ => panic!("unknown property key"),
        }
        index += 1;
    }
    Pattern {
        kind: kind.expect("kind should be set"),
    }
}

const fn to_kind(string: &str) -> SymbolKind {
    match string.as_bytes() {
        b"file" => SymbolKind::File,
        b"module" => SymbolKind::Module,
        b"namespace" => SymbolKind::Namespace,
        b"package" => SymbolKind::Package,
        b"class" => SymbolKind::Class,
        b"method" => SymbolKind::Method,
        b"property" => SymbolKind::Property,
        b"field" => SymbolKind::Field,
        b"constructor" => SymbolKind::Constructor,
        b"enum" => SymbolKind::Enum,
        b"interface" => SymbolKind::Interface,
        b"function" => SymbolKind::Function,
        b"variable" => SymbolKind::Variable,
        b"constant" => SymbolKind::Constant,
        b"string" => SymbolKind::String,
        b"number" => SymbolKind::Number,
        b"boolean" => SymbolKind::Boolean,
        b"array" => SymbolKind::Array,
        b"object" => SymbolKind::Object,
        b"key" => SymbolKind::Key,
        b"null" => SymbolKind::Null,
        b"enum_member" => SymbolKind::EnumMember,
        b"struct" => SymbolKind::Struct,
        b"event" => SymbolKind::Event,
        b"operator" => SymbolKind::Operator,
        b"type_parameter" => SymbolKind::TypeParameter,
        _ => panic!("unknown kind"),
    }
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

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        BaseSymbolInformation, DidOpenTextDocumentNotification, DidOpenTextDocumentParams,
        DocumentSymbolParams, DocumentSymbolRequest, DocumentSymbolResponse, Location,
        PartialResultParams, Position, Range, SymbolInformation, SymbolKind,
        TextDocumentIdentifier, TextDocumentItem, WorkDoneProgressParams,
    };
    use indoc::indoc;

    use crate::lsp::test_client::TestClient;

    /// simple document, flat results
    #[test]
    fn flat() {
        let client = TestClient::default();
        client.notify::<DidOpenTextDocumentNotification>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: "file:///Foo.java".into(),
                language_id: "java".into(),
                version: 0,
                text: indoc! {"
                public class foo {
                    public void bar(int x) {
                    }
                }
            "}
                .into(),
            },
        });
        let result = client
            .request::<DocumentSymbolRequest>(DocumentSymbolParams {
                text_document: TextDocumentIdentifier::new("file:///Foo.java".into()),
                partial_result_params: PartialResultParams::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        assert_eq!(
            result,
            DocumentSymbolResponse::SymbolInformationList(vec![
                SymbolInformation {
                    base_symbol_information: BaseSymbolInformation {
                        name: "foo".into(),
                        kind: SymbolKind::Class,
                        tags: None,
                        container_name: None
                    },
                    #[expect(deprecated, reason = "unavoidable")]
                    deprecated: None,
                    location: Location {
                        uri: "file:///Foo.java".into(),
                        range: Range::new(Position::new(0, 0), Position::new(3, 1)),
                    },
                },
                SymbolInformation {
                    base_symbol_information: BaseSymbolInformation {
                        name: "bar(int)".into(),
                        kind: SymbolKind::Method,
                        tags: None,
                        container_name: Some("foo".into())
                    },
                    #[expect(deprecated, reason = "unavoidable")]
                    deprecated: None,
                    location: Location {
                        uri: "file:///Foo.java".into(),
                        range: Range::new(Position::new(1, 4), Position::new(2, 5)),
                    },
                }
            ])
        );
    }
}
