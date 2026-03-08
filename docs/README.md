# Usage

This document contains the help content for the `pegon` command-line program.

**Command Overview:**

* [`pegon`↴](#pegon)
* [`pegon check`↴](#pegon-check)
* [`pegon format`↴](#pegon-format)
* [`pegon server`↴](#pegon-server)

## `pegon`

A slightly fast Java linter and code formatter, written in Rust.

More sentence

**Usage:** `pegon <COMMAND>`

###### **Subcommands:**

* `check` — Run pegon on the given files or directories
* `format` — Run the pegon formatter on the given files or directories
* `server` — Run the language server



## `pegon check`

Run pegon on the given files or directories.

More information

**Usage:** `pegon check [OPTIONS] [FILES]...`

###### **Arguments:**

* `<FILES>` — List of files or directories to check, or `-` to read from stdin

###### **Options:**

* `--fix` — Apply fixes to resolve lint violations
* `--output-format <OUTPUT_FORMAT>` — Diagnostic output format

  Default value: `full`

  Possible values: `full`, `concise`




## `pegon format`

Run the pegon formatter on the given files or directories

**Usage:** `pegon format [OPTIONS] [FILES]...`

###### **Arguments:**

* `<FILES>` — List of files or directories to format, or `-` to read from stdin

###### **Options:**

* `--check` — Avoid writing any formatted files back; instead, exit with a non-zero status code if any files would be modified, and zero otherwise



## `pegon server`

Run the language server

**Usage:** `pegon server [OPTIONS]`

###### **Options:**

* `--stdio` — Use standard I/O streams (default)
* `--socket <PORT>` — Listen on loopback TCP socket



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
