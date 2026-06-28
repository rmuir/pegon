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
; Types
((interface_declaration
  name: (identifier) @range)
  (#set! tokens.type "type"))

((annotation_type_declaration
  name: (identifier) @range)
  (#set! tokens.type "type"))

((class_declaration
  name: (identifier) @range)
  (#set! tokens.type "type"))

((record_declaration
  name: (identifier) @range)
  (#set! tokens.type "type"))

((enum_declaration
  name: (identifier) @range)
  (#set! tokens.type "type"))

((constructor_declaration
  name: (identifier) @range)
  (#set! tokens.type "type"))

((compact_constructor_declaration
  name: (identifier) @range)
  (#set! tokens.type "type"))

((type_identifier) @range
  (#set! tokens.type "type"))

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
  (#match? @range "^[a-z]+$")
  (#set! tokens.type "namespace"))

; new java.lang.xxx()
(scoped_type_identifier
  (type_identifier) @range
  (#match? @range "^[a-z]+$")
  (#set! tokens.type "namespace"))

; Variables
; ((identifier) @constant
;  (#match? @constant "^[A-Z_][A-Z0-9_]+$"))
; Fields
((field_declaration
  declarator: (variable_declarator
    name: (identifier) @range))
  (#set! tokens.type "property"))

((field_access
  field: (identifier) @range)
  (#set! tokens.type "property"))
