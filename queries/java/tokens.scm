; There's a lot of patterns because I found that most syntax highlighting falls apart badly
; on newer java features. This includes VSCode regex highlighting and to some degree, tree-sitter-java
; that users might be using. Definitely vim regex highlighting is not in a great place!
;
; The locals.scm is used, to differentiate "variable" from "parameter" from "property", which
; is really needed for a Java developer. In java these are all accessed the same way (implicit "this"),
; and shadowing is common and even idiomatic!
; ---
; keywords
([
  "assert"
  "break"
  "case"
  "catch"
  "class"
  "continue"
  "do"
  "else"
  "enum"
  "exports"
  "extends"
  "finally"
  "for"
  "if"
  "implements"
  "import"
  "interface"
  "module"
  "open"
  "opens"
  "package"
  "permits"
  "provides"
  "public"
  "requires"
  "record"
  "return"
  "static"
  "switch"
  "synchronized"
  "throw"
  "throws"
  "to"
  "transitive"
  "try"
  "uses"
  "when"
  "while"
  "with"
  "yield"
] @range
  (#set! token.type "keyword"))

; Operators
([
  "new"
  "instanceof"
] @range
  (#set! token.type "operator"))

; modifiers
((modifiers
  [
    (modifier)
    (visibility)
  ] @range)
  (#set! token.type "modifier"))

((requires_modifier) @range
  (#set! token.type "modifier"))

; modifications
((assignment_expression
  left: [
    (identifier) @range
    (array_access
      array: (identifier) @range)
  ])
  (#set! token.type "variable")
  (#set! token.modifier "modification"))

((update_expression
  [
    (identifier) @range
    (array_access
      array: (identifier) @range)
  ])
  (#set! token.type "variable")
  (#set! token.modifier "modification"))

((assignment_expression
  left: [
    (field_access
      field: (identifier) @range)
    (array_access
      array: (field_access
        field: (identifier) @range))
  ])
  (#set! token.type "property")
  (#set! token.modifier "modification"))

((update_expression
  [
    (field_access
      field: (identifier) @range)
    (array_access
      array: (field_access
        field: (identifier) @range))
  ])
  (#set! token.type "property")
  (#set! token.modifier "modification"))

; gonna be slow
; fall back to property if we aren't declared within doc
((identifier) @range
  (#set! token.type "property")
  (#set! token.scoped true))

; variable definitions
((local_variable_declaration
  declarator: (variable_declarator
    name: (identifier) @range))
  (#set! token.type "variable")
  (#set! token.modifier "definition"))

((enhanced_for_statement
  name: (identifier) @range)
  (#set! token.type "variable")
  (#set! token.modifier "definition"))

((instanceof_expression
  name: (identifier) @range)
  (#set! token.type "variable")
  (#set! token.modifier "definition"))

((record_pattern_component
  (identifier) @range .)
  (#set! token.type "variable")
  (#set! token.modifier "definition"))

((type_pattern
  (identifier) @range .)
  (#set! token.type "variable")
  (#set! token.modifier "definition"))

; labels
((labeled_statement
  (identifier) @range)
  (#set! token.type "label"))

((break_statement
  (identifier) @range)
  (#set! token.type "label"))

((continue_statement
  (identifier) @range)
  (#set! token.type "label"))

; constants
((identifier) @range
  (#match? @range "^[A-Z_][A-Z0-9_]+$")
  (#set! token.type "property")
  (#set! token.modifier "readonly")
  (#set! token.modifier2 "static"))

; Types
((interface_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifier "definition"))

((class_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifier "definition"))

((record_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifier "definition"))

((enum_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifier "definition"))

((constructor_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifier "definition"))

((compact_constructor_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifier "definition"))

((type_identifier) @range
  (#set! token.type "type")
  (#set! token.scoped true))

; builtin-types
([
  (boolean_type)
  (integral_type)
  (floating_point_type)
  (void_type)
] @range
  (#set! token.type "type")
  (#set! token.modifier "defaultLibrary"))

; builtin-type
((type_identifier) @range
  (#eq? @range "var")
  (#set! token.type "type")
  (#set! token.modifier "defaultLibrary"))

(((method_invocation
  object: (identifier) @range)
  (#match? @range "^[A-Z]"))
  (#set! token.type "type"))

(((method_reference
  .
  (identifier) @range)
  (#match? @range "^[A-Z]"))
  (#set! token.type "type"))

(((field_access
  object: (identifier) @range)
  (#match? @range "^[A-Z]"))
  (#set! token.type "type"))

((scoped_identifier
  (identifier) @range
  (#match? @range "^[A-Z]"))
  (#set! token.type "type"))

; imports java.lang.xxx
(scoped_identifier
  (identifier) @range
  (#match? @range "^[a-z_][a-z0-9_]+$")
  (#set! token.type "namespace"))

; static import java.lang.xxx.YYYY
((import_declaration
  "static"
  (scoped_identifier
    name: (identifier) @range))
  (#match? @range "^[a-z]")
  (#set! token.type "method")
  (#set! token.modifier "static"))

; new java.lang.xxx()
(scoped_type_identifier
  (type_identifier) @range
  (#match? @range "^[a-z_][a-z0-9_]+$")
  (#set! token.type "namespace"))

; fields
((field_declaration
  declarator: (variable_declarator
    name: (identifier) @range))
  (#set! token.type "property")
  (#set! token.modifier "definition"))

; field access
((field_access
  field: (identifier) @range)
  (#set! token.type "property"))

; nested class access
((field_access
  field: (identifier) @range)
  (#match? @range "^[A-Z].*[a-z]")
  (#set! token.type "type"))

((method_declaration
  name: (identifier) @range)
  (#set! token.type "method")
  (#set! token.modifier "definition"))

((method_declaration
  (modifiers
    (modifier
      "static"))
  name: (identifier) @range)
  (#set! token.type "method")
  (#set! token.modifier "static"))

((method_invocation
  name: (identifier) @range)
  (#set! token.type "method"))

; method call on unqualified type
((method_invocation
  object: (identifier) @_receiver
  name: (identifier) @range)
  (#match? @_receiver "^[A-Z].*[a-z]")
  (#set! token.type "method")
  (#set! token.modifier "static"))

; method call on qualified type
((method_invocation
  object: (field_access
    field: (identifier) @_receiver)
  name: (identifier) @range)
  (#match? @_receiver "^[A-Z].*[a-z]")
  (#set! token.type "method")
  (#set! token.modifier "static"))

((method_reference
  (identifier) @range .)
  (#set! token.type "method"))

; new as a method reference
((method_reference
  "new" @range .)
  (#set! token.type "method")
  (#set! token.modifier "defaultLibrary"))

; Parameters
((formal_parameter
  name: (identifier) @range)
  (#set! token.type "parameter")
  (#set! token.modifier "definition"))

((catch_formal_parameter
  name: (identifier) @range)
  (#set! token.type "parameter")
  (#set! token.modifier "definition"))

((spread_parameter
  (variable_declarator
    name: (identifier) @range)) ; int... foo
  (#set! token.type "parameter")
  (#set! token.modifier "definition"))

; Lambda parameter
((inferred_parameters
  (identifier) @range) ; (x,y) -> ...
  (#set! token.type "parameter")
  (#set! token.modifier "definition"))

((lambda_expression
  parameters: (identifier) @range) ; x -> ...
  (#set! token.type "parameter")
  (#set! token.modifier "definition"))

; type parameters
((type_parameter
  (type_identifier) @range)
  (#set! token.type "typeParameter")
  (#set! token.modifier "definition"))

; decorators
("@" @range
  (#set! token.type "decorator"))

((annotation_type_declaration
  name: (identifier) @range)
  (#set! token.type "decorator")
  (#set! token.modifier "definition"))

((annotation
  name: [
    (identifier) @range
    (scoped_identifier
      name: (identifier)) @range
  ])
  (#set! token.type "decorator"))

((marker_annotation
  name: [
    (identifier) @range
    (scoped_identifier
      name: (identifier) @range)
  ])
  (#set! token.type "decorator"))

((annotation_type_element_declaration
  name: (identifier) @range)
  (#set! token.type "property")
  (#set! token.modifier "definition"))

((element_value_pair
  key: (identifier) @range)
  (#set! token.type "property"))

; record "parameters" are really properties of the record
((record_declaration
  parameters: (formal_parameters
    (formal_parameter
      name: (identifier) @range)))
  (#set! token.type "property")
  (#set! token.modifier "definition"))

; builtin variables
([
  (this)
  (super)
] @range
  (#set! token.type "variable")
  (#set! token.modifier "defaultLibrary")
  (#set! token.modifier2 "readonly"))
