; Solves the same problem tree-sitter locals tries to solve, but works differently.
;
; In java, class's member field can be declared at the very bottom of the file but accessed
; by a method at the very top. Also for some pattern-matching functionality, they've
; abandoned lexical scoping entirely in favor of "flow" scoping. These things require
; special handling.
;
; On the other hand, local variables, parameters, catch parameters, lambda parameters, etc
; aren't allowed to be shadowed. This is nice since it allows for efficient processing, e.g.
; for those, we can build a hash with a list of scope ranges, and you just search backwards.
; We have to still take care of some java peculiarities so that idioms such as following work:
;
;   int x = x; // assign to local from member
;   this.x = x; // assign to member from parameter
;
; To solve that, two ranges are captured, one for the definition, and a start/end for the scope.
;
; There are also some problems with "tree-sitter-highlight" crate that uses the locals.
; The biggest hurdle is that it doesn't support incremental parsing.
; So it seems best to simply run this query before subsequent query that uses it.
; For big files, doing a full reparse hurts under heavy editing.
; ---
((block
  (local_variable_declaration
    type: (_) @type
    declarator: (variable_declarator
      name: (identifier) @definition)) @start
  "}" @end)
  (#set! local.type "variable")
  (#set! local.start.inclusive false))

; when declared in a switch block label, variable's scope
; extends to the remainder of the switch block, it "falls thru"
((switch_block
  (switch_block_statement_group
    (local_variable_declaration
      type: (_) @type
      declarator: (variable_declarator
        name: (identifier) @definition)) @start)
  "}" @end)
  (#set! local.type "variable")
  (#set! local.start.inclusive false))

((for_statement
  init: (local_variable_declaration
    type: (_) @type
    declarator: (variable_declarator
      name: (identifier) @definition)) @start
  body: (statement) @end)
  (#set! local.type "variable")
  (#set! local.start.inclusive false))

((enhanced_for_statement
  type: (_) @type
  name: (identifier) @definition @start)
  (#set! local.type "variable")) @end

((try_with_resources_statement
  resources: (resource_specification
    (resource
      type: (_) @type
      name: (identifier) @definition) @start)
  body: (block) @end)
  (#set! local.type "variable"))

; JEP 394
((instanceof_expression
  right: (_) @type
  name: (identifier) @definition @start @end)
  (#set! local.type "variable")
  (#set! local.flow true))

; JEP 440
((instanceof_expression
  pattern: (record_pattern
    (record_pattern_body
      (record_pattern_component
        (_) @type
        .
        (identifier) @definition @start @end .))))
  (#set! local.type "variable")
  (#set! local.flow true))

; JEP 441
((switch_rule
  (switch_label
    (pattern
      (type_pattern
        (_) @type
        .
        (identifier) @definition @start .)))) @end
  (#set! local.type "variable"))

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
  (#set! local.type "parameter"))

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
  (#set! local.type "parameter"))

((record_declaration
  parameters: (formal_parameters
    (formal_parameter
      type: (_) @type
      name: (identifier) @definition))
  body: (class_body) @start @end)
  (#set! local.type "property"))

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
  (#set! local.type "parameter"))

((catch_clause
  (catch_formal_parameter
    (catch_type) @type
    name: (identifier) @definition)
  body: (block) @start @end)
  (#set! local.type "parameter"))

; type parameter scopes
; TODO: support optional type bounds as @type?
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
