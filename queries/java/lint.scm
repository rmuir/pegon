; TS parsing error
([
  (ERROR)
  (MISSING)
] @error
  (#set! name "parse-error")
  (#set! severity "warning")
  (#set! title "Parse Error")
  (#set! label "Parse error here")
  (#set! help "Correct the invalid Java syntax"))

; https://google.github.io/styleguide/javaguide.html#s2.3.1-whitespace-characters
([
  (character_literal)
  (string_fragment)
  (multiline_string_fragment)
] @error
  (#match? @error "[\\s&&[^\\u0020\n]]")
  (#set! severity "error")
  (#set! name "illegal-whitespace")
  (#set! title "Illegal whitespace character")
  (#set! label "Whitespace used here")
  (#set! help "Escape the character: only 0x20 may appear in literals"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0008" "\\010" "\\10")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with the special escape \\b"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0009" "\\011" "\\11")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with the special escape \\t"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000a" "\\u000A" "\\012" "\\12")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with the special escape \\n"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000c" "\\u000C" "\\014" "\\14")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with the special escape \\f"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000d" "\\u000D" "\\015" "\\15")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with the special escape \\r"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0022" "\\042" "\\42")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with the special escape \\\""))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0027" "\\047" "\\47")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with the special escape \\'"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u005c" "\\u005C" "\\134")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with the special escape \\\\"))

; line-wrapped package declaration
; https://google.github.io/styleguide/javaguide.html#s3.2-package-declaration
((package_declaration) @error
  (#match? @error "\n")
  (#set! severity "error")
  (#set! name "wrapped-package")
  (#set! title "Do not line-wrap package declarations")
  (#set! label "package declared here")
  (#set! help "Remove newlines from the package statement"))

; wildcard imports
; https://google.github.io/styleguide/javaguide.html#s3.3.1-wildcard-imports
((import_declaration
  (asterisk) @error)
  (#set! severity "error")
  (#set! name "wildcard-import")
  (#set! title "Do not use wildcard imports")
  (#set! label "Wildcard used here")
  (#set! help "Replace the wildcard import with standard import(s)"))

; line-wrapped imports
; https://google.github.io/styleguide/javaguide.html#s3.3.2-import-line-wrapping
((import_declaration) @error
  (#match? @error "\n")
  (#set! severity "error")
  (#set! name "wrapped-import")
  (#set! title "Do not line-wrap imports")
  (#set! label "import declared here")
  (#set! help "Remove newlines from the import statement"))

; multiple top-level classes in the same file
; https://google.github.io/styleguide/javaguide.html#s3.4.1-one-top-level-class
(program
  (class_declaration)
  (class_declaration
    name: (identifier) @error)
  (#set! name "multiple-classes")
  (#set! severity "error")
  (#set! title "Multiple top-level classes in this file")
  (#set! label "Additional top-level class defined here")
  (#set! help "Move top-level classes into their own files: only one per file"))

; integer literal with lowercase 'l'
; https://google.github.io/styleguide/javaguide.html#s4.8.8-numeric-literals
((decimal_integer_literal) @error
  (#match? @error "l$")
  (#set! name "lower-long-literal")
  (#set! severity "error")
  (#set! title "Lowercase suffix used for long literal")
  (#set! label "literal used here")
  (#set! help "Change to an uppercase L suffix to improve legibility"))

; identifier containing illegal character
; https://google.github.io/styleguide/javaguide.html#s5.1-identifier-names
((identifier) @error
  (#match? @error "[^a-zA-Z0-9_]")
  (#set! severity "error")
  (#set! name "invalid-identifier-name")
  (#set! title "Illegal characters used in identifier")
  (#set! label "identifier here")
  (#set! help "Change to use only ASCII letters, digits, and underscores"))

; package names should be lowercase and digits only
; https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
((package_declaration
  (identifier) @error)
  (#match? @error "[^a-z0-9]")
  (#set! severity "error")
  (#set! name "invalid-package-name")
  (#set! title "Illegal characters used in package name")
  (#set! label "package declared here")
  (#set! help "Change package name to use lowercase and digits"))

; module names should be lowercase and digits only
; https://google.github.io/styleguide/javaguide.html#s5.2.1-package-names
((module_declaration
  (identifier) @error)
  (#match? @error "[^a-z0-9]")
  (#set! severity "error")
  (#set! name "invalid-module-name")
  (#set! title "Illegal characters used in module name")
  (#set! label "module declared here")
  (#set! help "Change module name to use lowercase and digits"))

; class names should be UpperCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.2-class-names
((class_declaration
  name: (identifier) @error)
  (#match? @error "^[a-z]")
  (#set! severity "error")
  (#set! name "invalid-class-name")
  (#set! title "Lowercase class name")
  (#set! label "class declared here")
  (#set! help "Change class name to use UpperCamelCase"))

; parameters should be lowerCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
(formal_parameters
  ((formal_parameter
    name: (identifier) @error)
    (#match? @error "^[A-Z]")
    (#set! severity "error")
    (#set! name "invalid-param-name")
    (#set! title "Uppercase parameter name")
    (#set! label "parameter declared here")
    (#set! help "Change parameter name to use lowerCamelCase"))) @visible

; spread parameter should be lowerCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.6-parameter-names
(formal_parameters
  (spread_parameter
    (variable_declarator
      name: (identifier) @error)
    (#match? @error "^[A-Z]")
    (#set! severity "error")
    (#set! name "invalid-param-name")
    (#set! title "Uppercase parameter name")
    (#set! label "parameter declared here")
    (#set! help "Change parameter name to use lowerCamelCase"))) @visible

; local variables should be lowerCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.7-local-variable-names
(local_variable_declaration
  declarator: (variable_declarator
    name: (identifier) @error)
  (#match? @error "^[A-Z]")
  (#set! severity "error")
  (#set! name "invalid-local-name")
  (#set! title "Uppercase local variable name")
  (#set! label "variable declared here")
  (#set! help "Change variable name to use lowerCamelCase: not static final")) @visible

; type variables should be UpperCamelCase
; https://google.github.io/styleguide/javaguide.html#s5.2.8-type-variable-names
((type_parameter
  (type_identifier) @error)
  (#match? @error "^[a-z]")
  (#set! severity "error")
  (#set! name "invalid-type-name")
  (#set! title "Lowercase type name")
  (#set! label "type variable declared here")
  (#set! help "Change type name to use UpperCamelCase"))

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
  (#set! severity "error")
  (#set! name "ignored-caught-exception")
  (#set! title "Caught exception ignored")
  (#set! label "exception being ignored")
  (#set! help "Handle the exception, add a comment, or indicate via unnamed variable _")) @visible

; Finalizers: not used
; https://google.github.io/styleguide/javaguide.html#s6.4-finalizers
((method_declaration
  type: (void_type)
  name: (identifier) @error
  parameters: (formal_parameters) @_params)
  (#eq? @error "finalize")
  ; only parentheses
  (#match? @_params "^[\\s]*[(][\\s]*[)][\\s]*$")
  (#set! severity "error")
  (#set! name "finalizer-used")
  (#set! title "Do not override Object.finalize")
  (#set! label "override here")
  (#set! help "Migrate to other resource management such as try-with-resources or cleaners"))
