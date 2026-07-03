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
  (#set! tokens.type "keyword"))

; Operators
([
  "new"
  "instanceof"
] @range
  (#set! tokens.type "operator"))

; modifiers
((modifiers
  [
    (modifier)
    (visibility)
  ] @range)
  (#set! tokens.type "modifier"))

((requires_modifier) @range
  (#set! tokens.type "modifier"))

; labels
((labeled_statement
  (identifier) @range)
  (#set! tokens.type "label"))

((break_statement
  (identifier) @range)
  (#set! tokens.type "label"))

((continue_statement
  (identifier) @range)
  (#set! tokens.type "label"))

; constants
((identifier) @range
  (#match? @range "^[A-Z_][A-Z0-9_]+$")
  (#set! tokens.type "property")
  (#set! tokens.modifiers "readonly,static"))

; Types
((interface_declaration
  name: (identifier) @range)
  (#set! tokens.type "type")
  (#set! tokens.modifiers "definition"))

((annotation_type_declaration
  name: (identifier) @range)
  (#set! tokens.type "decorator")
  (#set! tokens.modifiers "definition"))

((class_declaration
  name: (identifier) @range)
  (#set! tokens.type "type")
  (#set! tokens.modifiers "definition"))

((record_declaration
  name: (identifier) @range)
  (#set! tokens.type "type")
  (#set! tokens.modifiers "definition"))

((enum_declaration
  name: (identifier) @range)
  (#set! tokens.type "type")
  (#set! tokens.modifiers "definition"))

((constructor_declaration
  name: (identifier) @range)
  (#set! tokens.type "type")
  (#set! tokens.modifiers "definition"))

((compact_constructor_declaration
  name: (identifier) @range)
  (#set! tokens.type "type")
  (#set! tokens.modifiers "definition"))

((type_identifier) @range
  (#set! tokens.type "type"))

; builtin-types
([
  (boolean_type)
  (integral_type)
  (floating_point_type)
  (void_type)
] @range
  (#set! tokens.type "type")
  (#set! tokens.modifiers "defaultLibrary"))

; builtin-type
((type_identifier) @range
  (#eq? @range "var")
  (#set! tokens.type "type")
  (#set! tokens.modifiers "defaultLibrary"))

(((method_invocation
  object: (identifier) @range)
  (#match? @range "^[A-Z]"))
  (#set! tokens.type "type"))

(((method_reference
  .
  (identifier) @range)
  (#match? @range "^[A-Z]"))
  (#set! tokens.type "type"))

(((field_access
  object: (identifier) @range)
  (#match? @range "^[A-Z]"))
  (#set! tokens.type "type"))

((scoped_identifier
  (identifier) @range
  (#match? @range "^[A-Z]"))
  (#set! tokens.type "type"))

; imports java.lang.xxx
(scoped_identifier
  (identifier) @range
  (#match? @range "^[a-z_][a-z0-9_]+$")
  (#set! tokens.type "namespace"))

; new java.lang.xxx()
(scoped_type_identifier
  (type_identifier) @range
  (#match? @range "^[a-z_][a-z0-9_]+$")
  (#set! tokens.type "namespace"))

; fields
((field_declaration
  declarator: (variable_declarator
    name: (identifier) @range))
  (#set! tokens.type "property")
  (#set! tokens.modifiers "definition"))

; field access
((field_access
  field: (identifier) @range)
  (#set! tokens.type "property"))

; nested class access
((field_access
  field: (identifier) @range)
  (#match? @range "^[A-Z].*[a-z]")
  (#set! tokens.type "type"))

((method_declaration
  name: (identifier) @range)
  (#set! tokens.type "method")
  (#set! tokens.modifiers "definition"))

((method_invocation
  name: (identifier) @range)
  (#set! tokens.type "method"))

((method_reference
  (identifier) @range .)
  (#set! tokens.type "method"))

; new as a method reference
((method_reference
  "new" @range .)
  (#set! tokens.type "method")
  (#set! tokens.modifiers "defaultLibrary"))

; Parameters
((formal_parameter
  name: (identifier) @range)
  (#set! tokens.type "parameter")
  (#set! tokens.modifiers "definition"))

((catch_formal_parameter
  name: (identifier) @range)
  (#set! tokens.type "parameter")
  (#set! tokens.modifiers "definition"))

((spread_parameter
  (variable_declarator
    name: (identifier) @range)) ; int... foo
  (#set! tokens.type "parameter")
  (#set! tokens.modifiers "definition"))

; Lambda parameter
((inferred_parameters
  (identifier) @range) ; (x,y) -> ...
  (#set! tokens.type "parameter")
  (#set! tokens.modifiers "definition"))

((lambda_expression
  parameters: (identifier) @range) ; x -> ...
  (#set! tokens.type "parameter")
  (#set! tokens.modifiers "definition"))

; decorators
; TODO: do a has-ancestor or similar here, not quite right
((annotation
  name: (identifier) @range)
  (#set! tokens.type "decorator"))

((annotation
  name: (scoped_identifier
    name: (identifier) @range))
  (#set! tokens.type "decorator"))

((marker_annotation
  name: (identifier) @range)
  (#set! tokens.type "decorator"))

((marker_annotation
  name: (scoped_identifier
    name: (identifier) @range))
  (#set! tokens.type "decorator"))

((annotation_type_element_declaration
  name: (identifier) @range)
  (#set! tokens.type "property")
  (#set! tokens.modifiers "definition"))

((element_value_pair
  key: (identifier) @range)
  (#set! tokens.type "property"))

; builtin variables
([
  (this)
  (super)
] @range
  (#set! tokens.type "variable")
  (#set! tokens.modifiers "defaultLibrary,readonly"))
