use anyhow::Result;
use line_index::{LineIndex, TextSize, WideEncoding};
use lsp_types::{ClientCapabilities, Position, PositionEncodingKind};

pub enum Encoding {
    Utf8,
    Utf16,
    Utf32,
}

impl Encoding {
    pub fn preferred(capabilities: &ClientCapabilities) -> PositionEncodingKind {
        if let Some(general) = &capabilities.general
            && let Some(encodings) = &general.position_encodings
            && let Some(preferred) = encodings.first()
        {
            preferred.clone()
        } else {
            PositionEncodingKind::UTF16
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

impl TryFrom<PositionEncodingKind> for Encoding {
    type Error = ();

    fn try_from(value: PositionEncodingKind) -> Result<Self, Self::Error> {
        match value.as_str() {
            "utf-8" => Ok(Self::Utf8),
            "utf-16" => Ok(Self::Utf16),
            "utf-32" => Ok(Self::Utf32),
            _ => Err(()),
        }
    }
}
