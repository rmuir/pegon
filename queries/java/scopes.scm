; Just enough scope information to do what highlighting needs.
; The main purpose is to distinguish "variable" from "parameter" from "property".
; In java, coloring these consistently and differently can greatly help in code readability.
;
; fields can be declared at the very bottom of the file and accessed anywhere from inside class's body
; fields (e.g. in an object creation expression) can shadow things such as local variables!
((class_body
  (field_declaration
    declarator: (variable_declarator
      name: (identifier) @definition))) @start @end
  (#set! scope.type "property"))

; local variables can only be accessed after they are declared. they may shadow fields
((block
  (local_variable_declaration
    declarator: (variable_declarator
      name: (identifier) @definition)) @start
  "}" @end)
  (#set! scope.type "variable")
  (#set! scope.start.inclusive false))

((for_statement
  init: (local_variable_declaration
    declarator: (variable_declarator
      name: (identifier) @definition)) @start
  body: (statement) @end)
  (#set! scope.type "variable")
  (#set! scope.start.inclusive false))

((enhanced_for_statement
  name: (identifier) @definition @start)
  (#set! scope.type "variable")) @end

((try_with_resources_statement
  resources: (resource_specification
    (resource
      name: (identifier) @definition) @start)
  body: (block) @end)
  (#set! scope.type "variable"))

; JEP 394
((instanceof_expression
  name: (identifier) @definition @start @end)
  (#set! scope.type "variable")
  (#set! scope.flow true))

; JEP 440
((instanceof_expression
  pattern: (record_pattern
    (record_pattern_body
      (record_pattern_component
        (identifier) @definition @start @end .))))
  (#set! scope.type "variable")
  (#set! scope.flow true))

; JEP 441
((switch_rule
  (switch_label
    (pattern
      (type_pattern
        (identifier) @definition @start .)))) @end
  (#set! scope.type "variable"))

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
  (#set! scope.type "parameter"))

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
  (#set! scope.type "parameter"))

((record_declaration
  parameters: (formal_parameters
    (formal_parameter
      name: (identifier) @definition))
  body: (class_body) @start @end)
  (#set! scope.type "property"))

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
  (#set! scope.type "parameter"))

((catch_clause
  (catch_formal_parameter
    name: (identifier) @definition)
  body: (block) @start @end)
  (#set! scope.type "parameter"))

; type parameter scopes
((class_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! scope.type "typeParameter"))

((constructor_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! scope.type "typeParameter"))

((record_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! scope.type "typeParameter"))

((interface_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! scope.type "typeParameter"))

((method_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @definition) @start)) @end
  (#set! scope.type "typeParameter"))
