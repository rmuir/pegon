; local variables can only be accessed after they are declared. they may shadow fields
((block
  (local_variable_declaration
    declarator: (variable_declarator
      name: (identifier) @definition)) @start
  "}" @end)
  (#set! local.type "variable")
  (#set! local.start.inclusive false))

((for_statement
  init: (local_variable_declaration
    declarator: (variable_declarator
      name: (identifier) @definition)) @start
  body: (statement) @end)
  (#set! local.type "variable")
  (#set! local.start.inclusive false))

((enhanced_for_statement
  name: (identifier) @definition @start)
  (#set! local.type "variable")) @end

((try_with_resources_statement
  resources: (resource_specification
    (resource
      name: (identifier) @definition) @start)
  body: (block) @end)
  (#set! local.type "variable"))

; JEP 394
((instanceof_expression
  name: (identifier) @definition @start @end)
  (#set! local.type "variable")
  (#set! local.flow true))

; JEP 440
((instanceof_expression
  pattern: (record_pattern
    (record_pattern_body
      (record_pattern_component
        (identifier) @definition @start @end .))))
  (#set! local.type "variable")
  (#set! local.flow true))

; JEP 441
((switch_rule
  (switch_label
    (pattern
      (type_pattern
        (identifier) @definition @start .)))) @end
  (#set! local.type "variable"))

; parameters can be accessed inside respective bodies of the things they parameterize
((lambda_expression
  parameters: [
    (identifier) @definition
    (formal_parameters
      (formal_parameter
        name: (identifier) @definition))
    (inferred_parameters
      (identifier) @definition)
  ]
  body: [
    (expression)
    (block)
  ] @start @end)
  (#set! local.type "parameter"))

((constructor_declaration
  parameters: (formal_parameters
    [
      (formal_parameter
        name: (identifier) @definition)
      (spread_parameter
        (variable_declarator
          name: (identifier) @definition))
    ])
  body: (constructor_body) @start @end)
  (#set! local.type "parameter"))

((record_declaration
  parameters: (formal_parameters
    (formal_parameter
      name: (identifier) @definition))
  body: (class_body) @start @end)
  (#set! local.type "property"))

((method_declaration
  parameters: (formal_parameters
    [
      (formal_parameter
        name: (identifier) @definition)
      (spread_parameter
        (variable_declarator
          name: (identifier) @definition))
    ])
  body: (block) @start @end)
  (#set! local.type "parameter"))

((catch_clause
  (catch_formal_parameter
    name: (identifier) @definition)
  body: (block) @start @end)
  (#set! local.type "parameter"))

; type parameter scopes
((class_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! local.type "typeParameter"))

((constructor_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! local.type "typeParameter"))

((record_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! local.type "typeParameter"))

((interface_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! local.type "typeParameter"))

((method_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! local.type "typeParameter"))
