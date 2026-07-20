//! Client capabilities and encoding
//!
//! Not all LSP clients are equal, they can have different capabilities
//! and use different text encodings. This code is the "bending-over-backwards"
//! part needed in order to give the **editor** the best performance.
//!
//! Treesitter stores line + column information, but also raw byte offsets.
//! The byte offsets are lost in transmission since they aren't in the LSP
//! protocol. Additionally, the client may be using a different unicode
//! encoding. The SIMD-optimized [`LineIndex`] from `rust-analyzer` handles
//! these problems with no sweat.

use core::convert::From;

use gen_lsp_types::{
    ClientCapabilities, CodeActionClientCapabilities, DefinitionClientCapabilities,
    DiagnosticClientCapabilities, DiagnosticsCapabilities, DocumentHighlightClientCapabilities,
    DocumentSymbolClientCapabilities, FoldingRangeClientCapabilities, HoverClientCapabilities,
    InitializeParams, InlayHintClientCapabilities, MarkupKind, Position, PositionEncodingKind,
    PublishDiagnosticsClientCapabilities, SelectionRangeClientCapabilities,
    SemanticTokensClientCapabilities, TextDocumentClientCapabilities,
    TextDocumentContentChangeEvent, WindowClientCapabilities, WorkspaceClientCapabilities,
    WorkspaceEditClientCapabilities, WorkspaceFolder, WorkspaceFolders,
    WorkspaceSymbolClientCapabilities,
};
use line_index::{LineCol, LineIndex, TextSize, WideEncoding, WideLineCol};
use tree_sitter::Point;

/// A Language Server Protocol client
pub struct Client {
    /// Parameters sent by the client in the `initialize` request.
    ///
    /// the parameters describe various optional client capabilities
    /// which can be used for better performance and more features.
    init_params: InitializeParams,

    /// The client's preferred offset encoding.
    ///
    /// Supporting this only speeds up the client: java and javascript
    /// clients will prefer UTF-16, most everyone else will use UTF-8.
    /// Maybe somewhere there is a python editor using UTF-32!
    ///
    /// Although treesitter supports parsing tree with crazy encodings,
    /// we don't go that far: UTF-8 is used internally for sanity, and
    /// the character offsets are adjusted when (de)serializing requests
    /// and responses.
    encoding: Encoding,
}

impl Client {
    /// Create a new client with the parameters it sent in the
    /// initialize request.
    pub fn new(init_params: InitializeParams) -> Self {
        let encoding = Encoding::preferred(&init_params.capabilities);

        Self {
            init_params,
            encoding,
        }
    }

    /// Encodes a tree-sitter UTF-8 range into an LSP range (client's encoding)
    ///
    /// For the UTF-8 encoding, this is a no-op. for other encodings the index
    /// must be used.
    pub fn encode_range(
        &self,
        range: &tree_sitter::Range,
        index: &LineIndex,
    ) -> Option<gen_lsp_types::Range> {
        Some(gen_lsp_types::Range {
            start: self.encode_point(&range.start_point, index)?,
            end: self.encode_point(&range.end_point, index)?,
        })
    }

    /// Encodes a tree-sitter UTF-8 point into an LSP position (client's encoding)
    ///
    /// For the UTF-8 encoding, this is a no-op. for other encodings the index
    /// must be used.
    fn encode_point(&self, point: &Point, index: &LineIndex) -> Option<Position> {
        // check bounds are within u32
        let line_col = LineCol {
            line: u32::try_from(point.row).ok()?,
            col: u32::try_from(point.column).ok()?,
        };

        // comes from treesitter, bounds within document should be correct
        debug_assert!(index.offset(line_col).is_some());

        // translate using the index for wide encodings
        let (line, character) = match self.encoding {
            Encoding::Utf8 => (line_col.line, line_col.col),
            Encoding::Utf16 => {
                let wide = index.to_wide(WideEncoding::Utf16, line_col)?;
                (wide.line, wide.col)
            }
            Encoding::Utf32 => {
                let wide = index.to_wide(WideEncoding::Utf32, line_col)?;
                (wide.line, wide.col)
            }
        };
        Some(Position { line, character })
    }

    /// Decodes an LSP document change into a treesitter Range.
    ///
    /// We specify incremental sync, but it is unclear from the spec
    /// if clients are allowed to send us a full sync. If it happens,
    /// convert it into a full document range.
    pub fn decode_change(
        &self,
        change: &TextDocumentContentChangeEvent,
        index: &LineIndex,
    ) -> Option<tree_sitter::Range> {
        match change {
            TextDocumentContentChangeEvent::TextDocumentContentChangePartial(partial) => {
                self.decode_range(&partial.range, index)
            }
            TextDocumentContentChangeEvent::TextDocumentContentChangeWholeDocument(_) => {
                let end = index.try_line_col(index.len())?;
                Some(tree_sitter::Range {
                    start_byte: 0,
                    end_byte: index.len().into(),
                    start_point: Point { row: 0, column: 0 },
                    end_point: Point {
                        row: end.line as usize,
                        column: end.col as usize,
                    },
                })
            }
        }
    }

    /// Decodes an LSP range into a treesitter Range.
    ///
    /// Treesitter range contains more information than an LSP range,
    /// so the byte offsets must be looked up from the index.
    pub fn decode_range(
        &self,
        range: &gen_lsp_types::Range,
        index: &LineIndex,
    ) -> Option<tree_sitter::Range> {
        let start = self.decode_pos(range.start, index)?;
        let end = self.decode_pos(range.end, index)?;
        Some(tree_sitter::Range {
            start_byte: index.offset(start)?.into(),
            end_byte: index.offset(end)?.into(),
            start_point: Point {
                row: start.line as usize,
                column: start.col as usize,
            },
            end_point: Point {
                row: end.line as usize,
                column: end.col as usize,
            },
        })
    }

    /// Decodes an LSP Position (line+col) into a UTF-8 line+col.
    pub fn decode_pos(&self, position: Position, index: &LineIndex) -> Option<LineCol> {
        match self.encoding {
            Encoding::Utf8 => Some(LineCol {
                line: position.line,
                col: position.character,
            }),
            Encoding::Utf16 => index.to_utf8(
                WideEncoding::Utf16,
                WideLineCol {
                    line: position.line,
                    col: position.character,
                },
            ),
            Encoding::Utf32 => index.to_utf8(
                WideEncoding::Utf32,
                WideLineCol {
                    line: position.line,
                    col: position.character,
                },
            ),
        }
    }

    /// Converts from an byte offset to a row/column
    pub fn to_point(offset: usize, line_index: &LineIndex) -> Option<Point> {
        let offset = TextSize::try_from(offset).ok()?;
        let position = line_index.try_line_col(offset)?;
        Some(Point {
            row: position.line as usize,
            column: position.col as usize,
        })
    }

    /// Client's preferred position encoding.
    ///
    /// This only speeds up the client: java and javascript clients
    /// will prefer UTF-16, most everyone else will use UTF-8. Maybe
    /// somewhere there is a python editor using UTF-32!
    ///
    /// Although treesitter supports parsing tree with crazy encodings,
    /// we don't go that far: UTF-8 is used internally for sanity, and
    /// the character offsets are adjusted when (de)serializing requests
    /// and responses.
    pub fn negotiated_encoding(&self) -> PositionEncodingKind {
        self.encoding.into()
    }

    /// Does the client support the pull diagnostics model?
    ///
    /// This is less error-prone than the push model since it
    /// can be treated by the client like any other request.
    /// it is more efficient because it supports some basic
    /// caching (similar to HTTP 304) and because the client
    /// can choose when to make requests, versus having them
    /// pushed on every didChange.
    pub fn supports_pull_diagnostics(&self) -> bool {
        self.pull_diagnostics().is_some()
    }

    /// Does the client support markdown-formatted diagnostics?
    ///
    /// Only supported for pull diagnostics
    pub fn supports_markup_messages(&self, push: bool) -> bool {
        !push && (|| -> _ { self.pull_diagnostics()?.markup_message_support })().unwrap_or_default()
    }

    /// Does the client support receiving additional ranges
    /// with related information ("context")?
    pub fn supports_related_information(&self, push: bool) -> bool {
        (|| self.diagnostics_capabilities(push)?.related_information)().unwrap_or_default()
    }

    /// Does the client support receiving URLs for more information
    /// on the diagnostic code?
    pub fn supports_code_description(&self, push: bool) -> bool {
        (|| {
            self.diagnostics_capabilities(push)?
                .code_description_support
        })()
        .unwrap_or_default()
    }

    /// Does the client support preserving data between diagnostics
    /// and code actions?
    pub fn supports_data(&self, push: bool) -> bool {
        (|| self.diagnostics_capabilities(push)?.data_support)().unwrap_or_default()
    }

    /// Does the client support locationlink for definition?
    pub fn supports_links(&self) -> bool {
        (|| self.definition()?.link_support)().unwrap_or_default()
    }

    /// Does the client support hierarchical document symbols?
    pub fn supports_hierarchical_symbols(&self) -> bool {
        (|| {
            self.document_symbols()?
                .hierarchical_document_symbol_support
        })()
        .unwrap_or_default()
    }

    /// Does the client support tags on flat document symbols?
    pub fn supports_tags(&self) -> bool {
        (|| self.document_symbols()?.tag_support.as_ref())().is_some()
    }

    /// Does client supports receiving the document version
    /// in push diagnostics?
    ///
    /// Not relevant to pull diagnostics where the version is implicit.
    pub fn supports_version(&self) -> bool {
        (|| self.push_diagnostics()?.version_support)().unwrap_or_default()
    }

    /// Does the client preserve code action data between request and resolve?
    pub fn supports_code_action_data(&self) -> bool {
        (|| self.code_actions()?.data_support)().unwrap_or_default()
    }

    /// Does client support resolving workspace edits on code actions?
    pub fn supports_code_action_resolve_edit(&self) -> bool {
        (|| {
            Some(
                self.code_actions()?
                    .resolve_support
                    .as_ref()?
                    .properties
                    .contains(&"edit".into()),
            )
        })()
        .unwrap_or_default()
    }

    /// Does the client prefer markdown format for hover documentation?
    pub fn prefers_hover_markdown(&self) -> bool {
        (|| Some(*self.hover()?.content_format.as_ref()?.first()? == MarkupKind::Markdown))()
            .unwrap_or_default()
    }

    /// Does the client support resolving inlay hint textedits?
    pub fn supports_inlay_hint_resolve_edit(&self) -> bool {
        (|| {
            Some(
                self.inlay_hints()?
                    .resolve_support
                    .as_ref()?
                    .properties
                    .contains(&"textEdits".into()),
            )
        })()
        .unwrap_or_default()
    }

    /// Does the client support resolving inlay hint locations?
    pub fn supports_inlay_hint_resolve_label_location(&self) -> bool {
        (|| {
            Some(
                self.inlay_hints()?
                    .resolve_support
                    .as_ref()?
                    .properties
                    .contains(&"label.location".into()),
            )
        })()
        .unwrap_or_default()
    }

    /// Does the client support resolving inlay hint locations?
    ///
    /// TODO: neovim not following standard, fix it there
    pub fn supports_inlay_hint_resolve_neovim_location(&self) -> bool {
        (|| {
            Some(
                self.inlay_hints()?
                    .resolve_support
                    .as_ref()?
                    .properties
                    .contains(&"location".into()),
            )
        })()
        .unwrap_or_default()
    }

    /// Does the client support versioned document changes?
    pub fn supports_document_changes(&self) -> bool {
        (|| self.workspace_edit()?.document_changes)().unwrap_or_default()
    }

    /// Does the client support work-done progress?
    pub fn supports_progress(&self) -> bool {
        (|| self.window()?.work_done_progress)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of document synchronization?
    pub fn registers_sync(&self) -> bool {
        (|| {
            self.text_document()?
                .synchronization
                .as_ref()?
                .dynamic_registration
        })()
        .unwrap_or_default()
    }

    /// Does the client support dynamic registration of code actions?
    pub fn registers_code_actions(&self) -> bool {
        (|| self.code_actions()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of definitions?
    pub fn registers_definition(&self) -> bool {
        (|| self.definition()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of diagnostics?
    pub fn registers_diagnostics(&self) -> bool {
        (|| self.pull_diagnostics()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of document highlight?
    pub fn registers_document_highlight(&self) -> bool {
        (|| self.document_highlight()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of document symbols?
    pub fn registers_document_symbols(&self) -> bool {
        (|| self.document_symbols()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of folding range?
    pub fn registers_folding_range(&self) -> bool {
        (|| self.folding_range()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of hover?
    pub fn registers_hover(&self) -> bool {
        (|| self.hover()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of inlay hints?
    pub fn registers_inlay_hints(&self) -> bool {
        (|| self.inlay_hints()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of selection range?
    pub fn registers_selection_range(&self) -> bool {
        (|| self.selection_range()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of semantic tokens?
    pub fn registers_semantic_tokens(&self) -> bool {
        (|| self.semantic_tokens()?.dynamic_registration)().unwrap_or_default()
    }

    /// Does the client support dynamic registration of workspace symbols?
    pub fn registers_workspace_symbols(&self) -> bool {
        (|| self.workspace_symbols()?.dynamic_registration)().unwrap_or_default()
    }

    const fn text_document(&self) -> Option<&TextDocumentClientCapabilities> {
        self.init_params.capabilities.text_document.as_ref()
    }

    /// Client's name, to detect when vscode violates its own protocol
    const fn client_name(&self) -> &str {
        if let Some(client_info) = self.init_params.client_info.as_ref() {
            return client_info.name.as_str();
        }
        "<unknown>"
    }

    /// Hacks around buggy vscode client
    ///
    /// It doesn't populate `DiagnosticsCapability` inherited fields for pulls, only for pushes.
    fn diagnostics_capabilities(&self, push: bool) -> Option<&DiagnosticsCapabilities> {
        Some(match self.client_name() {
            "Visual Studio Code" => &self.push_diagnostics()?.diagnostics_capabilities,
            _ => {
                if push {
                    &self.push_diagnostics()?.diagnostics_capabilities
                } else {
                    &self.pull_diagnostics()?.diagnostics_capabilities
                }
            }
        })
    }

    fn pull_diagnostics(&self) -> Option<&DiagnosticClientCapabilities> {
        self.text_document()?.diagnostic.as_ref()
    }

    fn push_diagnostics(&self) -> Option<&PublishDiagnosticsClientCapabilities> {
        self.text_document()?.publish_diagnostics.as_ref()
    }

    fn code_actions(&self) -> Option<&CodeActionClientCapabilities> {
        self.text_document()?.code_action.as_ref()
    }

    fn definition(&self) -> Option<&DefinitionClientCapabilities> {
        self.text_document()?.definition.as_ref()
    }

    fn document_highlight(&self) -> Option<&DocumentHighlightClientCapabilities> {
        self.text_document()?.document_highlight.as_ref()
    }

    fn document_symbols(&self) -> Option<&DocumentSymbolClientCapabilities> {
        self.text_document()?.document_symbol.as_ref()
    }

    fn folding_range(&self) -> Option<&FoldingRangeClientCapabilities> {
        self.text_document()?.folding_range.as_ref()
    }

    fn hover(&self) -> Option<&HoverClientCapabilities> {
        self.text_document()?.hover.as_ref()
    }

    fn inlay_hints(&self) -> Option<&InlayHintClientCapabilities> {
        self.text_document()?.inlay_hint.as_ref()
    }

    fn selection_range(&self) -> Option<&SelectionRangeClientCapabilities> {
        self.text_document()?.selection_range.as_ref()
    }

    fn semantic_tokens(&self) -> Option<&SemanticTokensClientCapabilities> {
        self.text_document()?.semantic_tokens.as_ref()
    }

    const fn window(&self) -> Option<&WindowClientCapabilities> {
        self.init_params.capabilities.window.as_ref()
    }

    const fn workspace(&self) -> Option<&WorkspaceClientCapabilities> {
        self.init_params.capabilities.workspace.as_ref()
    }

    fn workspace_edit(&self) -> Option<&WorkspaceEditClientCapabilities> {
        self.workspace()?.workspace_edit.as_ref()
    }

    fn workspace_symbols(&self) -> Option<&WorkspaceSymbolClientCapabilities> {
        self.workspace()?.symbol.as_ref()
    }

    pub fn workspace_folders(&self) -> Vec<WorkspaceFolder> {
        // first look for workspace folders
        if let Some(WorkspaceFolders::WorkspaceFolderList(folders)) = self
            .init_params
            .workspace_folders_initialize_params
            .workspace_folders
            .as_ref()
        {
            return folders.clone();
        }
        // fall back to root_uri
        #[expect(deprecated, reason = "unavoidable")]
        if let Some(root_uri) = self.init_params.root_uri.as_ref() {
            return vec![WorkspaceFolder {
                uri: root_uri.clone(),
                name: "root_uri".into(),
            }];
        }
        // there's no workspace folders, standalone document, or ancient root_path
        // we're not dragging in some crazy URL crates for this.
        vec![]
    }
}

/// Internal representation to simplify logic
///
/// use an enum rather than [`PositionEncodingKind`]'s string
/// <https://github.com/gluon-lang/lsp-types/pull/267>
#[derive(Copy, Clone)]
enum Encoding {
    Utf8,
    Utf16,
    Utf32,
}

impl Encoding {
    fn preferred(capabilities: &ClientCapabilities) -> Self {
        if let Some(general) = &capabilities.general
            && let Some(encodings) = &general.position_encodings
            && let Some(preferred) = encodings.first()
        {
            preferred.into()
        } else {
            Self::Utf16
        }
    }
}

impl From<Encoding> for PositionEncodingKind {
    fn from(value: Encoding) -> Self {
        match value {
            Encoding::Utf8 => Self::UTF8,
            Encoding::Utf16 => Self::UTF16,
            Encoding::Utf32 => Self::UTF32,
        }
    }
}

impl From<&PositionEncodingKind> for Encoding {
    fn from(value: &PositionEncodingKind) -> Self {
        match value.as_str() {
            "utf-8" => Self::Utf8,
            "utf-32" => Self::Utf32,
            _ => Self::Utf16,
        }
    }
}

#[cfg(test)]
mod tests {
    use gen_lsp_types::GeneralClientCapabilities;

    use super::*;

    fn with_encoding(encoding: PositionEncodingKind) -> Client {
        Client::new(InitializeParams {
            capabilities: ClientCapabilities {
                general: Some(GeneralClientCapabilities {
                    position_encodings: Some(vec![encoding]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    fn point(row: usize, column: usize) -> Point {
        Point { row, column }
    }

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    fn lc(line: u32, col: u32) -> LineCol {
        LineCol { line, col }
    }

    #[test]
    fn defaults() {
        let client = Client::new(InitializeParams::default());
        assert_eq!(PositionEncodingKind::UTF16, client.negotiated_encoding());
        assert!(!client.supports_pull_diagnostics());
        assert!(!client.supports_related_information(false));
        assert!(!client.supports_related_information(true));
        assert!(!client.supports_code_description(false));
        assert!(!client.supports_code_description(true));
        assert!(!client.supports_version());
    }

    #[test]
    fn utf8_encode() {
        let utf8 = with_encoding(PositionEncodingKind::UTF8);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(Some(pos(0, 0)), utf8.encode_point(&point(0, 0), &index));
        // 1-byter
        assert_eq!(Some(pos(0, 1)), utf8.encode_point(&point(0, 1), &index));
        // 2-byter
        assert_eq!(Some(pos(0, 3)), utf8.encode_point(&point(0, 3), &index));
        // 3-byter
        assert_eq!(Some(pos(0, 6)), utf8.encode_point(&point(0, 6), &index));
        // 4-byter
        assert_eq!(Some(pos(0, 10)), utf8.encode_point(&point(0, 10), &index));
        // newline
        assert_eq!(Some(pos(1, 0)), utf8.encode_point(&point(1, 0), &index));
        // 1-byter
        assert_eq!(Some(pos(1, 1)), utf8.encode_point(&point(1, 1), &index));
    }

    #[test]
    fn utf8_decode() {
        let utf8 = with_encoding(PositionEncodingKind::UTF8);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(Some(lc(0, 0)), utf8.decode_pos(pos(0, 0), &index));
        // 1-byter
        assert_eq!(Some(lc(0, 1)), utf8.decode_pos(pos(0, 1), &index));
        // 2-byter
        assert_eq!(Some(lc(0, 3)), utf8.decode_pos(pos(0, 3), &index));
        // 3-byter
        assert_eq!(Some(lc(0, 6)), utf8.decode_pos(pos(0, 6), &index));
        // 4-byter
        assert_eq!(Some(lc(0, 10)), utf8.decode_pos(pos(0, 10), &index));
        // newline
        assert_eq!(Some(lc(1, 0)), utf8.decode_pos(pos(1, 0), &index));
        // 1-byter
        assert_eq!(Some(lc(1, 1)), utf8.decode_pos(pos(1, 1), &index));
    }

    #[test]
    fn utf16_encode() {
        let utf16 = with_encoding(PositionEncodingKind::UTF16);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(Some(pos(0, 0)), utf16.encode_point(&point(0, 0), &index));
        // 1-byter
        assert_eq!(Some(pos(0, 1)), utf16.encode_point(&point(0, 1), &index));
        // 2-byter
        assert_eq!(Some(pos(0, 2)), utf16.encode_point(&point(0, 3), &index));
        // 3-byter
        assert_eq!(Some(pos(0, 3)), utf16.encode_point(&point(0, 6), &index));
        // 4-byter
        assert_eq!(Some(pos(0, 5)), utf16.encode_point(&point(0, 10), &index));
        // newline
        assert_eq!(Some(pos(1, 0)), utf16.encode_point(&point(1, 0), &index));
        // 1-byter
        assert_eq!(Some(pos(1, 1)), utf16.encode_point(&point(1, 1), &index));
    }

    #[test]
    fn utf16_decode() {
        let utf16 = with_encoding(PositionEncodingKind::UTF16);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(Some(lc(0, 0)), utf16.decode_pos(pos(0, 0), &index));
        // 1-byter
        assert_eq!(Some(lc(0, 1)), utf16.decode_pos(pos(0, 1), &index));
        // 2-byter
        assert_eq!(Some(lc(0, 3)), utf16.decode_pos(pos(0, 2), &index));
        // 3-byter
        assert_eq!(Some(lc(0, 6)), utf16.decode_pos(pos(0, 3), &index));
        // 4-byter
        assert_eq!(Some(lc(0, 10)), utf16.decode_pos(pos(0, 5), &index));
        // newline
        assert_eq!(Some(lc(1, 0)), utf16.decode_pos(pos(1, 0), &index));
        // 1-byter
        assert_eq!(Some(lc(1, 1)), utf16.decode_pos(pos(1, 1), &index));
    }

    #[test]
    fn utf32_encode() {
        let utf32 = with_encoding(PositionEncodingKind::UTF32);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(Some(pos(0, 0)), utf32.encode_point(&point(0, 0), &index));
        // 1-byter
        assert_eq!(Some(pos(0, 1)), utf32.encode_point(&point(0, 1), &index));
        // 2-byter
        assert_eq!(Some(pos(0, 2)), utf32.encode_point(&point(0, 3), &index));
        // 3-byter
        assert_eq!(Some(pos(0, 3)), utf32.encode_point(&point(0, 6), &index));
        // 4-byter
        assert_eq!(Some(pos(0, 4)), utf32.encode_point(&point(0, 10), &index));
        // newline
        assert_eq!(Some(pos(1, 0)), utf32.encode_point(&point(1, 0), &index));
        // 1-byter
        assert_eq!(Some(pos(1, 1)), utf32.encode_point(&point(1, 1), &index));
    }

    #[test]
    fn utf32_decode() {
        let utf32 = with_encoding(PositionEncodingKind::UTF32);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(Some(lc(0, 0)), utf32.decode_pos(pos(0, 0), &index));
        // 1-byter
        assert_eq!(Some(lc(0, 1)), utf32.decode_pos(pos(0, 1), &index));
        // 2-byter
        assert_eq!(Some(lc(0, 3)), utf32.decode_pos(pos(0, 2), &index));
        // 3-byter
        assert_eq!(Some(lc(0, 6)), utf32.decode_pos(pos(0, 3), &index));
        // 4-byter
        assert_eq!(Some(lc(0, 10)), utf32.decode_pos(pos(0, 4), &index));
        // newline
        assert_eq!(Some(lc(1, 0)), utf32.decode_pos(pos(1, 0), &index));
        // 1-byter
        assert_eq!(Some(lc(1, 1)), utf32.decode_pos(pos(1, 1), &index));
    }
}
