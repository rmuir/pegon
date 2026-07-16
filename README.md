# pegon

Fast Java language server

## Features

* Zero-hassle configuration.
* High editor performance.
* Adherence to the Language Server Protocol standard.
* CLI diagnostics for linting in CI/precommit.
* Small standalone binary with no external dependencies.

## Install

### Binaries

Pegon is available on [pypi](https://pypi.org/project/pegon/).

Use `uvx` to run a one-off lint check on your Java code:

```sh
uvx pegon check
```

Or install with `uv` or `pip`:

```sh
# with uv
uv tool install pegon@latest
# with pip
pip install pegon
```

### Install from Source

```sh
cargo install pegon
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

## FAQ

**Q. This code looks like AI slop!**

A. No LLM was used: if the code looks bad, it is because I wrote this application to learn Rust.

**Q. What's with the Google Style?**

A. Google Style supports high-performance Java development, and most developers roughly follow it:

* Single top-level class with matching `.java` name supports search, fuzzy-finder
* Variable naming conventions support easier code review and understanding
* Import restrictions (e.g. no wildcards) support efficient analysis

**Q. Where's completion / go-to-definition? I can't live without it!**

A. These features are coming, continue to use `ctags` for now.

**Q. Where's workspace symbol search? I can't live without it!**

A. This feature is coming, I recommend a good fuzzy finder or `ctags` for now.

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
