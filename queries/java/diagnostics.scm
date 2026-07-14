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
  (#set! diagnostic.name "syntax-error")
  (#set! diagnostic.title "Syntax error")
  (#set! diagnostic.help "Suppressed any further diagnostics for this file")
  (#set! diagnostic.severity "error"))

; TS parsing error
((MISSING) @error
  (#set! diagnostic.name "syntax-missing")
  (#set! diagnostic.title "Missing `{node.kind}`")
  (#set! diagnostic.help "Suppressed any further diagnostics for this file")
  (#set! diagnostic.severity "error"))

; Whitespace other than ASCII horizontal space inside a literal.
; @see https://google.github.io/styleguide/javaguide.html#s2.3.1-whitespace-characters
([
  (character_literal)
  (string_fragment)
  (multiline_string_fragment)
] @error
  (#match? @error "[\\s&&[^\\u0020\r\n]]")
  (#set! diagnostic.name "literal-special-space")
  (#set! diagnostic.title "Literal contains unescaped special whitespace")
  (#set! diagnostic.help "Escape the special whitespace: only `0x20` may appear in literals")
  (#set! diagnostic.severity "warn")) ; TODO: implement autofix

; Octal backspace escape instead of `\b`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\010" "\\10")
  (#set! diagnostic.name "octal-backspace")
  (#set! diagnostic.title "Octal backspace escape: `{node.text}`")
  (#set! diagnostic.label "Backspace")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\b`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\b")
  (#set! diagnostic.severity "hint"))

; Unicode hex backspace escape instead of `\b`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0008")
  (#set! diagnostic.name "hex-backspace")
  (#set! diagnostic.title "Hexadecimal backspace escape: `{node.text}`")
  (#set! diagnostic.label "Backspace")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\b`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\b")
  (#set! diagnostic.severity "hint"))

; Octal tab escape instead of `\t`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\011" "\\11")
  (#set! diagnostic.name "octal-tab")
  (#set! diagnostic.title "Octal tab escape: `{node.text}`")
  (#set! diagnostic.label "Tab")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\t`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\t")
  (#set! diagnostic.severity "hint"))

; Unicode hex tab escape instead of `\t`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0009")
  (#set! diagnostic.name "hex-tab")
  (#set! diagnostic.title "Hexadecimal tab escape: `{node.text}`")
  (#set! diagnostic.label "Tab")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\t`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\t")
  (#set! diagnostic.severity "hint"))

; Octal newline escape instead of `\n`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\012" "\\12")
  (#set! diagnostic.name "octal-newline")
  (#set! diagnostic.title "Octal newline escape: `{node.text}`")
  (#set! diagnostic.label "Newline")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\n`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\n")
  (#set! diagnostic.severity "hint"))

; Unicode hex newline escape instead of `\n`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000a" "\\u000A")
  (#set! diagnostic.name "hex-newline")
  (#set! diagnostic.title "Hexadecimal newline escape: `{node.text}`")
  (#set! diagnostic.label "Newline")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\n`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\n")
  (#set! diagnostic.severity "hint"))

; Octal form feed escape instead of `\f`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\014" "\\14")
  (#set! diagnostic.name "octal-formfeed")
  (#set! diagnostic.title "Octal form feed escape: `{node.text}`")
  (#set! diagnostic.label "Form feed")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\f`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\f")
  (#set! diagnostic.severity "hint"))

; Unicode hex form feed escape instead of `\f`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000c" "\\u000C")
  (#set! diagnostic.name "hex-formfeed")
  (#set! diagnostic.title "Hexadecimal form feed escape: `{node.text}`")
  (#set! diagnostic.label "Form feed")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\f`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\f")
  (#set! diagnostic.severity "hint"))

; Octal carriage return escape instead of `\r`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\015" "\\15")
  (#set! diagnostic.name "octal-return")
  (#set! diagnostic.title "Octal carriage return escape: `{node.text}`")
  (#set! diagnostic.label "Carriage return")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\r`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\r")
  (#set! diagnostic.severity "hint"))

; Unicode hex carriage return escape instead of `\r`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000d" "\\u000D")
  (#set! diagnostic.name "hex-return")
  (#set! diagnostic.title "Hexadecimal carriage return escape: `{node.text}`")
  (#set! diagnostic.label "Carriage return")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\r`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\r")
  (#set! diagnostic.severity "hint"))

; Octal double-quote escape instead of `\"`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\042" "\\42")
  (#set! diagnostic.name "octal-double-quote")
  (#set! diagnostic.title "Octal double quote escape: `{node.text}`")
  (#set! diagnostic.label "Double quote")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\\"`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\\"")
  (#set! diagnostic.severity "hint"))

; Unicode hex double-quote escape instead of `\"`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0022")
  (#set! diagnostic.name "hex-double-quote")
  (#set! diagnostic.title "Hexadecimal double quote escape: `{node.text}`")
  (#set! diagnostic.label "Double quote")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\\"`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\\"")
  (#set! diagnostic.severity "hint"))

; Octal single-quote escape instead of `\'`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\047" "\\47")
  (#set! diagnostic.name "octal-single-quote")
  (#set! diagnostic.title "Octal single quote escape: `{node.text}`")
  (#set! diagnostic.label "Single quote")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\'`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\'")
  (#set! diagnostic.severity "hint"))

; Unicode hex single-quote escape instead of `\'`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0027")
  (#set! diagnostic.name "hex-single-quote")
  (#set! diagnostic.title "Hexadecimal single quote escape: `{node.text}`")
  (#set! diagnostic.label "Single quote")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\'`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\'")
  (#set! diagnostic.severity "hint"))

; Octal backslash escape instead of `\\`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\134")
  (#set! diagnostic.name "octal-backslash")
  (#set! diagnostic.title "Octal backslash escape: `{node.text}`")
  (#set! diagnostic.label "Backslash")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\\\`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\\\")
  (#set! diagnostic.severity "hint"))

; Unicode hex backslash escape instead of `\\`
; @see https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u005c" "\\u005C")
  (#set! diagnostic.name "hex-backslash")
  (#set! diagnostic.title "Hexadecimal backslash escape: `{node.text}`")
  (#set! diagnostic.label "Backslash")
  (#set! diagnostic.help "Replace `{node.text}` with special escape `\\\\`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "\\\\")
  (#set! diagnostic.severity "hint"))

; Line-wrapped package declaration
; @see https://google.github.io/styleguide/javaguide.html#s3.2-package-declaration
((package_declaration
  . ; don't check ones with annotations for now
  "package") @error
  (#match? @error "\n")
  (#set! diagnostic.name "wrapped-package")
  (#set! diagnostic.title "Line-wrapped package declaration: `{node.text}`")
  (#set! diagnostic.help "Remove newlines from the package statement")
  (#set! diagnostic.severity "info"))

; Wildcard imports
; @see https://google.github.io/styleguide/javaguide.html#s3.3.1-wildcard-imports
(import_declaration
  (asterisk) @error
  (#set! diagnostic.name "wildcard-import")
  (#set! diagnostic.title "Wildcard import")
  (#set! diagnostic.help "Replace the wildcard import with standard import(s)")
  (#set! diagnostic.severity "warn"))

; Line-wrapped imports
; @see https://google.github.io/styleguide/javaguide.html#s3.3.2-import-line-wrapping
((import_declaration) @error
  (#match? @error "\n")
  (#set! diagnostic.name "wrapped-import")
  (#set! diagnostic.title "Line-wrapped import")
  (#set! diagnostic.help "Remove newlines from the import statement")
  (#set! diagnostic.severity "info"))

; Unsorted static imports
; @see https://google.github.io/styleguide/javaguide.html#s3.3.3-import-ordering-and-spacing
(program
  (import_declaration
    "static"
    (scoped_identifier) @error)
  .
  (import_declaration
    "static"
    (scoped_identifier) @context)
  (#lt? @context @error)
  (#set! diagnostic.name "unsorted-static-import")
  (#set! diagnostic.title "Static import out of order: `{node.text}`")
  (#set! diagnostic.help "Static imports should be in alphabetical order")
  (#set! diagnostic.label "sorts after")
  (#set! diagnostic.context.label "sorts before")
  (#set! diagnostic.severity "info"))

; Unsorted static imports
; @see https://google.github.io/styleguide/javaguide.html#s3.3.3-import-ordering-and-spacing
(program
  (import_declaration
    "static"
    (scoped_identifier) @error)
  .
  [
    (block_comment)
    (line_comment)
  ]+
  .
  (import_declaration
    "static"
    (scoped_identifier) @context)
  (#lt? @context @error)
  (#set! diagnostic.name "unsorted-static-import")
  (#set! diagnostic.title "Static import out of order: `{node.text}`")
  (#set! diagnostic.help "Static imports should be in alphabetical order")
  (#set! diagnostic.label "sorts after")
  (#set! diagnostic.context.label "sorts before")
  (#set! diagnostic.severity "info"))

; Unsorted imports
; @see https://google.github.io/styleguide/javaguide.html#s3.3.3-import-ordering-and-spacing
(program
  (import_declaration
    (scoped_identifier) @error) @_node1
  .
  (import_declaration
    (scoped_identifier) @context) @_node2
  (#not-match? @_node1 "^import\\s+static")
  (#not-match? @_node2 "^import\\s+static")
  (#lt? @context @error)
  (#set! diagnostic.name "unsorted-import")
  (#set! diagnostic.title "Import out of order: `{node.text}`")
  (#set! diagnostic.help "Imports should be in alphabetical order")
  (#set! diagnostic.label "sorts after")
  (#set! diagnostic.context.label "sorts before")
  (#set! diagnostic.severity "info"))

; Unsorted imports
; @see https://google.github.io/styleguide/javaguide.html#s3.3.3-import-ordering-and-spacing
(program
  (import_declaration
    (scoped_identifier) @error) @_node1
  .
  [
    (block_comment)
    (line_comment)
  ]+
  .
  (import_declaration
    (scoped_identifier) @context) @_node2
  (#not-match? @_node1 "^import\\s+static")
  (#not-match? @_node2 "^import\\s+static")
  (#lt? @context @error)
  (#set! diagnostic.name "unsorted-import")
  (#set! diagnostic.title "Import out of order: `{node.text}`")
  (#set! diagnostic.help "Imports should be in alphabetical order")
  (#set! diagnostic.label "sorts after")
  (#set! diagnostic.context.label "sorts before")
  (#set! diagnostic.severity "info"))

; Unsorted static imports
; @see https://google.github.io/styleguide/javaguide.html#s3.3.3-import-ordering-and-spacing
(program
  (import_declaration
    (scoped_identifier) @error) @_node1
  .
  (import_declaration
    "static"
    (scoped_identifier) @context)
  (#not-match? @_node1 "^import\\s+static")
  (#set! diagnostic.name "unsorted-import-group")
  (#set! diagnostic.title "Import out of order: `{node.text}`")
  (#set! diagnostic.help "Static imports should be grouped before regular imports")
  (#set! diagnostic.label "sorts after")
  (#set! diagnostic.context.label "sorts before")
  (#set! diagnostic.severity "info"))

(program
  (import_declaration
    (scoped_identifier) @error) @_node1
  .
  [
    (block_comment)
    (line_comment)
  ]+
  .
  (import_declaration
    "static"
    (scoped_identifier) @context)
  (#not-match? @_node1 "^import\\s+static")
  (#set! diagnostic.name "unsorted-import-group")
  (#set! diagnostic.title "Import out of order: `{node.text}`")
  (#set! diagnostic.help "Static imports should be grouped before regular imports")
  (#set! diagnostic.label "sorts after")
  (#set! diagnostic.context.label "sorts before")
  (#set! diagnostic.severity "info"))

; Multiple top-level classes in the same file
; @see https://google.github.io/styleguide/javaguide.html#s3.4.1-one-top-level-class
(program
  .
  [
    (package_declaration)
    (import_declaration)
    (line_comment)
    (block_comment)
  ]+
  .
  [
    (class_declaration
      name: (identifier) @context)
    (interface_declaration
      name: (identifier) @context)
    (record_declaration
      name: (identifier) @context)
    (enum_declaration
      name: (identifier) @context)
    (annotation_type_declaration
      name: (identifier) @context)
  ]
  ; parser uses ABI 14
  [
    (class_declaration
      name: (identifier) @error)
    (interface_declaration
      name: (identifier) @error)
    (record_declaration
      name: (identifier) @error)
    (enum_declaration
      name: (identifier) @error)
    (annotation_type_declaration
      name: (identifier) @error)
  ]
  (#set! diagnostic.name "multiple-classes")
  (#set! diagnostic.title "Multiple top-level classes: `{node.text}`")
  (#set! diagnostic.label "Additional class")
  (#set! diagnostic.context.label "First class")
  (#set! diagnostic.help "Move `{node.text}` to separate `{node.text}.java` file")
  (#set! diagnostic.severity "warn"))

; One variable per declaration
; @see https://google.github.io/styleguide/javaguide.html#s4.8.2-variable-declarations
(block
  (local_variable_declaration
    type: (_)
    .
    (variable_declarator
      name: (identifier) @context)
    .
    (variable_declarator
      name: (identifier) @error))
  (#set! diagnostic.name "multiple-declaration")
  (#set! diagnostic.title "Multiple variable declaration: `{node.text}`")
  (#set! diagnostic.label "Additional variable")
  (#set! diagnostic.context.label "First variable")
  (#set! diagnostic.help "Move `{node.text}` to separate declaration")
  (#set! diagnostic.severity "info"))

; Integer literal with lowercase 'l'
; @see https://google.github.io/styleguide/javaguide.html#s4.8.8-numeric-literals
((decimal_integer_literal) @error
  (#match? @error "l$")
  (#set! diagnostic.name "lowercase-long-literal")
  (#set! diagnostic.title "Lowercase long integer literal: `{node.text}`")
  (#set! diagnostic.help "Replace with uppercase L suffix to improve legibility")
  (#set! diagnostic.severity "info")) ; TODO: autofix

; Dollar sign in identifier
; @see https://google.github.io/styleguide/javaguide.html#s5.1-identifier-names
((identifier) @error
  (#match? @error "[$]")
  (#set! diagnostic.name "dollar-identifier")
  (#set! diagnostic.title "Dollar sign in identifier: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using only ASCII letters, digits, and underscores")
  (#set! diagnostic.severity "info"))

; Identifier containing unicode character
; @see https://google.github.io/styleguide/javaguide.html#s5.1-identifier-names
((identifier) @error
  (#match? @error "[^a-zA-Z0-9_$]")
  (#set! diagnostic.name "unicode-identifier")
  (#set! diagnostic.title "Unicode in identifier: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using only ASCII letters, digits, and underscores")
  (#set! diagnostic.severity "warn"))

; Package names should be lowercase and digits only
; @see https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
(package_declaration
  (identifier) @error
  (#match? @error "[A-Z]")
  (#set! diagnostic.name "uppercase-package")
  (#set! diagnostic.title "Uppercase in package: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using only lowercase and digits")
  (#set! diagnostic.severity "warn"))

; Package names should be lowercase and digits only
; @see https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
(package_declaration
  (identifier) @error
  (#match? @error "[_]")
  (#set! diagnostic.name "underscore-package")
  (#set! diagnostic.title "Underscore in package: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using only lowercase and digits")
  (#set! diagnostic.severity "warn"))

; Module names should be lowercase and digits only
; @see https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
(module_declaration
  (identifier) @error
  (#match? @error "[A-Z]")
  (#set! diagnostic.name "uppercase-module")
  (#set! diagnostic.title "Uppercase in module: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using only lowercase and digits")
  (#set! diagnostic.severity "warn"))

; Module names should be lowercase and digits only
; @see https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
(module_declaration
  (identifier) @error
  (#match? @error "[_]")
  (#set! diagnostic.name "underscore-module")
  (#set! diagnostic.title "Underscore in module: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using only lowercase and digits")
  (#set! diagnostic.severity "warn"))

; Class names should be UpperCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.2-class-names
(class_declaration
  name: (identifier) @error
  (#match? @error "^[a-z]")
  (#set! diagnostic.name "lowercase-class")
  (#set! diagnostic.title "Lowercase class: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using UpperCamelCase")
  (#set! diagnostic.severity "warn"))

; Class names should be UpperCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.2-class-names
(record_declaration
  name: (identifier) @error
  (#match? @error "^[a-z]")
  (#set! diagnostic.name "lowercase-record")
  (#set! diagnostic.title "Lowercase record: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using UpperCamelCase")
  (#set! diagnostic.severity "warn"))

; Class names should be UpperCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.2-class-names
(enum_declaration
  name: (identifier) @error
  (#match? @error "^[a-z]")
  (#set! diagnostic.name "lowercase-enum")
  (#set! diagnostic.title "Lowercase enum: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using UpperCamelCase")
  (#set! diagnostic.severity "warn"))

; Class names should be UpperCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.2-class-names
(interface_declaration
  name: (identifier) @error
  (#match? @error "^[a-z]")
  (#set! diagnostic.name "lowercase-interface")
  (#set! diagnostic.title "Lowercase interface: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using UpperCamelCase")
  (#set! diagnostic.severity "warn"))

; Class names should be UpperCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.2-class-names
(annotation_type_declaration
  name: (identifier) @error
  (#match? @error "^[a-z]")
  (#set! diagnostic.name "lowercase-annotation")
  (#set! diagnostic.title "Lowercase annotation: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using UpperCamelCase")
  (#set! diagnostic.severity "warn"))

; Method names should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.3-method-names
(method_declaration
  name: (identifier) @error
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-method")
  (#set! diagnostic.title "Uppercase method: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "warn"))

; Method names should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.3-method-names
(annotation_type_element_declaration
  name: (identifier) @error
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-element")
  (#set! diagnostic.title "Uppercase annotation element: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "warn"))

; Enumerated type constants should be UPPER_SNAKE_CASE
; @see https://google.github.io/styleguide/javaguide.html#s5.2.4-constant-names
(enum_constant
  name: (identifier) @error
  (#match? @error "[a-z]")
  (#set! diagnostic.name "lowercase-enum-constant")
  (#set! diagnostic.title "Lowercase in enum constant: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using UPPER_SNAKE_CASE")
  (#set! diagnostic.severity "info"))

; Primitive type constants should be UPPER_SNAKE_CASE
; @see https://google.github.io/styleguide/javaguide.html#s5.2.4-constant-names
(field_declaration
  (modifiers) @_modifiers
  type: [
    (boolean_type)
    (integral_type)
    (floating_point_type)
  ] @context
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @_modifiers "final")
  (#match? @_modifiers "static")
  (#match? @error "[a-z]")
  (#not-eq? @error "serialVersionUID")
  (#set! diagnostic.name "lowercase-primitive-constant")
  (#set! diagnostic.title "Lowercase in constant field: `{node.text}`")
  (#set! diagnostic.context.label "Immutable type")
  (#set! diagnostic.help "Rename `{node.text}` using UPPER_SNAKE_CASE")
  (#set! diagnostic.severity "info"))

; String constants should be UPPER_SNAKE_CASE
; @see https://google.github.io/styleguide/javaguide.html#s5.2.4-constant-names
(field_declaration
  (modifiers) @_modifiers
  type: (type_identifier) @_type @context
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @_modifiers "final")
  (#match? @_modifiers "static")
  (#match? @error "[a-z]")
  (#eq? @_type "String")
  (#set! diagnostic.name "lowercase-string-constant")
  (#set! diagnostic.title "Lowercase in constant field: `{node.text}`")
  (#set! diagnostic.context.label "Immutable type")
  (#set! diagnostic.help "Rename `{node.text}` using UPPER_SNAKE_CASE")
  (#set! diagnostic.severity "info"))

; Null constants should be UPPER_SNAKE_CASE
; @see https://google.github.io/styleguide/javaguide.html#s5.2.4-constant-names
(field_declaration
  (modifiers) @_modifiers
  declarator: (variable_declarator
    name: (identifier) @error
    value: (null_literal) @context)
  (#match? @_modifiers "final")
  (#match? @_modifiers "static")
  (#match? @error "[a-z]")
  (#set! diagnostic.name "lowercase-null-constant")
  (#set! diagnostic.title "Lowercase in constant field: `{node.text}`")
  (#set! diagnostic.context.label "Immutable")
  (#set! diagnostic.help "Rename `{node.text}` using UPPER_SNAKE_CASE")
  (#set! diagnostic.severity "info"))

; Empty array constants should be UPPER_SNAKE_CASE
; @see https://google.github.io/styleguide/javaguide.html#s5.2.4-constant-names
(field_declaration
  (modifiers) @_modifiers
  declarator: (variable_declarator
    name: (identifier) @error
    value: (array_initializer) @_array @context)
  (#match? @_modifiers "final")
  (#match? @_modifiers "static")
  (#match? @error "[a-z]")
  (#match? @_array "^[{]\\s*[}]$")
  (#set! diagnostic.name "lowercase-array-constant")
  (#set! diagnostic.title "Lowercase in constant field: `{node.text}`")
  (#set! diagnostic.context.label "Immutable")
  (#set! diagnostic.help "Rename `{node.text}` using UPPER_SNAKE_CASE")
  (#set! diagnostic.severity "info"))

; non-constants should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.5-non-constant-field-names
(field_declaration
  (modifiers)? @_modifiers
  type: (_) @context
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#not-match? @_modifiers "final")
  (#not-match? @_modifiers "static")
  (#set! diagnostic.name "uppercase-field")
  (#set! diagnostic.title "Uppercase field: `{node.text}`")
  (#set! diagnostic.context.label "Not `static final`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "warn"))

; non-constants should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.5-non-constant-field-names
(field_declaration
  (modifiers) @_modifiers
  type: (_) @context
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @_modifiers "static")
  (#not-match? @_modifiers "final")
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-static-field")
  (#set! diagnostic.title "Uppercase mutable static field: `{node.text}`")
  (#set! diagnostic.context.label "Not `static final`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase, or make `static final`")
  (#set! diagnostic.severity "warn"))

; non-constants should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.5-non-constant-field-names
(field_declaration
  (modifiers) @_modifiers
  type: (_) @context
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @_modifiers "final")
  (#not-match? @_modifiers "static")
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-final-field")
  (#set! diagnostic.title "Uppercase field: `{node.text}`")
  (#set! diagnostic.context.label "Not `static final`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "info"))

; Parameters should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
(formal_parameter
  name: (identifier) @error
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-param")
  (#set! diagnostic.title "Uppercase parameter: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "warn")) @visible

; Varargs parameter should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
(spread_parameter
  (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-vararg")
  (#set! diagnostic.title "Uppercase vararg: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "warn")) @visible

; Catch parameters should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
(catch_formal_parameter
  name: (identifier) @error
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-catch-param")
  (#set! diagnostic.title "Uppercase catch parameter: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "warn")) @visible

; Try-with-resource parameters should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
(resource
  name: (identifier) @error
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-resource")
  (#set! diagnostic.title "Uppercase resource: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "warn")) @visible

; Local variables should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.7-local-variable-names
(local_variable_declaration
  .
  type: (_)
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-local")
  (#set! diagnostic.title "Uppercase local variable: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "warn"))

; Local variables should be lowerCamelCase (final variant)
; @see https://google.github.io/styleguide/javaguide.html#s5.2.7-local-variable-names
(local_variable_declaration
  .
  (modifiers) @_modifiers @context
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#match? @_modifiers "final")
  (#set! diagnostic.name "uppercase-final-local")
  (#set! diagnostic.title "Uppercase local variable: `{node.text}`")
  (#set! diagnostic.context.label "Not `static final`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "info"))

; Local variables should be lowerCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.7-local-variable-names
(enhanced_for_statement
  name: (identifier) @error
  (#match? @error "^[A-Z]")
  (#set! diagnostic.name "uppercase-for-local")
  (#set! diagnostic.title "Uppercase local variable: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using lowerCamelCase")
  (#set! diagnostic.severity "warn"))

; Type variables should be UpperCamelCase
; @see https://google.github.io/styleguide/javaguide.html#s5.2.8-type-variable-names
(type_parameter
  (type_identifier) @error
  (#match? @error "^[a-z]")
  (#set! diagnostic.name "lowercase-type")
  (#set! diagnostic.title "Lowercase type parameter: `{node.text}`")
  (#set! diagnostic.help "Rename `{node.text}` using UpperCamelCase")
  (#set! diagnostic.severity "warn")) @visible

; Caught exceptions: not ignored
; @see https://google.github.io/styleguide/javaguide.html#s6.2-caught-exceptions
(catch_clause
  (catch_formal_parameter
    (catch_type)
    name: (identifier) @error)
  body: (block) @_block
  ; unnamed variable
  (#not-any-of? @error "_" "ignored" "tolerated" "accepted" "acceptable" "ok" "success" "optional")
  (#not-match? @error "^expected.*")
  ; no real content at all
  (#not-match? @_block "[a-zA-Z0-9_]")
  (#set! diagnostic.name "swallowed-exception")
  (#set! diagnostic.title "Unhandled caught exception: `{node.text}`")
  (#set! diagnostic.help "Handle `{node.text}`, add a comment, or indicate via unnamed variable `_`")
  (#set! diagnostic.fix.kind "static")
  (#set! diagnostic.fix.arg "_")
  (#set! diagnostic.severity "info")) @visible ; body is small (empty)

; Finalizers: not used
; @see https://google.github.io/styleguide/javaguide.html#s6.4-finalizers
(method_declaration
  type: (void_type) @visible
  ; body could be large
  name: (identifier) @error
  parameters: (formal_parameters) @_params
  (#eq? @error "finalize")
  ; only parentheses
  (#match? @_params "^[\\s]*[(][\\s]*[)][\\s]*$")
  (#set! diagnostic.name "finalizer-used")
  (#set! diagnostic.title "Finalizer used: `{node.text}`")
  (#set! diagnostic.help
    "Migrate to other resource management such as try-with-resources or cleaners")
  (#set! diagnostic.severity "warn"))
