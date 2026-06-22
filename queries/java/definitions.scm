; always a keyword
[
  "case"
  "catch"
  "class"
  "continue"
  "default"
  "do"
  "else"
  "enum"
  "extends"
  "final"
  "finally"
  "for"
  "if"
  "implements"
  "import"
  "interface"
  "native"
  "new"
  "package"
  "private"
  "protected"
  "public"
  "return"
  "short"
  "static"
  "strictfp"
  (super)
  "switch"
  "synchronized"
  (this)
  "throw"
  "throws"
  "transient"
  (void_type)
  "volatile"
  "while"
] @selection @range

; sometimes a keyword
[
  "exports"
  "module"
  "non-sealed"
  "open"
  "opens"
  "permits"
  "provides"
  "record"
  "requires"
  "sealed"
  "to"
  "transitive"
  "uses"
  ((type_identifier) @_type
    (#eq? @_type "var"))
  "when"
  "with"
  "yield"
] @selection @range

; abstract keyword on a class
(class_declaration
  (modifiers
    (modifier
      "abstract") @selection) @range)

; abstract keyword on a method
(method_declaration
  (modifiers
    (modifier
      "abstract") @selection) @range)

; abstract keyword on an interface
; TODO: add a diagnostic?
(interface_declaration
  (modifiers
    (modifier
      "abstract") @selection) @range)

; assertion keyword
(assert_statement
  "assert" @selection) @range

; boolean keyword
(boolean_type) @selection @range

; boolean used as a field
(field_declaration
  type: (boolean_type) @selection) @range

; byte
"byte" @selection @range

; char
"char" @selection @range

; short
"short" @selection @range

; int
"int" @selection @range

; long
"long" @selection @range

; float
"float" @selection @range

; float
"double" @selection @range

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
