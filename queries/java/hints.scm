; synchronized block start/end
((synchronized_statement
  "synchronized" @label @location
  (parenthesized_expression) @label
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; try block start/end
((try_statement
  "try" @label @location
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; catch block start/end
((catch_clause
  "catch" @label @location
  (catch_formal_parameter
    (catch_type) @label)
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; finally block start/end
((finally_clause
  "finally" @label @location
  (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; try-with-resources block start/end
((try_with_resources_statement
  "try" @label @location
  resources: (resource_specification
    "("
    .
    (resource
      name: (_) @label
      "=" @label))
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.suffix " …")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; if block start/end
((if_statement
  "if" @label @location
  condition: (parenthesized_expression
    (expression) @label)
  consequence: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; if block start/end
((if_statement
  "else" @label @location
  alternative: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; while block start/end
((while_statement
  "while" @label @location
  condition: (parenthesized_expression
    (expression) @label)
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; for block start/end
((for_statement
  "for" @label @location
  condition: (_) @label
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; for block start/end
((enhanced_for_statement
  "for" @label @location
  name: (_) @label
  ":" @label
  value: (_) @label
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; switch block start/end
((switch_expression
  "switch" @label @location
  condition: (parenthesized_expression
    (expression) @label)
  body: (switch_block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; module block start-end
((module_declaration
  "module" @label @location
  name: (_) @label
  body: (module_body
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; enum block start-end
((enum_declaration
  "enum" @label @location
  name: (_) @label
  body: (enum_body
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; enum constant block start-end
((enum_constant
  name: (_) @label
  body: (class_body
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; enum block start-end
((class_declaration
  "class" @label @location
  name: (_) @label
  body: (class_body
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; static block start-end
((static_initializer
  "static" @label @location
  (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.suffix " {}")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; constructor block start-end
((constructor_declaration
  name: (_) @label
  body: (constructor_body
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.suffix "()")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; record block start-end
((record_declaration
  "record" @label @location
  name: (_) @label
  body: (class_body
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; annotation type block start-end
((annotation_type_declaration
  "@interface" @label @location
  name: (_) @label
  body: (annotation_type_body
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; interface block start-end
((interface_declaration
  "interface" @label @location
  name: (_) @label
  body: (interface_body
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; array initializer start-end
((variable_declarator
  name: (_) @label
  "=" @label
  value: (array_initializer
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; method block start-end
((method_declaration
  name: (_) @label
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.suffix "()")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; compact constructor block start-end
((compact_constructor_declaration
  name: (_) @label
  body: (block
    "}" @position)) @_region
  (#match? @_region "\n")
  (#eol? @position)
  (#set! hint.prefix "// ")
  (#set! hint.suffix "()")
  (#set! hint.pad.left true)
  (#set! hint.pad.medial true))

; diamond type inference
(local_variable_declaration
  type: (generic_type
    (type_arguments
      [
        (type_identifier)
        (scoped_type_identifier)
        (generic_type)
        ","
      ]+ @label))
  declarator: (variable_declarator
    value: (object_creation_expression
      type: (generic_type
        (type_arguments
          .
          "<" @position
          .
          ">" .)))))

; diamond type inference
(field_declaration
  type: (generic_type
    (type_arguments
      [
        (type_identifier)
        (scoped_type_identifier)
        (generic_type)
        ","
      ]+ @label))
  declarator: (variable_declarator
    value: (object_creation_expression
      type: (generic_type
        (type_arguments
          .
          "<" @position
          .
          ">" .)))))
