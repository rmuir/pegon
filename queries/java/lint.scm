; TS parsing error
([
  (ERROR)
  (MISSING)
] @error
  (#set! name "parse-error")
  (#set! title "Parse Error")
  (#set! note "Correct the invalid Java syntax")
  (#set! severity "hint"))

; Whitespace other than ASCII horizontal space inside a literal.
; https://google.github.io/styleguide/javaguide.html#s2.3.1-whitespace-characters
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

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\010" "\\10")
  (#set! name "octal-backspace")
  (#set! title "Octal backspace escape")
  (#set! label "Backspace")
  (#set! note "Replace with the special escape `\\b`")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0008")
  (#set! name "unicode-backspace")
  (#set! title "Unicode backspace escape")
  (#set! label "Backspace")
  (#set! note "Replace with the special escape `\\b`")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\011" "\\11")
  (#set! name "octal-tab")
  (#set! title "Octal tab escape")
  (#set! label "Tab")
  (#set! note "Replace with the special escape `\\t`")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#eq? @error "\\u0009")
  (#set! name "unicode-tab")
  (#set! title "Unicode tab escape")
  (#set! label "Tab")
  (#set! note "Replace with the special escape `\\t`")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\012" "\\12")
  (#set! name "octal-newline")
  (#set! title "Octal newline escape")
  (#set! label "Newline")
  (#set! note "Replace with the special escape `\\n`")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000a" "\\u000A")
  (#set! name "unicode-newline")
  (#set! title "Unicode newline escape")
  (#set! label "Newline")
  (#set! note "Replace with the special escape `\\n`")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000c" "\\u000C" "\\014" "\\14")
  (#set! name "wrong-escape")
  (#set! title "Form feed escaped incorrectly")
  (#set! note "Replace the octal/hex escape with the special escape \\f")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000d" "\\u000D" "\\015" "\\15")
  (#set! name "wrong-escape")
  (#set! title "Carriage return escaped incorrectly")
  (#set! note "Replace the octal/hex escape with the special escape \\r")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0022" "\\042" "\\42")
  (#set! name "wrong-escape")
  (#set! title "Double quote escaped incorrectly")
  (#set! note "Replace the octal/hex escape with the special escape \\\"")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0027" "\\047" "\\47")
  (#set! name "wrong-escape")
  (#set! title "Single quote escaped incorrectly")
  (#set! note "Replace the octal/hex escape with the special escape \\'")
  (#set! severity "error"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u005c" "\\u005C" "\\134")
  (#set! name "wrong-escape")
  (#set! title "Backslash escaped incorrectly")
  (#set! note "Replace the raw escape with the special escape \\\\")
  (#set! severity "error"))

; line-wrapped package declaration
; https://google.github.io/styleguide/javaguide.html#s3.2-package-declaration
((package_declaration) @error
  (#match? @error "\n")
  (#set! name "wrapped-package")
  (#set! title "Line-wrapped package declaration")
  (#set! label "Wrapped")
  (#set! note "Remove newlines from the package statement")
  (#set! severity "error"))

; wildcard imports
; https://google.github.io/styleguide/javaguide.html#s3.3.1-wildcard-imports
((import_declaration
  (_) ; TODO: thing being imported
  (asterisk) @error)
  (#set! name "wildcard-import")
  (#set! title "Wildcard import")
  (#set! label "Wildcard")
  (#set! note "Replace the wildcard import with standard import(s)")
  (#set! severity "error"))

; line-wrapped imports
; https://google.github.io/styleguide/javaguide.html#s3.3.2-import-line-wrapping
((import_declaration) @error
  (#match? @error "\n")
  (#set! name "line-wrapped-import")
  (#set! title "Line-wrapped import")
  (#set! label "Wrapped")
  (#set! note "Remove newlines from the import statement")
  (#set! severity "error"))

; multiple top-level classes in the same file
; https://google.github.io/styleguide/javaguide.html#s3.4.1-one-top-level-class
(program
  (class_declaration)
  (class_declaration
    name: (identifier) @error)
  (#set! name "multiple-classes")
  (#set! title "Multiple top-level classes")
  (#set! label "Additional class in file")
  (#set! note "Move top-level classes into their own files: only one per file")
  (#set! severity "error"))

; integer literal with lowercase 'l'
; https://google.github.io/styleguide/javaguide.html#s4.8.8-numeric-literals
((decimal_integer_literal) @error
  (#match? @error "l$")
  (#set! name "lowercase-long-literal")
  (#set! title "Lowercase long integer literal")
  (#set! label "Lowercase")
  (#set! note "Replace with uppercase L suffix to improve legibility")
  (#set! severity "error"))

; dollar sign in identifier
; https://google.github.io/styleguide/javaguide.html#s5.1-identifier-names
((identifier) @error
  (#match? @error "[$]")
  (#set! name "dollar-in-identifier")
  (#set! title "Dollar sign in identifier")
  (#set! label "Identifier")
  (#set! note "Rename using only ASCII letters, digits, and underscores")
  (#set! severity "error"))

; identifier containing unicode character
; https://google.github.io/styleguide/javaguide.html#s5.1-identifier-names
((identifier) @error
  (#match? @error "[^a-zA-Z0-9_$]")
  (#set! name "unicode-identifier")
  (#set! title "Unicode in identifier")
  (#set! label "Identifier")
  (#set! note "Rename using only ASCII letters, digits, and underscores")
  (#set! severity "error"))

; package names should be lowercase and digits only
; https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
((package_declaration
  (identifier) @error)
  (#match? @error "[^a-z0-9]")
  (#set! name "invalid-package-name")
  (#set! title "Invalid package name")
  (#set! label "Package")
  (#set! note "Rename package using only lowercase and digits")
  (#set! severity "error"))

; module names should be lowercase and digits only
; https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
((module_declaration
  (identifier) @error)
  (#match? @error "[^a-z0-9]")
  (#set! name "invalid-module-name")
  (#set! title "Invalid module name")
  (#set! label "Module")
  (#set! note "Rename module using only lowercase and digits")
  (#set! severity "error"))

; class names should be UpperCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.2-class-names
((class_declaration
  name: (identifier) @error)
  (#match? @error "^[a-z]")
  (#set! name "lowercase-class-name")
  (#set! title "Lowercase class name")
  (#set! label "Lowercase")
  (#set! note "Rename class using UpperCamelCase")
  (#set! severity "error"))

; parameters should be lowerCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
((formal_parameter
  name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! name "uppercase-param-name")
  (#set! title "Uppercase parameter name")
  (#set! label "Uppercase")
  (#set! note "Rename parameter using lowerCamelCase")
  (#set! severity "error")) @visible

; spread parameter should be lowerCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
(spread_parameter
  (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! name "uppercase-vararg-name")
  (#set! title "Uppercase vararg parameter name")
  (#set! label "Uppercase")
  (#set! note "Rename vararg parameter using lowerCamelCase")
  (#set! severity "error")) @visible

; local variables should be lowerCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.7-local-variable-names
(local_variable_declaration
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! name "uppercase-local-name")
  (#set! title "Uppercase local variable name")
  (#set! label "Uppercase")
  (#set! note "Rename local variable using lowerCamelCase")
  (#set! severity "error")) @visible

; type variables should be UpperCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.8-type-variable-names
((type_parameter
  (type_identifier) @error)
  (#match? @error "^[a-z]")
  (#set! name "lowercase-type-name")
  (#set! title "Lowercase type parameter name")
  (#set! label "Lowercase")
  (#set! note "Rename type using UpperCamelCase")
  (#set! severity "error")) @visible

; Caught exceptions: not ignored
; https://google.github.io/styleguide/javaguide.html#s6.2-caught-exceptions
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
  (#set! title "Unhandled caught exception")
  (#set! label "Exception ignored")
  (#set! note "Handle the exception, add a comment, or indicate via unnamed variable _")
  (#set! severity "error")) @visible ; body is small (empty)

; Finalizers: not used
; https://google.github.io/styleguide/javaguide.html#s6.4-finalizers
((method_declaration
  type: (void_type) @visible
  ; body could be large
  name: (identifier) @error
  parameters: (formal_parameters) @_params)
  (#eq? @error "finalize")
  ; only parentheses
  (#match? @_params "^[\\s]*[(][\\s]*[)][\\s]*$")
  (#set! name "finalizer-used")
  (#set! title "Finalizer used")
  (#set! label "Overrides Object.finalize()")
  (#set! note "Migrate to other resource management such as try-with-resources or cleaners")
  (#set! severity "error"))
