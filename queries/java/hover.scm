; always a keyword
([
  "abstract"
  "assert"
  (boolean_type)
  "break"
  (integral_type)
  "case"
  "catch"
  "class"
  "continue"
  "default"
  "do"
  (floating_point_type)
  "double"
  "else"
  "enum"
  "extends"
  "final"
  "finally"
  "for"
  "if"
  "implements"
  "import"
  "instanceof"
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

; abstract keyword on a method
; TODO: add a diagnostic?
((interface_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.description "All interfaces are abstract: remove this keyword.")
  (#set! hover.kind "abstract interface modifier")
  (#set! hover.spec "jls-9.html#jls-9.1.1.1"))

; assertion keyword
((assert_statement
  "assert" @range)
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
