# Query files

Queries written in tree-sitter query language.
See <https://tree-sitter.github.io/tree-sitter/using-parsers/queries/1-syntax.html>

## Developer Setup

When working on these files, you'll want the following tools:

* <https://codeberg.org/grammar-orchard/tree-sitter-java-orchard>
* <https://github.com/ribru17/ts_query_ls>

To get started quickly, run the following from the repository root:

```sh
# install grammar
git clone ssh://git@codeberg.org/grammar-orchard/tree-sitter-java-orchard.git
cd tree-sitter-java-orchard
tree-sitter generate
tree-sitter build
export TREE_SITTER_JAVA_ORCHARD_HOME=`pwd`
# install linter/formatter/LSP
cargo install ts_query_ls
```

The `.tsqueryrc.json` looks for tree-sitter-java-orchard parser in the following:

* current directory (e.g. WASM file)
* `TREE_SITTER_JAVA_ORCHARD_HOME` environment variable
