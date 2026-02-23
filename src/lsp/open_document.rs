use tree_sitter::Tree;

pub struct OpenDocument {
    pub(crate) text: String,
    pub(crate) version: i32,
    pub(crate) tree: Tree,
}
