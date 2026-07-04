; TODO: capturing and using modifiers would be nice (e.g. readonly), also types, definitely qualified types...
; fields can be declared at the very bottom of the file and accessed anywhere from inside class's body
((class_body
  (field_declaration
    declarator: (variable_declarator
      name: (identifier) @variable))) @start @end
  (#set! scope.type "property"))

; local variables can only be accessed after they are declared. they may shadow fields
; TODO: there are more cases than this like single-statement situations, deal with them
((block
  (local_variable_declaration
    declarator: (variable_declarator
      name: (identifier) @variable)) @start
  "}" @end)
  (#set! scope.type "variable")
  (#set! scope.start.inclusive false))

; parameters can be accessed inside respective bodies of the things they parameterize
((lambda_expression
  parameters: [
    (identifier) @variable
    (formal_parameters
      (formal_parameter
        name: (identifier) @variable))
    (inferred_parameters
      (identifier) @variable)
  ]
  body: (_) @start @end)
  (#set! scope.type "parameter"))

((constructor_declaration
  parameters: (formal_parameters
    [
      (formal_parameter
        name: (identifier) @variable)
      (spread_parameter
        (variable_declarator
          name: (identifier) @variable))
    ])
  body: (constructor_body) @start @end)
  (#set! scope.type "parameter"))

; TODO: not so sure, i think these should be "property" ?
((record_declaration
  parameters: (formal_parameters
    (formal_parameter
      name: (identifier) @variable))
  body: (class_body) @start @end)
  (#set! scope.type "parameter"))

((method_declaration
  parameters: (formal_parameters
    [
      (formal_parameter
        name: (identifier) @variable)
      (spread_parameter
        (variable_declarator
          name: (identifier) @variable))
    ])
  body: (block) @start @end)
  (#set! scope.type "parameter"))

((catch_clause
  (catch_formal_parameter
    name: (identifier) @variable)
  body: (block) @start @end)
  (#set! scope.type "parameter"))

; type parameter scopes
((class_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @variable) @start)) @end
  (#set! scope.type "typeParameter"))

((constructor_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @variable) @start)) @end
  (#set! scope.type "typeParameter"))

((record_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @variable) @start)) @end
  (#set! scope.type "typeParameter"))

((interface_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @variable) @start)) @end
  (#set! scope.type "typeParameter"))

((method_declaration
  type_parameters: (type_parameters
    (type_parameter
      (type_identifier) @variable) @start)) @end
  (#set! scope.type "typeParameter"))
