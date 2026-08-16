; open block: increase indent
("{" @node
  (#set! "format.indent.delta" 1)
  (#set! "format.space.before" true)
  (#set! "format.newline.after" true))

; close block: reduce indent
("}" @node
  (#set! "format.indent.delta" -1)
  (#set! "format.newline.after" true))

; end of statement, newline
(";" @node
  (#set! "format.newline.after" true))

; marker annotation, newline
((marker_annotation
  name: (_) @node)
  (#set! "format.newline.after" true))

; regular annotation, newline
((annotation_argument_list
  ")" @node .)
  (#set! "format.newline.after" true))

; comments: indent them, newline them (for now)
([
  (line_comment)
  (block_comment)
] @node
  (#set! "format.newline.after" true))

; space after comment
("," @node
  (#set! "format.space.after" true))

; space between type and the name
((formal_parameter
  name: (_) @node)
  (#set! "format.space.before" true))

; space between type and name
((catch_formal_parameter
  name: (_) @node)
  (#set! "format.space.before" true))

; space between type and name
((variable_declarator
  name: (_) @node)
  (#set! "format.space.before" true))

; space between type and name
((enhanced_for_statement
  name: (_) @node)
  (#set! "format.space.before" true))

(type_arguments
  "<" @node)

(type_arguments
  ">" @node)

; no space between @interface
((annotation_type_declaration
  "@"
  "interface" @node)
  (#set! "format.space.after" true))

([
  "abstract"
  "assert"
  "break"
  "case"
  "catch"
  "class"
  "continue"
  "default"
  "do"
  "else"
  "enum"
  "exports"
  "extends"
  "final"
  "finally"
  "for"
  "if"
  "implements"
  "import"
  "interface"
  "module"
  "native"
  "non-sealed"
  "open"
  "opens"
  "package"
  "permits"
  "provides"
  "private"
  "protected"
  "public"
  "requires"
  "record"
  "return"
  "sealed"
  "static"
  "strictfp"
  "switch"
  "synchronized"
  "throw"
  "throws"
  "to"
  "transient"
  "transitive"
  "try"
  "uses"
  "volatile"
  "when"
  "while"
  "with"
  "yield"
  "new"
  "instanceof"
  "="
  "+="
  "-="
  "*="
  "/="
  "&="
  "|="
  "^="
  "%="
  "<<="
  ">>="
  ">>>="
  ">"
  "<"
  ">="
  "<="
  "=="
  "!="
  "&&"
  "||"
  "+"
  "-"
  "*"
  "/"
  "&"
  "|"
  "^"
  "%"
  "<<"
  ">>"
  ">>>"
  "->"
  ":"
] @node
  (#set! "format.space.before" true)
  (#set! "format.space.after" true))

((assert_statement
  ":" @node)
  (#set! "format.space.before" true)
  (#set! "format.space.after" true))

((enhanced_for_statement
  ":" @node)
  (#set! "format.space.before" true)
  (#set! "format.space.after" true))

((method_declaration
  name: (identifier) @node)
  (#set! "format.space.before" true))

_ @node
