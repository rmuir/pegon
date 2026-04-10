; anonymous class
(object_creation_expression
  type: (type_identifier) @collapsed
  (class_body)) @range

(annotation_type_declaration
  name: (identifier) @collapsed) @range

(class_declaration
  name: (identifier) @collapsed
  type_parameters: (type_parameters)? @detail) @range

(enum_declaration
  name: (identifier) @collapsed) @range

(interface_declaration
  name: (identifier) @collapsed
  type_parameters: (type_parameters)? @detail) @range

; kind=23 (Struct fallback to Class)
(record_declaration
  name: (identifier) @collapsed
  type_parameters: (type_parameters)? @detail
  (#set! "kind" "Struct")) @range

; "members"
; kind=6 (Method)
(annotation_type_element_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier)?
  type: (_) @detail
  name: (identifier) @selection
  (#set! "kind" "Method")) @range

; kind=9 (Constructor)
(compact_constructor_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier)?
  name: (identifier) @selection
  (#set! "kind" "Constructor")) @range

; kind=14 (Constant)
(constant_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier)?
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#set! "kind" "Constant")) @range

; kind=9 (Constructor)
(constructor_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier)?
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
  (#set! "kind" "Constructor")) @range

; kind=22 (EnumMember fallback to Field)
(enum_constant
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier)?
  name: (identifier) @selection
  (#set! "kind" "EnumMember")) @range

; kind=14 (Constant)
; static final field
(field_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier) @_modifiers
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#match? @_modifiers "final")
  (#match? @_modifiers "static")
  (#set! "kind" "Constant")) @range

; kind=8 (Field)
; final field
(field_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier) @_modifiers
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#match? @_modifiers "final")
  (#not-match? @_modifiers "static")
  (#set! "kind" "Field")) @range

; kind=8 (Field)
; static field
(field_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier) @_modifiers
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#not-match? @_modifiers "final")
  (#match? @_modifiers "static")
  (#set! "kind" "Field")) @range

; kind=8 (Field)
; member field
(field_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier)? @_modifiers
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#not-match? @_modifiers "final")
  (#not-match? @_modifiers "static")
  (#set! "kind" "Field")) @range

; kind=12 (Function)
(method_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier) @_modifiers
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
  (#set! "kind" "Function")) @range

; kind=6 (Method)
(method_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @marker)*
    [
      (modifier)
      (visibility)
    ]* @modifier)? @_modifiers
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
  (#set! "kind" "Method")) @range
