use line_index::{LineIndex, TextSize, WideEncoding};
use lsp_server::Connection;
use lsp_types::{
    ClientCapabilities, InitializeParams, OneOf, Position, PositionEncodingKind, SaveOptions,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};

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

    pub(crate) fn to_position(&self, offset: usize, line_index: &LineIndex) -> Option<Position> {
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

    /// TODO
    #[allow(dead_code)]
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

    pub(crate) fn server_capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            position_encoding: Some(self.encoding.into()),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    // TODO: delta updates
                    change: Some(TextDocumentSyncKind::FULL),
                    save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                        include_text: Some(true),
                    })),
                    ..Default::default()
                },
            )),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Left(true)),
                }),
                file_operations: None,
            }),
            ..ServerCapabilities::default()
        }
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
