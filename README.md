# pegon

Fast Java language server

## Features

* Zero-hassle configuration.
* High editor performance.
* Adherence to the Language Server Protocol standard.
* CLI diagnostics for linting in CI/precommit.
* Small standalone binary with no external dependencies.

## Install

```sh
cargo install --git https://github.com/rmuir/pegon
```

## Editor Setup

### Neovim

**~/.config/nvim/lsp/pegon.lua**:

```lua
--- @type vim.lsp.Config
return {
  cmd = { 'pegon', 'server' },
  filetypes = { 'java' },
  root_markers = { '.git' },
}
```

**~/.config/nvim/init.lua**:

```lua
vim.lsp.enable({ 'pegon' })
```

## Architecture

* Parser: [tree-sitter-java-orchard](https://crates.io/crates/tree-sitter-java-orchard) for modern Java syntax support
* LSP: [lsp-server](https://crates.io/crates/lsp-server) from rust-analyzer, with [gen-lsp-types](https://crates.io/crates/gen-lsp-types)
* CLI: [clap](https://crates.io/crates/clap) with [annotate-snippets](https://crates.io/crates/annotate-snippets) for diagnostics

## LSP Support

### LSP Language features

| Method                              | Notes                                                |
|------------------------------------ | ---------------------------------------------------- |
| `textDocument/codeAction`           | quick fix, organize imports                          |
| `codeAction/resolve`                | defers logic for fast "light bulb"                   |
| `textDocument/definition`           | inlay hint interaction                               |
| `textDocument/diagnostic`           | syntax errors, style deviations, related information |
| `textDocument/documentHighlight`    | related syntax elements                              |
| `textDocument/documentSymbol`       | signature info, detailed type info, deprecations     |
| `textDocument/foldingRange`         | regions, imports, comments, javadoc handling         |
| `textDocument/hover`                | operators, JLS links                                 |
| `textDocument/inlayHint`            | closing braces, generic types                        |
| `textDocument/selectionRange`       | incremental selection of tree nodes                  |
| `textDocument/semanticTokens/full`  | highlighting                                         |
| `textDocument/semanticTokens/range` | highlighting                                         |

### LSP lifecycle methods

| Method                              | Notes                                                |
|------------------------------------ | ---------------------------------------------------- |
| `initialize`                        | negotiates client encoding for best performance      |
| `shutdown`                          | will always actually shut down, not leak processes   |
| `client/registerCapability`         | dynamic registration scoped to `Java` files          |
| `textDocument/didOpen`              | graceful errors on non-`Java` files                  |
| `textDocument/didChange`            | incremental updates, incremental parsing             |
| `textDocument/didClose`             | clears any pushed diagnostics per spec               |
| `window/logMessage`                 | notification errors reported to client               |
| `$/cancelRequest`                   | queue + threadpool for requests with cancellation    |

## Background

Historically, Java developers have used sophisticated IDEs to cope with the verbosity of the language.
It is possible to harness this in your text editor, thanks to [Eclipse JDTLS](https://github.com/eclipse-jdtls/eclipse.jdt.ls).
JDTLS is very powerful, but there are some challenges:

* Notoriously difficult for users to configure and get working
* Requires custom LSP client code such as [nvim-jdtls](https://github.com/mfussenegger/nvim-jdtls) for full functionality
* High consumption of CPU, memory and disk resources.
* Slow startup: must compile all of the code.
* Slow response time/lag on large Java source files.
* Many moving parts: LSP configuration, additional editor plugins, JVM, Gradle/Maven
* Formats code to a standard nobody wants.

Other Java tooling for formatting and linting have similar problems, as they must compile the code.
These tools slowly hook into the compiler (usually invasively!), and sometimes require the user to
wrestle with maven, gradle, `JAVA_HOME`, `CLASSPATH`, etc.

The intent of `pegon` is not to rewrite this same situation in Rust, but instead to provide an alternative.

## FAQ

**Q. This code looks like AI slop!**

A. No LLM was used: if the code looks bad, it is because I wrote this application to learn Rust.

**Q. What's with the Google Style?**

A. Google Style supports high-performance Java development, and most developers roughly follow it:

* Single top-level class with matching `.java` name supports search, fuzzy-finder
* Variable naming conventions support easier code review and understanding
* Import restrictions (e.g. no wildcards) support efficient analysis

**Q. Where's completion / go-to-definition? I can't live without it!**

A. These features are planned, continue to use `ctags` for now.

**Q. Where's workspace symbol search? I can't live without it!**

A. This feature is inherently slow, I recommend a good fuzzy finder or `ctags` instead.

**Q. What about running tests from my editor? This is essential for TDD!**

A. I strongly recommend [vim-test](https://github.com/vim-test/vim-test) for this.

If your java build uses gradle, the daemon allows for very fast iteration with no config hassle.
Black magic.

**Q. Where's XYZ fancy refactoring? I can't live without it!**

A. Currently the code actions are minimal. Check out some alternatives for now:

* [vim-doge](https://github.com/kkoomen/vim-doge) and [neogen](https://github.com/danymat/neogen) can generate Javadoc
* [ast-grep](https://github.com/ast-grep/ast-grep) can do structural search/rewrite

**Q. What's with the name?**

A. This program is named after the [Javanese writing system](https://en.wikipedia.org/wiki/Pegon_script#Etymology). (**ڤَيڮَون**)

## Thanks

Special thanks to the open-source maintainers of the dependencies used.
