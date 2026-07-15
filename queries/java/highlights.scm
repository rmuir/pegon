; Document highlighting
;
; For example, when hovering over a catch block, related try and finally are highlighted.
; The feature can help when there is deep nesting (which is common in java)
;
; TBD: locals make a lot of sense to use here
; ---
; if-else
((if_statement
  "if" @range @reference
  "else"? @range @reference)
  (#set! highlight.kind 2))

; do-while
((do_statement
  "do" @range @reference
  "while" @range @reference)
  (#set! highlight.kind 2))

; switch/case/default
((switch_expression
  "switch" @range @reference
  body: (switch_block
    [
      (switch_block_statement_group
        (switch_label
          [
            "case"
            "default"
          ] @range @reference))
      (switch_rule
        (switch_label
          [
            "case"
            "default"
          ] @range @reference))
    ]*))
  (#set! highlight.kind 2))

; try/catch/finally
((try_statement
  "try" @range @reference
  [
    (catch_clause
      "catch" @range @reference)
    (finally_clause
      "finally" @range @reference)
  ]*)
  (#set! highlight.kind 2))

; try/catch/finally
((try_with_resources_statement
  "try" @range @reference
  [
    (catch_clause
      "catch" @range @reference)
    (finally_clause
      "finally" @range @reference)
  ]*)
  (#set! highlight.kind 2))

; do block start/end
((do_statement
  "do" @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; synchronized block start/end
((synchronized_statement
  "synchronized" @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; try block start/end
((try_statement
  "try" @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; catch block start/end
((catch_clause
  "catch" @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; finally block start/end
((finally_clause
  "finally" @reference
  (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; try-with-resources block start/end
((try_with_resources_statement
  "try" @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; if block start/end
((if_statement
  "if" @reference
  consequence: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; if block start/end
((if_statement
  "else" @reference
  alternative: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; while block start/end
((while_statement
  "while" @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; for block start/end
((for_statement
  "for" @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; for block start/end
((enhanced_for_statement
  "for" @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; module block start-end
((module_declaration
  "module" @reference
  name: [
    (identifier)
    (scoped_identifier)
  ] @reference
  body: (module_body
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; enum block start-end
((enum_declaration
  "enum" @reference
  name: (identifier) @reference
  body: (enum_body
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; enum constant block start-end
((enum_constant
  name: (identifier) @reference
  body: (class_body
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; class block start-end
((class_declaration
  "class" @reference
  name: (identifier) @reference
  body: (class_body
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; static block start-end
((static_initializer
  "static" @reference
  (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; constructor block start-end
((constructor_declaration
  name: (identifier) @reference
  body: (constructor_body
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; record block start-end
((record_declaration
  "record" @reference
  name: (identifier) @reference
  body: (class_body
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; annotation type block start-end
((annotation_type_declaration
  "@interface" @reference
  name: (identifier) @reference
  body: (annotation_type_body
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; interface block start-end
((interface_declaration
  "interface" @reference
  name: (identifier) @reference
  body: (interface_body
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; array initializer start-end
((variable_declarator
  name: [
    (identifier)
    (underscore_pattern)
  ] @reference
  "=" @reference
  value: (array_initializer
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; method block start-end
((method_declaration
  name: (identifier) @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))

; compact constructor block start-end
((compact_constructor_declaration
  name: (identifier) @reference
  body: (block
    "{" @range @reference
    "}" @range @reference))
  (#set! highlight.kind 2))
