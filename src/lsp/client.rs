use core::convert::From;
use core::ops::Range;

use line_index::{LineCol, LineIndex, WideEncoding, WideLineCol};
use lsp_types::{ClientCapabilities, InitializeParams, Position, PositionEncodingKind};
use tree_sitter::Point;

pub struct Client {
    init_params: InitializeParams,
    encoding: Encoding,
}

impl Client {
    pub(crate) fn new(init_params: InitializeParams) -> Self {
        let encoding = Encoding::preferred(&init_params.capabilities);
        Self {
            init_params,
            encoding,
        }
    }

    // encodes a tree-sitter UTF-8 range into an LSP range (client's encoding)
    // use this to encode client responses.
    pub(crate) fn encode_range(
        &self,
        range: &tree_sitter::Range,
        index: &LineIndex,
    ) -> Option<lsp_types::Range> {
        Some(lsp_types::Range {
            start: self.encode_point(&range.start_point, index)?,
            end: self.encode_point(&range.end_point, index)?,
        })
    }

    // encodes a tree-sitter UTF-8 point into an LSP position (client's encoding)
    // use this to encode client responses.
    pub(crate) fn encode_point(&self, point: &Point, index: &LineIndex) -> Option<Position> {
        // check bounds
        let linecol = LineCol {
            line: u32::try_from(point.row).ok()?,
            col: u32::try_from(point.column).ok()?,
        };

        // translate using the index for wide encodings
        match self.encoding {
            Encoding::Utf8 => Some(Position {
                line: linecol.line,
                character: linecol.col,
            }),
            Encoding::Utf16 => {
                let wide = index.to_wide(WideEncoding::Utf16, linecol)?;
                Some(Position {
                    line: wide.line,
                    character: wide.col,
                })
            }
            Encoding::Utf32 => {
                let wide = index.to_wide(WideEncoding::Utf32, linecol)?;
                Some(Position {
                    line: wide.line,
                    character: wide.col,
                })
            }
        }
    }

    pub(crate) fn decode_pos2(&self, position: Position, index: &LineIndex) -> Option<LineCol> {
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

    /// decodes the LSP position to a range
    /// you must check that it is in-bounds and lies on utf-8 boundary
    pub(crate) fn decode_range(
        &self,
        range: lsp_types::Range,
        line_index: &LineIndex,
    ) -> Option<Range<usize>> {
        if let Some(start) = self.decode_position(range.start, line_index)
            && let Some(end) = self.decode_position(range.end, line_index)
        {
            Some(start..end)
        } else {
            None
        }
    }

    /// decodes the LSP position to an offset
    /// you must check that it is in-bounds and lies on utf-8 boundary
    fn decode_position(&self, position: Position, line_index: &LineIndex) -> Option<usize> {
        let line_col = match self.encoding {
            Encoding::Utf8 => Some(LineCol {
                line: position.line,
                col: position.character,
            }),
            Encoding::Utf16 => line_index.to_utf8(
                WideEncoding::Utf16,
                WideLineCol {
                    line: position.line,
                    col: position.character,
                },
            ),
            Encoding::Utf32 => line_index.to_utf8(
                WideEncoding::Utf32,
                WideLineCol {
                    line: position.line,
                    col: position.character,
                },
            ),
        }?;
        line_index.offset(line_col).map(usize::from)
    }

    pub(crate) fn negotiated_encoding(&self) -> PositionEncodingKind {
        self.encoding.into()
    }

    pub(crate) fn pull_diagnostics_support(&self) -> bool {
        (|| -> _ {
            self.init_params
                .capabilities
                .text_document
                .as_ref()?
                .diagnostic
                .as_ref()
        })()
        .is_some()
    }

    pub(crate) fn related_information_support(&self) -> bool {
        (|| -> _ {
            self.init_params
                .capabilities
                .text_document
                .as_ref()?
                .publish_diagnostics
                .as_ref()?
                .related_information
        })()
        .unwrap_or_default()
    }

    pub(crate) fn code_description_support(&self) -> bool {
        (|| -> _ {
            self.init_params
                .capabilities
                .text_document
                .as_ref()?
                .publish_diagnostics
                .as_ref()?
                .code_description_support
        })()
        .unwrap_or_default()
    }

    pub(crate) fn version_support(&self) -> bool {
        (|| -> _ {
            self.init_params
                .capabilities
                .text_document
                .as_ref()?
                .publish_diagnostics
                .as_ref()?
                .version_support
        })()
        .unwrap_or_default()
    }
}

// internal representation to simplify logic:
// use an enum rather than PositionEncodingKind's string
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
    use lsp_types::GeneralClientCapabilities;

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

    fn pos(line: u32, character: u32) -> Option<Position> {
        Some(Position { line, character })
    }

    #[test]
    fn defaults() {
        let client = Client::new(InitializeParams::default());
        assert_eq!(PositionEncodingKind::UTF16, client.negotiated_encoding());
        assert!(!client.pull_diagnostics_support());
        assert!(!client.related_information_support());
        assert!(!client.code_description_support());
        assert!(!client.version_support());
    }

    #[test]
    fn utf8_encode() {
        let utf8 = with_encoding(PositionEncodingKind::UTF8);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(pos(0, 0), utf8.encode_point(&point(0, 0), &index));
        // 1-byter
        assert_eq!(pos(0, 1), utf8.encode_point(&point(0, 1), &index));
        // 2-byter
        assert_eq!(pos(0, 3), utf8.encode_point(&point(0, 3), &index));
        // 3-byter
        assert_eq!(pos(0, 6), utf8.encode_point(&point(0, 6), &index));
        // 4-byter
        assert_eq!(pos(0, 10), utf8.encode_point(&point(0, 10), &index));
        // newline
        assert_eq!(pos(1, 0), utf8.encode_point(&point(1, 0), &index));
        // 1-byter
        assert_eq!(pos(1, 1), utf8.encode_point(&point(1, 1), &index));
    }

    #[test]
    fn utf8_decode() {
        let utf8 = with_encoding(PositionEncodingKind::UTF8);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(Some(0), utf8.decode_position(Position::new(0, 0), &index));
        // 1-byter
        assert_eq!(Some(1), utf8.decode_position(Position::new(0, 1), &index));
        // 2-byter
        assert_eq!(Some(3), utf8.decode_position(Position::new(0, 3), &index));
        // 3-byter
        assert_eq!(Some(6), utf8.decode_position(Position::new(0, 6), &index));
        // 4-byter
        assert_eq!(Some(10), utf8.decode_position(Position::new(0, 10), &index));
        // newline
        assert_eq!(Some(11), utf8.decode_position(Position::new(1, 0), &index));
        // 1-byter
        assert_eq!(Some(12), utf8.decode_position(Position::new(1, 1), &index));
    }

    #[test]
    fn utf16_encode() {
        let utf16 = with_encoding(PositionEncodingKind::UTF16);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(pos(0, 0), utf16.encode_point(&point(0, 0), &index));
        // 1-byter
        assert_eq!(pos(0, 1), utf16.encode_point(&point(0, 1), &index));
        // 2-byter
        assert_eq!(pos(0, 2), utf16.encode_point(&point(0, 3), &index));
        // 3-byter
        assert_eq!(pos(0, 3), utf16.encode_point(&point(0, 6), &index));
        // 4-byter
        assert_eq!(pos(0, 5), utf16.encode_point(&point(0, 10), &index));
        // newline
        assert_eq!(pos(1, 0), utf16.encode_point(&point(1, 0), &index));
        // 1-byter
        assert_eq!(pos(1, 1), utf16.encode_point(&point(1, 1), &index));
    }

    #[test]
    fn utf16_decode() {
        let utf16 = with_encoding(PositionEncodingKind::UTF16);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(Some(0), utf16.decode_position(Position::new(0, 0), &index));
        // 1-byter
        assert_eq!(Some(1), utf16.decode_position(Position::new(0, 1), &index));
        // 2-byter
        assert_eq!(Some(3), utf16.decode_position(Position::new(0, 2), &index));
        // 3-byter
        assert_eq!(Some(6), utf16.decode_position(Position::new(0, 3), &index));
        // 4-byter
        assert_eq!(Some(10), utf16.decode_position(Position::new(0, 5), &index));
        // newline
        assert_eq!(Some(11), utf16.decode_position(Position::new(1, 0), &index));
        // 1-byter
        assert_eq!(Some(12), utf16.decode_position(Position::new(1, 1), &index));
    }

    #[test]
    fn utf32_encode() {
        let utf32 = with_encoding(PositionEncodingKind::UTF32);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(pos(0, 0), utf32.encode_point(&point(0, 0), &index));
        // 1-byter
        assert_eq!(pos(0, 1), utf32.encode_point(&point(0, 1), &index));
        // 2-byter
        assert_eq!(pos(0, 2), utf32.encode_point(&point(0, 3), &index));
        // 3-byter
        assert_eq!(pos(0, 3), utf32.encode_point(&point(0, 6), &index));
        // 4-byter
        assert_eq!(pos(0, 4), utf32.encode_point(&point(0, 10), &index));
        // newline
        assert_eq!(pos(1, 0), utf32.encode_point(&point(1, 0), &index));
        // 1-byter
        assert_eq!(pos(1, 1), utf32.encode_point(&point(1, 1), &index));
    }

    #[test]
    fn utf32_decode() {
        let utf32 = with_encoding(PositionEncodingKind::UTF32);
        let index = LineIndex::new("1\u{6f3}\u{2165}\u{1f130}\n2");
        assert_eq!(Some(0), utf32.decode_position(Position::new(0, 0), &index));
        // 1-byter
        assert_eq!(Some(1), utf32.decode_position(Position::new(0, 1), &index));
        // 2-byter
        assert_eq!(Some(3), utf32.decode_position(Position::new(0, 2), &index));
        // 3-byter
        assert_eq!(Some(6), utf32.decode_position(Position::new(0, 3), &index));
        // 4-byter
        assert_eq!(Some(10), utf32.decode_position(Position::new(0, 4), &index));
        // newline
        assert_eq!(Some(11), utf32.decode_position(Position::new(1, 0), &index));
        // 1-byter
        assert_eq!(Some(12), utf32.decode_position(Position::new(1, 1), &index));
    }
}
