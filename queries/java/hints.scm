; synchronized block start/end
((synchronized_statement
  "synchronized" @value
  (parenthesized_expression) @value
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n"))

; try block start/end
((try_statement
  "try" @value
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#set! hint.suffix " {…}"))

; catch block start/end
((catch_clause
  "catch" @value
  (catch_formal_parameter
    (catch_type) @value)
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n"))

; finally block start/end
((finally_clause
  "finally" @value
  (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#set! hint.suffix " {…}"))

; try-with-resources block start/end
((try_with_resources_statement
  "try" @value
  resources: (resource_specification
    "("
    .
    (resource
      name: (_) @value
      "=" @value))
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#set! hint.suffix " …"))

; if block start/end
((if_statement
  "if" @value
  condition: (parenthesized_expression
    (expression) @value)
  consequence: (block
    "}" @position)) @_region
  (#match? @_region "\n"))

; if block start/end
((if_statement
  "else" @value
  alternative: (block
    "}" @position)) @_region
  (#match? @_region "\n"))

; while block start/end
((while_statement
  "while" @value
  condition: (parenthesized_expression
    (expression) @value)
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n"))

; for block start/end
((for_statement
  "for" @value
  condition: (_) @value
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n"))

; for block start/end
((enhanced_for_statement
  "for" @value
  name: (_) @value
  ":" @value
  value: (_) @value
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n"))

; switch block start/end
((switch_expression
  "switch" @value
  condition: (parenthesized_expression
    (expression) @value)
  body: (switch_block
    "}" @position)) @_region
  (#match? @_region "\n"))

; module block start-end
((module_declaration
  "module" @value
  name: (_) @value
  body: (module_body
    "}" @position)) @_region
  (#match? @_region "\n"))

; enum block start-end
((enum_declaration
  "enum" @value
  name: (_) @value
  body: (enum_body
    "}" @position)) @_region
  (#match? @_region "\n"))

; enum constant block start-end
((enum_constant
  name: (_) @value
  body: (class_body
    "}" @position)) @_region
  (#match? @_region "\n"))

; enum block start-end
((class_declaration
  "class" @value
  name: (_) @value
  body: (class_body
    "}" @position)) @_region
  (#match? @_region "\n"))

; static block start-end
((static_initializer
  "static" @value
  (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#set! hint.suffix " {…}"))

; constructor block start-end
((constructor_declaration
  name: (_) @value
  body: (constructor_body
    "}" @position)) @_region
  (#match? @_region "\n"))

; record block start-end
((record_declaration
  "record" @value
  name: (_) @value
  body: (class_body
    "}" @position)) @_region
  (#match? @_region "\n"))

; annotation type block start-end
((annotation_type_declaration
  "@interface" @value
  name: (_) @value
  body: (annotation_type_body
    "}" @position)) @_region
  (#match? @_region "\n"))

; interface block start-end
((interface_declaration
  "interface" @value
  name: (_) @value
  body: (interface_body
    "}" @position)) @_region
  (#match? @_region "\n"))

; array initializer start-end
((variable_declarator
  name: (_) @value
  "=" @value
  value: (array_initializer
    "}" @position)) @_region
  (#match? @_region "\n"))

; method block start-end
((method_declaration
  name: (_) @value
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n"))

; compact constructor block start-end
((compact_constructor_declaration
  name: (_) @value
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n"))
