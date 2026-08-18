; if-with-else, reduce indent, but don't force a newline
((if_statement
  consequence: (block
    "}" @node .)
  .
  "else")
  (#set! "format.indent.before" -1))

; try block followed by catch/finally, reduce indent, but don't force a newline
((try_statement
  body: (block
    "}" @node .)
  .
  [
    (catch_clause)
    (finally_clause)
  ])
  (#set! "format.indent.before" -1))

; try-with-resources block followed by catch/finally, reduce indent, but don't force a newline
((try_with_resources_statement
  body: (block
    "}" @node .)
  .
  [
    (catch_clause)
    (finally_clause)
  ])
  (#set! "format.indent.before" -1))

; catch block followed by catch/finally, reduce indent, but don't force a newline
((catch_clause
  body: (block
    "}" @node .))
  .
  [
    (catch_clause)
    (finally_clause)
  ]
  (#set! "format.indent.before" -1))

; empty block, allowed shorthand form
((block
  "{" @node
  .
  "}")
  (#set! "format.indent.after" 1)
  (#set! "format.space.before" true))

((block
  "{"
  .
  "}" @node)
  (#set! "format.indent.before" -1)
  (#set! "format.newline.after" 1))

; empty block, allowed shorthand form
((constructor_body
  "{" @node
  .
  "}")
  (#set! "format.indent.after" 1)
  (#set! "format.space.before" true))

((constructor_body
  "{"
  .
  "}" @node)
  (#set! "format.indent.before" -1)
  (#set! "format.newline.after" 1))

; empty block, allowed shorthand form
((class_body
  "{" @node
  .
  "}")
  (#set! "format.indent.after" 1)
  (#set! "format.space.before" true))

((class_body
  "{"
  .
  "}" @node)
  (#set! "format.indent.before" -1)
  (#set! "format.newline.after" 1))

; no newline after lambda close
((lambda_expression
  (block
    "}" @node))
  (#set! "format.indent.before" -1))

; nor object creation expression
((object_creation_expression
  (class_body
    "}" @node))
  (#set! "format.indent.before" -1))

; open block: increase indent
("{" @node
  (#set! "format.indent.after" 1)
  (#set! "format.newline.after" 1)
  (#set! "format.space.before" true))

; close block: reduce indent
("}" @node
  (#set! "format.indent.before" -1)
  (#set! "format.newline.before" 1)
  (#set! "format.newline.after" 1))

; end of package decl before another node, extra blank line
((package_declaration
  ";" @node)
  .
  (_)
  (#set! "format.newline.after" 2))

; keep on the same line if possible
((for_statement
  ";" @node)
  (#set! "format.space.after" true))

; keep on the same line if possible
((for_statement
  init: (local_variable_declaration
    ";" @node .))
  (#set! "format.space.after" true))

; end of statement, newline
(";" @node
  (#set! "format.newline.after" 1))

; marker annotation, newline
((marker_annotation
  name: (scoped_identifier
    name: (_) @node))
  (#set! "format.newline.after" 1))

; marker annotation, newline
((marker_annotation
  name: (_) @node)
  (#set! "format.newline.after" 1))

; regular annotation, newline
((annotation_argument_list
  ")" @node .)
  (#set! "format.newline.after" 1))

; newlines after enum constants
(enum_body
  "," @node
  (#set! "format.newline.after" 1))

; space after comma otherwise
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

; space between type and name
((resource
  name: (_) @node)
  (#set! "format.space.before" true))

; space between type and name
((receiver_parameter
  (type_identifier) @node)
  (#set! "format.space.after" true))

; space between type and name
((receiver_parameter
  (scoped_type_identifier
    (type_identifier) @node))
  (#set! "format.space.after" true))

; JEP 394
((instanceof_expression
  name: (_) @node)
  (#set! "format.space.before" true))

; JEP 440
((record_pattern_component
  (_) @node .)
  (#set! "format.space.before" true))

; JEP 441
((type_pattern
  (_) @node .)
  (#set! "format.space.before" true))

; no space between @interface
((annotation_type_declaration
  "@"
  "interface" @node)
  (#set! "format.space.after" true))

; no space before new
("new" @node
  (#set! "format.space.after" true))

; no space after single-statement forms
((break_statement
  "break" @node
  .
  ";")
  (#set! "format.space.before" true))

; no space after single-statement forms
((continue_statement
  "continue" @node
  .
  ";")
  (#set! "format.space.before" true))

; no space after single-statement forms
((return_statement
  "return" @node
  .
  ";")
  (#set! "format.space.before" true))

; space after modifiers
((modifier
  _ @node)
  (#set! "format.space.after" true))

; nothing special
(class_literal
  "class" @node)

; nothing special
(asterisk
  "*" @node)

((wildcard
  [
    (annotation)
    (marker_annotation)
  ]
  "?" @node)
  (#set! "format.space.before" true))

; nothing special
(wildcard
  "?" @node)

; must insert space to disambiguate from an update expression
((unary_expression
  operator: _ @node
  operand: [
    (unary_expression)
    (update_expression)
  ])
  (#set! "format.space.after" true))

; nothing special
(unary_expression
  operator: _ @node)

; nothing special
(type_arguments
  [
    "<"
    ">"
  ] @node)

; nothing special
(type_parameters
  "<" @node)

; space after
((type_parameters
  ">" @node)
  (#set! "format.space.after" true))

; fallthrough switch
((switch_block_statement_group
  ":" @node .)
  (#set! "format.newline.after" 1))

; no space before : in a switch
((switch_block_statement_group
  ":" @node)
  (#set! "format.space.after" true))

; no space after default label
(switch_label
  "default" @node)

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
  "?"
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

((annotation_type_element_declaration
  name: (identifier) @node)
  (#set! "format.space.before" true))

((cast_expression
  ")" @node)
  (#set! "format.space.after" true))

((wildcard
  (super) @node)
  (#set! "format.space.before" true)
  (#set! "format.space.after" true))

; insane corner case, treesitter allows it with a space
; but not with, so don't remove the space :)
((field_access
  object: [
    (decimal_integer_literal)
    (hex_integer_literal)
    (octal_integer_literal)
    (binary_integer_literal)
  ] @node)
  (#set! "format.space.after" true))

; comments: indent them, newline them (for now)
([
  (line_comment)
  (block_comment)
] @node
  (#set! "format.newline.after" 1))

_ @node
