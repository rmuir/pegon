; Lint queries written in tree-sitter query language.
; See <https://tree-sitter.github.io/tree-sitter/using-parsers/queries/1-syntax.html>
;
; Maximize relevance of the diagnostic content: use a scalpel.
;
;  * Narrow the @error to the most precise node possible.
;  * Take care with @visible: avoid any captures that span multiple lines.
;  * Extra care with @context: avoid insane ANSI line drawings.
;  * Split a rule into multiple patterns, if it makes it easier on the user.
;
; When working on these files, you'll want the following tools:
;
;  * <https://github.com/ribru17/ts_query_ls>
;  * <https://github.com/tree-sitter/tree-sitter-java>
;
; To get started quickly, run the following from the repository root:
;
; ```sh
; cargo install ts_query_ls
; curl -fLO https://github.com/tree-sitter/tree-sitter-java/releases/download/v0.23.5/tree-sitter-java.wasm
; ```
;
; The `.tsqueryrc.json` looks for tree-sitter-java parser in the following:
;
;  * current directory (e.g. WASM file from curl example above)
;  * /usr/lib/treesitter (e.g. `yay -S tree-sitter-java` from Arch AUR)
;  * nvim-treesitter installation (in case you use neovim and have it already)
;
; TS parsing error
((ERROR) @error
  (#set! name "parse-error")
  (#set! title "Parse Error")
  (#set! label "Syntax problem")
  (#set! note "Correct the invalid Java syntax")
  (#set! severity "hint"))

; TS parsing error
((MISSING) @error
  (#set! name "parse-error")
  (#set! title "Parse Error")
  (#set! label "Missing {node.kind}")
  (#set! note "Correct the invalid Java syntax")
  (#set! severity "hint"))

; Whitespace other than ASCII horizontal space inside a literal.
; @see https://google.github.io/styleguide/javaguide.html#s2.3.1-whitespace-characters
([
  (character_literal)
  (string_fragment)
  (multiline_string_fragment)
] @error
  (#match? @error "[\\s&&[^\\u0020\n]]")
  (#set! name "literal-special-space")
  (#set! title "Special whitespace in literal")
  (#set! label "Literal")
  (#set! note "Escape the special whitespace: only `0x20` may appear in literals")
  (#set! severity "error"))

; Octal backspace escape instead of `\b`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\010" "\\10")
  (#set! name "octal-backspace")
  (#set! title "Octal backspace escape: `{node.text}`")
  (#set! label "Backspace")
  (#set! note "Replace `{node.text}` with special escape `\\b`")
  (#set! fix "\\b")
  (#set! severity "warning"))

; Unicode hex backspace escape instead of `\b`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0008")
  (#set! name "hex-backspace")
  (#set! title "Hexadecimal backspace escape: `{node.text}`")
  (#set! label "Backspace")
  (#set! note "Replace `{node.text}` with special escape `\\b`")
  (#set! fix "\\b")
  (#set! severity "warning"))

; Octal tab escape instead of `\t`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\011" "\\11")
  (#set! name "octal-tab")
  (#set! title "Octal tab escape: `{node.text}`")
  (#set! label "Tab")
  (#set! note "Replace `{node.text}` with special escape `\\t`")
  (#set! fix "\\t")
  (#set! severity "warning"))

; Unicode hex tab escape instead of `\t`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0009")
  (#set! name "hex-tab")
  (#set! title "Hexadecimal tab escape: `{node.text}`")
  (#set! label "Tab")
  (#set! note "Replace `{node.text}` with special escape `\\t`")
  (#set! fix "\\t")
  (#set! severity "warning"))

; Octal newline escape instead of `\n`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\012" "\\12")
  (#set! name "octal-newline")
  (#set! title "Octal newline escape: `{node.text}`")
  (#set! label "Newline")
  (#set! note "Replace `{node.text}` with special escape `\\n`")
  (#set! fix "\\n")
  (#set! severity "warning"))

; Unicode hex newline escape instead of `\n`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000a" "\\u000A")
  (#set! name "hex-newline")
  (#set! title "Hexadecimal newline escape: `{node.text}`")
  (#set! label "Newline")
  (#set! note "Replace `{node.text}` with special escape `\\n`")
  (#set! fix "\\n")
  (#set! severity "warning"))

; Octal form feed escape instead of `\f`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\014" "\\14")
  (#set! name "octal-formfeed")
  (#set! title "Octal form feed escape: `{node.text}`")
  (#set! label "Form feed")
  (#set! note "Replace `{node.text}` with special escape `\\f`")
  (#set! fix "\\f")
  (#set! severity "warning"))

; Unicode hex form feed escape instead of `\f`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000c" "\\u000C")
  (#set! name "hex-formfeed")
  (#set! title "Hexadecimal form feed escape: `{node.text}`")
  (#set! label "Form feed")
  (#set! note "Replace `{node.text}` with special escape `\\f`")
  (#set! fix "\\f")
  (#set! severity "warning"))

; Octal carriage return escape instead of `\r`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\015" "\\15")
  (#set! name "octal-return")
  (#set! title "Octal carriage return escape: `{node.text}`")
  (#set! label "Carriage return")
  (#set! note "Replace `{node.text}` with special escape `\\r`")
  (#set! fix "\\r")
  (#set! severity "warning"))

; Unicode hex carriage return escape instead of `\r`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000d" "\\u000D")
  (#set! name "hex-return")
  (#set! title "Hexadecimal carriage return escape: `{node.text}`")
  (#set! label "Carriage return")
  (#set! note "Replace `{node.text}` with special escape `\\r`")
  (#set! fix "\\r")
  (#set! severity "warning"))

; Octal double-quote escape instead of `\"`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\042" "\\42")
  (#set! name "octal-double-quote")
  (#set! title "Octal double quote escape: `{node.text}`")
  (#set! label "Double quote")
  (#set! note "Replace `{node.text}` with special escape `\\\"`")
  (#set! fix "\\\"")
  (#set! severity "warning"))

; Unicode hex double-quote escape instead of `\"`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0022")
  (#set! name "hex-double-quote")
  (#set! title "Hexadecimal double quote escape: `{node.text}`")
  (#set! label "Double quote")
  (#set! note "Replace `{node.text}` with special escape `\\\"`")
  (#set! fix "\\\"")
  (#set! severity "warning"))

; Octal single-quote escape instead of `\'`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\047" "\\47")
  (#set! name "octal-single-quote")
  (#set! title "Octal single quote escape: `{node.text}`")
  (#set! label "Single quote")
  (#set! note "Replace `{node.text}` with special escape `\\'`")
  (#set! fix "\\'")
  (#set! severity "warning"))

; Unicode hex single-quote escape instead of `\'`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0027")
  (#set! name "hex-single-quote")
  (#set! title "Hexadecimal single quote escape: `{node.text}`")
  (#set! label "Single quote")
  (#set! note "Replace `{node.text}` with special escape `\\'`")
  (#set! fix "\\'")
  (#set! severity "warning"))

; Octal backslash escape instead of `\\`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\134")
  (#set! name "octal-backslash")
  (#set! title "Octal backslash escape: `{node.text}`")
  (#set! label "Backslash")
  (#set! note "Replace `{node.text}` with special escape `\\\\`")
  (#set! fix "\\\\")
  (#set! severity "warning"))

; Unicode hex backslash escape instead of `\\`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u005c" "\\u005C")
  (#set! name "hex-backslash")
  (#set! title "Hexadecimal backslash escape: `{node.text}`")
  (#set! label "Backslash")
  (#set! note "Replace `{node.text}` with special escape `\\\\`")
  (#set! fix "\\\\")
  (#set! severity "warning"))

; Line-wrapped package declaration
; @see https://google.github.io/styleguide/javaguide.html#s3.2-package-declaration
((package_declaration
  .
  [
    (identifier)
    (scoped_identifier)
  ]) @error
  (#match? @error "\n")
  (#set! name "wrapped-package")
  (#set! title "Line-wrapped package declaration: `{node.text}`")
  (#set! label "Wrapped")
  (#set! note "Remove newlines from the package statement")
  (#set! severity "error"))

; Wildcard imports
; @see https://google.github.io/styleguide/javaguide.html#s3.3.1-wildcard-imports
((import_declaration
  (asterisk) @error)
  (#set! name "wildcard-import")
  (#set! title "Wildcard import")
  (#set! label "Wildcard")
  (#set! note "Replace the wildcard import with standard import(s)")
  (#set! severity "error"))

; Line-wrapped imports
; @see https://google.github.io/styleguide/javaguide.html#s3.3.2-import-line-wrapping
((import_declaration) @error
  (#match? @error "\n")
  (#set! name "wrapped-import")
  (#set! title "Line-wrapped import")
  (#set! label "Wrapped")
  (#set! note "Remove newlines from the import statement")
  (#set! severity "error"))

; Multiple top-level classes in the same file
; @see https://google.github.io/styleguide/javaguide.html#s3.4.1-one-top-level-class
(program
  (class_declaration
    name: (identifier) @context)+
  (class_declaration
    name: (identifier) @error)
  (#set! name "multiple-classes")
  (#set! title "Multiple top-level classes: `{node.text}`")
  (#set! label "Additional class")
  (#set! context.label "Previous class")
  (#set! note "Move `{node.text}` to separate `{node.text}.java` file")
  (#set! severity "error"))

; Integer literal with lowercase 'l'
; @see https://google.github.io/styleguide/javaguide.html#s4.8.8-numeric-literals
((decimal_integer_literal) @error
  (#match? @error "l$")
  (#set! name "lowercase-long-literal")
  (#set! title "Lowercase long integer literal: `{node.text}`")
  (#set! label "Lowercase")
  (#set! note "Replace with uppercase L suffix to improve legibility")
  (#set! severity "error"))

; Dollar sign in identifier
; @see https://google.github.io/styleguide/javaguide.html#s5.1-identifier-names
((identifier) @error
  (#match? @error "[$]")
  (#set! name "dollar-in-identifier")
  (#set! title "Dollar sign in identifier: `{node.text}`")
  (#set! label "Identifier")
  (#set! note "Rename `{node.text}` using only ASCII letters, digits, and underscores")
  (#set! severity "error"))

; Identifier containing unicode character
; @see https://google.github.io/styleguide/javaguide.html#s5.1-identifier-names
((identifier) @error
  (#match? @error "[^a-zA-Z0-9_$]")
  (#set! name "unicode-identifier")
  (#set! title "Unicode in identifier: `{node.text}`")
  (#set! label "Identifier")
  (#set! note "Rename `{node.text}` using only ASCII letters, digits, and underscores")
  (#set! severity "error"))

; Package names should be lowercase and digits only
; @see https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
((package_declaration
  (identifier) @error)
  (#match? @error "[^a-z0-9]")
  (#set! name "invalid-package-name")
  (#set! title "Invalid package name: `{node.text}`")
  (#set! label "Package")
  (#set! note "Rename `{node.text}` using only lowercase and digits")
  (#set! severity "error"))

; Module names should be lowercase and digits only
; @see https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
((module_declaration
  (identifier) @error)
  (#match? @error "[^a-z0-9]")
  (#set! name "invalid-module-name")
  (#set! title "Invalid module name: `{node.text}`")
  (#set! label "Module")
  (#set! note "Rename `{node.text}` using only lowercase and digits")
  (#set! severity "error"))

; Class names should be UpperCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.2-class-names
((class_declaration
  name: (identifier) @error)
  (#match? @error "^[a-z]")
  (#set! name "lowercase-class")
  (#set! title "Lowercase class name: `{node.text}`")
  (#set! label "Lowercase")
  (#set! note "Rename `{node.text}` using UpperCamelCase")
  (#set! severity "error"))

; Parameters should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
((formal_parameter
  name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! name "uppercase-param")
  (#set! title "Uppercase parameter name: `{node.text}`")
  (#set! label "Uppercase")
  (#set! note "Rename `{node.text}` using lowerCamelCase")
  (#set! severity "error")) @visible

; Varargs parameter should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
(spread_parameter
  (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! name "uppercase-vararg")
  (#set! title "Uppercase vararg name: `{node.text}`")
  (#set! label "Uppercase")
  (#set! note "Rename `{node.text}` using lowerCamelCase")
  (#set! severity "error")) @visible

; Local variables should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.7-local-variable-names
(local_variable_declaration
  .
  type: (_)
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! name "uppercase-local")
  (#set! title "Uppercase local variable name: `{node.text}`")
  (#set! label "Uppercase")
  (#set! note "Rename `{node.text}` using lowerCamelCase")
  (#set! severity "error"))

; Local variables should be lowerCamelCase (final variant)
; @see https://google.github.io/styleguide/javaguide.html#s5.2.7-local-variable-names
(local_variable_declaration
  .
  (modifiers) @context
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! name "uppercase-final-local")
  (#set! title "Uppercase local variable name: `{node.text}`")
  (#set! label "Uppercase")
  (#set! context.label "Not a `static final` constant")
  (#set! note "Rename `{node.text}` using lowerCamelCase")
  (#set! severity "error"))

; Type variables should be UpperCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.8-type-variable-names
((type_parameter
  (type_identifier) @error)
  (#match? @error "^[a-z]")
  (#set! name "lowercase-type")
  (#set! title "Lowercase type parameter name: `{node.text}`")
  (#set! label "Lowercase")
  (#set! note "Rename `{node.text}` using UpperCamelCase")
  (#set! severity "error")) @visible

; Caught exceptions: not ignored
; @see https://google.github.io/styleguide/javaguide.html#s6.2-caught-exceptions
(catch_clause
  (catch_formal_parameter
    (catch_type)
    name: (identifier) @error)
  body: (block) @_block
  ; unnamed variable
  (#not-eq? @error "_")
  ; no real content at all
  (#not-match? @_block "[a-zA-Z0-9_]")
  (#set! name "swallowed-exception")
  (#set! title "Unhandled caught exception: `{node.text}`")
  (#set! label "Exception ignored")
  (#set! note "Handle `{node.text}`, add a comment, or indicate via unnamed variable `_`")
  (#set! severity "error")) @visible ; body is small (empty)

; Finalizers: not used
; @see https://google.github.io/styleguide/javaguide.html#s6.4-finalizers
((method_declaration
  type: (void_type) @visible
  ; body could be large
  name: (identifier) @error
  parameters: (formal_parameters) @_params)
  (#eq? @error "finalize")
  ; only parentheses
  (#match? @_params "^[\\s]*[(][\\s]*[)][\\s]*$")
  (#set! name "finalizer-used")
  (#set! title "Finalizer used: `{node.text}`")
  (#set! label "Overrides Object.finalize()")
  (#set! note "Migrate to other resource management such as try-with-resources or cleaners")
  (#set! severity "error"))
