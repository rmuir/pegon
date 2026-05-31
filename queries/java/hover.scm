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
  (#set! kind "reserved keyword")
  (#set! spec "https://docs.oracle.com/javase/specs/jls/se26/html/jls-3.html#jls-3.9"))

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
  (#set! kind "contextual keyword")
  (#set! spec "https://docs.oracle.com/javase/specs/jls/se26/html/jls-3.html#jls-3.9"))
