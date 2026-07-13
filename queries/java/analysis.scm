((program
  (import_declaration
    (scoped_identifier
      name: (identifier) @definition @start) @type) @_node) @end
  (#not-match? @_node "^import\\s+static")
  (#set! analysis.kind "type"))

((class_body
  (field_declaration
    type: (_) @type
    declarator: (variable_declarator
      name: (identifier) @definition))) @start @end
  (#set! analysis.kind "property"))

; local variables can only be accessed after they are declared. they may shadow fields
((block
  (local_variable_declaration
    type: (_) @type
    declarator: (variable_declarator
      name: (identifier) @definition)) @start
  "}" @end)
  (#set! analysis.kind "variable")
  (#set! analysis.start.inclusive false))

((for_statement
  init: (local_variable_declaration
    type: (_) @type
    declarator: (variable_declarator
      name: (identifier) @definition)) @start
  body: (statement) @end)
  (#set! analysis.kind "variable")
  (#set! analysis.start.inclusive false))

((enhanced_for_statement
  type: (_) @type
  name: (identifier) @definition @start)
  (#set! analysis.kind "variable")) @end

((try_with_resources_statement
  resources: (resource_specification
    (resource
      type: (_) @type
      name: (identifier) @definition) @start)
  body: (block) @end)
  (#set! analysis.kind "variable"))

; JEP 394
((instanceof_expression
  right: (_) @type
  name: (identifier) @definition @start @end)
  (#set! analysis.kind "variable")
  (#set! analysis.flow true))

; JEP 440
((instanceof_expression
  pattern: (record_pattern
    (record_pattern_body
      (record_pattern_component
        (_) @type
        .
        (identifier) @definition @start @end .))))
  (#set! analysis.kind "variable")
  (#set! analysis.flow true))

; JEP 441
((switch_rule
  (switch_label
    (pattern
      (type_pattern
        (_) @type
        .
        (identifier) @definition @start .)))) @end
  (#set! analysis.kind "variable"))

; parameters can be accessed inside respective bodies of the things they parameterize
((lambda_expression
  parameters: [
    (identifier) @definition
    (formal_parameters
      (formal_parameter
        type: (_) @type
        name: (identifier) @definition))
    (inferred_parameters
      (identifier) @definition)
  ]
  body: [
    (expression)
    (block)
  ] @start @end)
  (#set! analysis.kind "parameter"))

((constructor_declaration
  parameters: (formal_parameters
    [
      (formal_parameter
        type: (_) @type
        name: (identifier) @definition)
      (spread_parameter
        type: (_) @type
        (variable_declarator
          name: (identifier) @definition))
    ])
  body: (constructor_body) @start @end)
  (#set! analysis.kind "parameter"))

((record_declaration
  parameters: (formal_parameters
    (formal_parameter
      type: (_) @type
      name: (identifier) @definition))
  body: (class_body) @start @end)
  (#set! analysis.kind "property"))

((method_declaration
  parameters: (formal_parameters
    [
      (formal_parameter
        type: (_) @type
        name: (identifier) @definition)
      (spread_parameter
        type: (_) @type
        (variable_declarator
          name: (identifier) @definition))
    ])
  body: (block) @start @end)
  (#set! analysis.kind "parameter"))

((catch_clause
  (catch_formal_parameter
    (catch_type) @type
    name: (identifier) @definition)
  body: (block) @start @end)
  (#set! analysis.kind "parameter"))

; type parameter scopes
((class_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition
      (type_bound)? @type) @start)) @end
  (#set! analysis.kind "typeParameter"))

((constructor_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition
      (type_bound)? @type) @start)) @end
  (#set! analysis.kind "typeParameter"))

((record_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition
      (type_bound)? @type) @start)) @end
  (#set! analysis.kind "typeParameter"))

((interface_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition
      (type_bound)? @type) @start)) @end
  (#set! analysis.kind "typeParameter"))

((method_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition
      (type_bound)? @type) @start)) @end
  (#set! analysis.kind "typeParameter"))
