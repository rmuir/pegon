; synchronized block start/end
(synchronized_statement
  "synchronized" @value
  (parenthesized_expression) @value
  body: (block
    "}" @position))

; try block start/end
(try_statement
  "try" @value
  body: (block
    "}" @position))

; catch block start/end
(catch_clause
  "catch" @value
  (catch_formal_parameter
    (catch_type) @value)
  body: (block
    "}" @position))

; finally block start/end
(finally_clause
  "finally" @value
  (block
    "}" @position))

; try-with-resources block start/end
(try_with_resources_statement
  "try" @value
  body: (block
    "}" @position))

; if block start/end
(if_statement
  "if" @value
  consequence: (block
    "}" @position))

; if block start/end
(if_statement
  "else" @value
  alternative: (block
    "}" @position))

; while block start/end
(while_statement
  "while" @value
  body: (block
    "}" @position))

; for block start/end
(for_statement
  "for" @value
  body: (block
    "}" @position))

; for block start/end
(enhanced_for_statement
  "for" @value
  body: (block
    "}" @position))

; module block start-end
(module_declaration
  "module" @value
  name: (_) @value
  body: (module_body
    "}" @position))

; enum block start-end
(enum_declaration
  "enum" @value
  name: (_) @value
  body: (enum_body
    "}" @position))

; enum constant block start-end
(enum_constant
  name: (_) @value
  body: (class_body
    "}" @position))

; enum block start-end
(class_declaration
  "class" @value
  name: (_) @value
  body: (class_body
    "}" @position))

; static block start-end
(static_initializer
  "static" @value
  (block
    "}" @position))

; constructor block start-end
(constructor_declaration
  name: (_) @value
  body: (constructor_body
    "}" @position))

; record block start-end
(record_declaration
  "record" @value
  name: (_) @value
  body: (class_body
    "}" @position))

; annotation type block start-end
(annotation_type_declaration
  "@interface" @value
  name: (_) @value
  body: (annotation_type_body
    "}" @position))

; interface block start-end
(interface_declaration
  "interface" @value
  name: (_) @value
  body: (interface_body
    "}" @position))

; array initializer start-end
(variable_declarator
  name: (_) @value
  "=" @value
  value: (array_initializer
    "}" @position))

; method block start-end
(method_declaration
  name: (_) @value
  body: (block
    "}" @position))

; compact constructor block start-end
(compact_constructor_declaration
  name: (_) @value
  body: (block
    "}" @position))
