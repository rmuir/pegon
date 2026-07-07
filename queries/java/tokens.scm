; Semantic Token Selector             Neovim Treesitter       Neovim Syntax         Mini.Hues         VSCode TextMate Scope
;
; class                               @type                   Type                  Type              entity.name.type.class
; class.defaultLibrary                                                                                support.class
; comment                             @comment                Comment
; decorator                           @attribute              Macro                 Macro
; enum                                @type                   Type                  Type              entity.name.type.enum
; enumMember                          @constant               Constant              Constant          variable.other.enummember
; event                               @type                   Type                                    variable.other.event
; function                            @function               Function              Function          entity.name.function
; function.defaultLibrary                                                                             support.function
; interface                           @type                   Type                  Type              entity.name.type.interface
; keyword                             @keyword                Keyword
; macro                               @constant.macro         Constant              PreProc           entity.name.function.preprocessor
; method                              @function.method        Function              Function          entity.name.function.member
; modifier                            @type.qualifier         Type
; namespace                           @module                 Structure             Identifier        entity.name.namespace
; number                              @number                 Number
; operator                            @operator               Operator
; parameter                           @variable.parameter     Variable              "blue"            variable.parameter
; property                            @property               Identifier            Identifier        variable.other.property
; property.readonly                                                                                   variable.other.constant.property
; regexp                              @string.regexp          SpecialChar
; string                              @string                 String
; struct                              @type                   Type                  Type              storage.type.struct
; type                                @type                   Type                  Type              entity.name.type
; type.defaultLibrary                                                                                 support.type
; typeParameter                       @type.definition        Type                  Type
; variable                            @variable               Variable              Variable          variable.other.readwrite , entity.name.variable
; variable.readonly                                                                                   variable.other.constant
; variable.readonly.defaultLibrary                                                                    support.constant
; *.deprecated                                                DiagnosticDeprecated  "red"
; comments
;------------------
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
  (#set! token.modifiers "modification"))

((update_expression
  [
    (identifier) @range
    (array_access
      array: (identifier) @range)
  ])
  (#set! token.type "variable")
  (#set! token.modifiers "modification"))

((assignment_expression
  left: [
    (field_access
      field: (identifier) @range)
    (array_access
      array: (field_access
        field: (identifier) @range))
  ])
  (#set! token.type "property")
  (#set! token.modifiers "modification"))

((update_expression
  [
    (field_access
      field: (identifier) @range)
    (array_access
      array: (field_access
        field: (identifier) @range))
  ])
  (#set! token.type "property")
  (#set! token.modifiers "modification"))

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
  (#set! token.modifiers "definition"))

((enhanced_for_statement
  name: (identifier) @range)
  (#set! token.type "variable")
  (#set! token.modifiers "definition"))

((instanceof_expression
  name: (identifier) @range)
  (#set! token.type "variable")
  (#set! token.modifiers "definition"))

((record_pattern_component
  (identifier) @range .)
  (#set! token.type "variable")
  (#set! token.modifiers "definition"))

((type_pattern
  (identifier) @range .)
  (#set! token.type "variable")
  (#set! token.modifiers "definition"))

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
  (#set! token.modifiers "readonly,static"))

; Types
((interface_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifiers "definition"))

((class_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifiers "definition"))

((record_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifiers "definition"))

((enum_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifiers "definition"))

((constructor_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifiers "definition"))

((compact_constructor_declaration
  name: (identifier) @range)
  (#set! token.type "type")
  (#set! token.modifiers "definition"))

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
  (#set! token.modifiers "defaultLibrary"))

; builtin-type
((type_identifier) @range
  (#eq? @range "var")
  (#set! token.type "type")
  (#set! token.modifiers "defaultLibrary"))

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
  (#set! token.modifiers "definition"))

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
  (#set! token.modifiers "definition"))

((method_invocation
  name: (identifier) @range)
  (#set! token.type "method"))

((method_reference
  (identifier) @range .)
  (#set! token.type "method"))

; new as a method reference
((method_reference
  "new" @range .)
  (#set! token.type "method")
  (#set! token.modifiers "defaultLibrary"))

; Parameters
((formal_parameter
  name: (identifier) @range)
  (#set! token.type "parameter")
  (#set! token.modifiers "definition"))

((catch_formal_parameter
  name: (identifier) @range)
  (#set! token.type "parameter")
  (#set! token.modifiers "definition"))

((spread_parameter
  (variable_declarator
    name: (identifier) @range)) ; int... foo
  (#set! token.type "parameter")
  (#set! token.modifiers "definition"))

; Lambda parameter
((inferred_parameters
  (identifier) @range) ; (x,y) -> ...
  (#set! token.type "parameter")
  (#set! token.modifiers "definition"))

((lambda_expression
  parameters: (identifier) @range) ; x -> ...
  (#set! token.type "parameter")
  (#set! token.modifiers "definition"))

; type parameters
((type_parameter
  (type_identifier) @range)
  (#set! token.type "typeParameter")
  (#set! token.modifiers "definition"))

; decorators
("@" @range
  (#set! token.type "decorator"))

((annotation_type_declaration
  name: (identifier) @range)
  (#set! token.type "decorator")
  (#set! token.modifiers "definition"))

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
  (#set! token.modifiers "definition"))

((element_value_pair
  key: (identifier) @range)
  (#set! token.type "property"))

; record "parameters" are really properties of the record
((record_declaration
  parameters: (formal_parameters
    (formal_parameter
      name: (identifier) @range)))
  (#set! token.type "property")
  (#set! token.modifiers "definition"))

; builtin variables
([
  (this)
  (super)
] @range
  (#set! token.type "variable")
  (#set! token.modifiers "defaultLibrary,readonly"))
