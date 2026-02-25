use line_index::LineIndex;
use tree_sitter::Tree;

pub struct OpenDocument {
    pub(crate) text: String,
    pub(crate) version: i32,
    pub(crate) line_index: LineIndex,
    pub(crate) tree: Tree,
}
