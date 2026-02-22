use std::ops::Range;

use line_index::{LineCol, LineIndex, TextSize, WideEncoding, WideLineCol};
use lsp_server::Connection;
use lsp_types::{ClientCapabilities, InitializeParams, Position, PositionEncodingKind};

pub struct Client {
    pub(crate) connection: Connection,
    init_params: InitializeParams,
    encoding: Encoding,
}

impl Client {
    pub(crate) fn new(connection: Connection, init_params: InitializeParams) -> Self {
        let encoding = Encoding::preferred(&init_params.capabilities);
        Self {
            connection,
            init_params,
            encoding,
        }
    }

    pub(crate) fn encode_position(
        &self,
        offset: usize,
        line_index: &LineIndex,
    ) -> Option<Position> {
        #[allow(clippy::cast_possible_truncation)]
        let position = line_index.try_line_col(TextSize::from(offset as u32))?;
        match self.encoding {
            Encoding::Utf8 => Some(Position::new(position.line, position.col)),
            Encoding::Utf16 => {
                let wide = line_index.to_wide(WideEncoding::Utf16, position)?;
                Some(Position::new(wide.line, wide.col))
            }
            Encoding::Utf32 => {
                let wide = line_index.to_wide(WideEncoding::Utf32, position)?;
                Some(Position::new(wide.line, wide.col))
            }
        }
    }

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

    pub(crate) fn decode_position(
        &self,
        position: Position,
        line_index: &LineIndex,
    ) -> Option<usize> {
        match self.encoding {
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
        }
        .and_then(|line_col| line_index.offset(line_col).map(usize::from))
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
