use line_index::{LineIndex, TextSize, WideEncoding};
use lsp_types::{ClientCapabilities, Position, PositionEncodingKind};

#[derive(Clone)]
pub enum Encoding {
    Utf8,
    Utf16,
    Utf32,
}

impl Encoding {
    pub fn preferred(capabilities: &ClientCapabilities) -> Self {
        if let Some(general) = &capabilities.general
            && let Some(encodings) = &general.position_encodings
            && let Some(preferred) = encodings.first()
        {
            preferred.into()
        } else {
            Self::Utf16
        }
    }

    pub fn to_position(&self, offset: usize, line_index: &LineIndex) -> Option<Position> {
        let position = line_index.try_line_col(TextSize::from(offset as u32))?;
        match self {
            Self::Utf8 => Some(Position::new(position.line, position.col)),
            Self::Utf16 => {
                let wide = line_index.to_wide(WideEncoding::Utf16, position)?;
                Some(Position::new(wide.line, wide.col))
            }
            Self::Utf32 => {
                let wide = line_index.to_wide(WideEncoding::Utf32, position)?;
                Some(Position::new(wide.line, wide.col))
            }
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
