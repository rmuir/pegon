; classes
; anonymous class: not represented here but needed for proper hierarchy
(object_creation_expression
  type: (type_identifier) @selection
  (class_body) @range
  (#set! "kind" "Class"))

; kind=11 (Interface) ?
(annotation_type_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  name: (identifier) @selection
  (#set! "kind" "Interface")) @range

; kind=5 (Class)
(class_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  name: (identifier) @selection
  type_parameters: (type_parameters)? @detail
  (#set! "kind" "Class")) @range

; kind=10 (Enum)
(enum_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  name: (identifier) @selection
  (#set! "kind" "Enum")) @range

; kind=11 (Interface)
(interface_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  name: (identifier) @selection
  type_parameters: (type_parameters)? @detail
  (#set! "kind" "Interface")) @range

; kind=23 (Struct fallback to Class)
(record_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  name: (identifier) @selection
  type_parameters: (type_parameters)? @detail
  (#set! "kind" "Struct")) @range

; "members"
; kind=6 (Method)
(annotation_type_element_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  type: (_) @detail
  name: (identifier) @selection
  (#set! "kind" "Method")) @range

; kind=9 (Constructor)
(compact_constructor_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  name: (identifier) @selection
  (#set! "kind" "Constructor")) @range

; kind=14 (Constant)
(constant_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#set! "kind" "Constant")) @range

; kind=9 (Constructor)
(constructor_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  name: (identifier) @selection
  parameters: (formal_parameters) @detail
  (#set! "kind" "Constructor")) @range

; kind=22 (EnumMember fallback to Field)
(enum_constant
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  name: (identifier) @selection
  (#set! "kind" "EnumMember")) @range

; kind=8 (Field)
(field_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @selection)
  (#set! "kind" "Field")) @range

; kind=6 (Method)
(method_declaration
  (modifiers
    (marker_annotation
      name: (identifier) @deprecated
      (#eq? @deprecated "Deprecated")))?
  name: (identifier) @selection
  parameters: (formal_parameters) @detail
  (#set! "kind" "Method")) @range
