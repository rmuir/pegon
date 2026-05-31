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
    (modifier) @range))
  (#eq? @range "abstract")
  (#set! hover.description "This class isn't concrete: only subclasses can be instantiated.")
  (#set! hover.kind "abstract class modifier")
  (#set! hover.spec "jls-8.html#jls-8.1.1.1"))
