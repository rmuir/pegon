; anonymous class
; kind=19 (Object fallback to Class)
(object_creation_expression
  type: (type_identifier) @selection
  (class_body)
  (#symbol.kind! 19)) @range

; kind=11 (Interface) ?
(annotation_type_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  name: (identifier) @selection
  (#symbol.kind! 11)) @range

; kind=5 (Class)
(class_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  name: (identifier) @selection
  type_parameters: (type_parameters)? @detail
  (#symbol.kind! 5)) @range

; kind=10 (Enum)
(enum_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  name: (identifier) @selection
  (#symbol.kind! 10)) @range

; kind=11 (Interface)
(interface_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  name: (identifier) @selection
  type_parameters: (type_parameters)? @detail
  (#symbol.kind! 11)) @range

; kind=23 (Struct fallback to Class)
(record_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  name: (identifier) @selection
  type_parameters: (type_parameters)? @detail
  (#symbol.kind! 23)) @range

; "members"
; kind=6 (Method)
(annotation_type_element_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  type: (_) @detail
  name: (identifier) @selection
  (#symbol.kind! 6)) @range

; kind=9 (Constructor)
(compact_constructor_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  name: (identifier) @selection
  (#symbol.kind! 9)) @range

; kind=14 (Constant)
(constant_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#symbol.kind! 14)) @range

; kind=9 (Constructor)
(constructor_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  name: (identifier) @selection
  parameters: (formal_parameters
    "(" @signature
    [
      (receiver_parameter)
      (formal_parameter
        type: (_) @signature
        dimensions: (dimensions)? @signature)
      (spread_parameter
        type: (_) @signature
        "..." @signature
        dimensions: (dimensions)? @signature)
      ","
    ]*
    ")" @signature)
  (#symbol.kind! 9)) @range

; kind=22 (EnumMember fallback to Field)
(enum_constant
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*)?
  name: (identifier) @selection
  (#symbol.kind! 22)) @range

; kind=14 (Constant)
; static final field
(field_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]*) @_modifiers
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#match? @_modifiers "final")
  (#match? @_modifiers "static")
  (#symbol.kind! 14)) @range

; kind=8 (Field)
; final field
(field_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]*) @_modifiers
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#match? @_modifiers "final")
  (#not-match? @_modifiers "static")
  (#symbol.kind! 8)) @range

; kind=8 (Field)
; static field
(field_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]*) @_modifiers
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#not-match? @_modifiers "final")
  (#match? @_modifiers "static")
  (#symbol.kind! 8)) @range

; kind=8 (Field)
; member field
(field_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]*)? @_modifiers
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#not-match? @_modifiers "final")
  (#not-match? @_modifiers "static")
  (#symbol.kind! 8)) @range

; kind=12 (Function)
(method_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]*) @_modifiers
  type: (_) @detail
  name: (identifier) @selection
  parameters: (formal_parameters
    "(" @signature
    [
      (receiver_parameter)
      (formal_parameter
        type: (_) @signature
        dimensions: (dimensions)? @signature)
      (spread_parameter
        type: (_) @signature
        "..." @signature
        dimensions: (dimensions)? @signature)
      ","
    ]*
    ")" @signature)
  (#match? @_modifiers "static")
  (#symbol.kind! 12)) @range

; kind=6 (Method)
(method_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]*)? @_modifiers
  type: (_) @detail
  name: (identifier) @selection
  parameters: (formal_parameters
    "(" @signature
    [
      (receiver_parameter)
      (formal_parameter
        type: (_) @signature
        dimensions: (dimensions)? @signature)
      (spread_parameter
        type: (_) @signature
        "..." @signature
        dimensions: (dimensions)? @signature)
      ","
    ]*
    ")" @signature)
  (#not-match? @_modifiers "static")
  (#symbol.kind! 6)) @range
