; keywords
[
  "abstract"
  (boolean_type)
  "byte"
  "case"
  "catch"
  "char"
  "class"
  "continue"
  "default"
  "do"
  "double"
  "else"
  "enum"
  "exports"
  "extends"
  "final"
  "finally"
  "float"
  "for"
  "if"
  "implements"
  "import"
  "int"
  "interface"
  "long"
  "module"
  "open"
  "opens"
  "native"
  "non-sealed"
  "new"
  "package"
  "permits"
  "private"
  "protected"
  "provides"
  "public"
  "record"
  "requires"
  "return"
  "sealed"
  "short"
  "static"
  "strictfp"
  (super)
  "switch"
  "synchronized"
  (this)
  "throw"
  "throws"
  "to"
  "transient"
  "transitive"
  "uses"
  ((type_identifier) @_type
    (#eq? @_type "var"))
  (void_type)
  "volatile"
  "when"
  "while"
  "with"
  "yield"
] @selection @range

; assertion keyword
(assert_statement
  "assert" @selection) @range

; instanceof
(instanceof_expression
  "instanceof" @selection) @range

; break (no label)
(break_statement
  "break" @selection) @range

; try
(try_statement
  "try" @selection) @range

(try_with_resources_statement
  "try" @selection) @range

; identifiers
([
  (identifier)
  (type_identifier)
] @range @selection
  (#set! definition.scoped true))

; but not these yet
((field_access
  field: (identifier) @range @selection)
  (#set! definition.bail true))

; and not these yet
((method_invocation
  object: (_)
  name: (identifier) @range @selection)
  (#set! definition.bail true))

; declarations
(module_declaration
  name: [
    (identifier)
    (scoped_identifier)
  ] @selection) @range

(package_declaration
  [
    (identifier)
    (scoped_identifier)
  ] @selection) @range

; specify the import patterns explicitly
; structures don't use fields and its too easy to capture wrong data
; module import
(import_declaration
  "module"
  [
    (identifier)
    (scoped_identifier)
  ] @selection) @range

; static import
(import_declaration
  "static"
  [
    (identifier)
    (scoped_identifier)
  ] @selection) @range

; regular import
(import_declaration
  "import"
  .
  [
    (identifier)
    (scoped_identifier)
  ] @selection) @range

; class-like
(class_declaration
  name: (identifier) @selection) @range

(record_declaration
  name: (identifier) @selection) @range

(interface_declaration
  name: (identifier) @selection) @range

(annotation_type_declaration
  name: (identifier) @selection) @range

(enum_declaration
  name: (identifier) @selection) @range

(object_creation_expression
  "new" @selection) @range

; member-like
(annotation_type_element_declaration
  name: (identifier) @selection) @range

(compact_constructor_declaration
  name: (identifier) @selection) @range

(constant_declaration
  declarator: (variable_declarator
    name: (identifier) @selection)) @range

(constructor_declaration
  name: (identifier) @selection) @range

(enum_constant
  name: (identifier) @selection) @range

(field_declaration
  declarator: (variable_declarator
    name: (identifier) @selection)) @range

(method_declaration
  name: (identifier) @selection) @range

; local-like
(local_variable_declaration
  declarator: (variable_declarator
    name: (identifier) @selection)) @range

(enhanced_for_statement
  name: (identifier) @selection) @range

(formal_parameter
  name: (identifier) @selection) @range

(catch_formal_parameter
  name: (identifier) @selection) @range

(inferred_parameters
  (identifier) @selection @range)

(lambda_expression
  parameters: (identifier) @selection) @range
