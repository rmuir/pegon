; classes
; kind=11 (Interface) ?
(annotation_type_declaration
  name: (identifier) @name) @range

; kind=5 (Class)
(class_declaration
  name: (identifier) @name
  type_parameters: (type_parameters)? @detail) @range

; kind=10 (Enum)
(enum_declaration
  name: (identifier) @name) @range

; kind=11 (Interface)
(interface_declaration
  name: (identifier) @name
  type_parameters: (type_parameters)? @detail) @range

; kind=23 (Struct fallback to Class)
(record_declaration
  name: (identifier) @name
  type_parameters: (type_parameters)? @detail) @range

; "members"
; kind=6 (Method)
(annotation_type_element_declaration
  type: (_) @detail
  name: (identifier) @name) @range

; kind=9 (Constructor)
(compact_constructor_declaration
  name: (identifier) @name) @range

; kind=14 (Constant)
(constant_declaration
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @name)) @range

; kind=9 (Constructor)
(constructor_declaration
  name: (identifier) @name
  parameters: (formal_parameters) @detail) @range

; kind=22 (EnumMember fallback to Field)
(enum_constant
  name: (identifier) @name) @range

; kind=8 (Field)
(field_declaration
  type: (_) @detail
  declarator: (variable_declarator
    name: (identifier) @name)) @range

; kind=6 (Method)
(method_declaration
  name: (identifier) @name
  parameters: (formal_parameters) @detail) @range

; anonymous class: not represented here but needed for proper hierarchy
(object_creation_expression
  type: (type_identifier) @name
  (class_body) @definition.class) ; anonymous
