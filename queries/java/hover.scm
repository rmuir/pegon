; always a keyword
([
  "break"
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
  "try"
  (void_type)
  "volatile"
  "while"
] @range
  (#set! hover.description "You can't name a variable with this word.")
  (#set! hover.kind "reserved keyword")
  (#set! hover.spec "jls-3.html#jls-3.9"))

; sometimes a keyword
([
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
] @range
  (#set! hover.description "You probably shouldn't name a variable with this word.")
  (#set! hover.kind "contextual keyword")
  (#set! hover.spec "jls-3.html#jls-3.9"))

; abstract keyword on a class
((class_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.description "This class isn't concrete: only subclasses can be instantiated.")
  (#set! hover.kind "abstract class modifier")
  (#set! hover.spec "jls-8.html#jls-8.1.1.1"))

; abstract keyword on a method
((method_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.description "This method isn't concrete: a subclass must implement it.")
  (#set! hover.kind "abstract method modifier")
  (#set! hover.spec "jls-8.html#jls-8.4.3.1"))

; abstract keyword on an interface
; TODO: add a diagnostic?
((interface_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.description "All interfaces are abstract: remove this keyword.")
  (#set! hover.kind "abstract interface modifier")
  (#set! hover.spec "jls-9.html#jls-9.1.1.1"))

; assertion keyword
("assert" @range
  (#set! hover.description "Raises `AssertionError` on failure if assertions are enabled.")
  (#set! hover.kind "assert statement")
  (#set! hover.spec "jls-14.html#jls-14.10"))

; boolean keyword
((boolean_type) @range
  (#set! hover.description "Boolean type: `true` or `false`")
  (#set! hover.kind "boolean type")
  (#set! hover.spec "jls-4.html#jls-4.2.5"))

; boolean used as a field
((field_declaration
  type: (boolean_type) @range)
  (#set! hover.description "Boolean type: `true` or `false`. Probably uses a byte of space here.")
  (#set! hover.kind "boolean field")
  (#set! hover.spec "jls-4.html#jls-4.2.5"))

; byte
("byte" @range
  (#set! hover.description "8-bit signed integer: -128 .. 127.")
  (#set! hover.kind "byte type")
  (#set! hover.spec "jls-4.html#jls-4.2.1"))

; char
("char" @range
  (#set! hover.description "16-bit unsigned integer: 0 .. 65,535 (\\u0000-\\uFFFF).")
  (#set! hover.kind "char type")
  (#set! hover.spec "jls-4.html#jls-4.2.1"))

; short
("short" @range
  (#set! hover.description "16-bit signed integer: -32,768 .. 32,767.")
  (#set! hover.kind "short type")
  (#set! hover.spec "jls-4.html#jls-4.2.1"))

; int
("int" @range
  (#set! hover.description "32-bit signed integer: -2,147,483,648 .. 2,147,483,647.")
  (#set! hover.kind "integer type")
  (#set! hover.spec "jls-4.html#jls-4.2.1"))

; long
("long" @range
  (#set! hover.description
    "64-bit signed integer: -9,223,372,036,854,775,808 .. 9,223,372,036,854,775,807.")
  (#set! hover.kind "long integer type")
  (#set! hover.spec "jls-4.html#jls-4.2.1"))

; float
("float" @range
  (#set! hover.description "32-bit IEEE binary32 float: 8-bit exponent.")
  (#set! hover.kind "single-precision floating-point type")
  (#set! hover.spec "jls-4.html#jls-4.2.3"))

; float
("double" @range
  (#set! hover.description "64-bit IEEE binary64 float: 11-bit exponent.")
  (#set! hover.kind "double-precision floating-point type")
  (#set! hover.spec "jls-4.html#jls-4.2.3"))

; instanceof
((instanceof_expression
  "instanceof" @range
  right: (_))
  (#set! hover.description "True if expression is non-null and compatible.")
  (#set! hover.kind "type comparison operator")
  (#set! hover.spec "jls-15.html#jls-15.20.2"))

; instanceof
((instanceof_expression
  "instanceof" @range
  pattern: (_))
  (#set! hover.description "True if expression is non-null and matches.")
  (#set! hover.kind "pattern match operator")
  (#set! hover.spec "jls-15.html#jls-15.20.2"))
