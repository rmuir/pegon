; classes
; kind=11 (Interface) ?
(annotation_type_declaration
  name: (identifier) @name) @definition.annotation

; kind=5 (Class)
(class_declaration
  name: (identifier) @name) @definition.class

; kind=10 (Enum)
(enum_declaration
  name: (identifier) @name) @definition.enum

; kind=11 (Interface)
(interface_declaration
  name: (identifier) @name) @definition.interface

; kind=23 (Struct fallback to Class)
(record_declaration
  name: (identifier) @name) @definition.record

; "members"
; kind=6 (Method)
(annotation_type_element_declaration
  name: (identifier) @name) @definition.annotationElement ; field?

; kind=9 (Constructor)
(compact_constructor_declaration
  name: (identifier) @name) @definition.constructor

; kind=14 (Constant)
(constant_declaration
  declarator: (variable_declarator
    name: (identifier) @name)) @definition.field

; kind=9 (Constructor)
(constructor_declaration
  name: (identifier) @name) @definition.constructor

; kind=22 (EnumMember fallback to Field)
(enum_constant
  name: (identifier) @name) @definition.enum_constant

; kind=8 (Field)
(field_declaration
  declarator: (variable_declarator
    name: (identifier) @name)) @definition.field

; kind=6 (Method)
(method_declaration
  name: (identifier) @name) @definition.method
