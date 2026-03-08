```text
A slightly fast Java linter and code formatter, written in Rust.

More sentence

Usage: pegon check [OPTIONS] [FILES]...
       pegon format [OPTIONS] [FILES]...
       pegon server [OPTIONS]

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

pegon check:
Run pegon on the given files or directories
      --fix
          Apply fixes to resolve lint violations

      --output-format <FMT>
          Diagnostic error format

          Possible values:
          - full:    Cargo-style format
          - concise: Grep-style format
          
          [env: PEGON_OUTPUT_FORMAT=]
          [default: full]

  -h, --help
          Print help (see a summary with '-h')

  [FILES]...
          List of files or directories to check
          
          Use `-` for standard input. [default: CWD]

pegon format:
Run the pegon formatter on the given files or directories
      --check
          Avoid writing any formatted files back; instead, exit with a non-zero status code if any files would be modified, and zero otherwise

  -h, --help
          Print help

  [FILES]...
          List of files or directories to format, or `-` to read from stdin

pegon server:
Run the language server
      --stdio
          Use standard I/O streams [default]

      --socket <PORT>
          Listen on loopback TCP socket

  -h, --help
          Print help

```
