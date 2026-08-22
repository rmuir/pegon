; Formatting patterns work a bit different than the others.
; * only terminal nodes are captured (things that are written)
; * first pattern to match "wins" for the node.
; * the last pattern in this file matches any terminal node and simply writes it.
;
; The idea is to encode precisely where indents increase/decrease,
; where spaces are needed, where newlines are needed.
;
; Spaces and newlines are merged automatically. So if there's AB, and A requires
; a space after it, and B requires a space before it, there will only be one space.
;
; Comments have to be treated a bit special, to handle trailing comments, and even
; "medial form" comments /* like this */ that sometimes happen.
; ---
; if-with-else, reduce indent, but don't force a newline
((if_statement
  consequence: (block
    "}" @node .)
  .
  "else")
  (#set! format.indent.before -1))

; do block end followed by while, reduce indent, but don't force a newline
((do_statement
  body: (block
    "}" @node .)) ; "while" is implicitly required by the grammar
  (#set! format.indent.before -1))

; enum constant } followed by comma, reduce indent, but don't force a newline
((enum_body
  (enum_constant
    body: (class_body
      "}" @node .))
  .
  ",")
  (#set! format.indent.before -1))

; try block followed by catch/finally, reduce indent, but don't force a newline
((try_statement
  body: (block
    "}" @node .)
  .
  [
    (catch_clause)
    (finally_clause)
  ])
  (#set! format.indent.before -1))

; try-with-resources block followed by catch/finally, reduce indent, but don't force a newline
((try_with_resources_statement
  body: (block
    "}" @node .)
  .
  [
    (catch_clause)
    (finally_clause)
  ])
  (#set! format.indent.before -1))

; catch block followed by catch/finally, reduce indent, but don't force a newline
((catch_clause
  body: (block
    "}" @node .))
  .
  [
    (catch_clause)
    (finally_clause)
  ]
  (#set! format.indent.before -1))

; empty block open, allowed shorthand form
((block
  "{" @node
  .
  "}")
  (#set! format.indent.after 1)
  (#set! format.space.before true))

; empty block close, allowed shorthand form
((block
  "{"
  .
  "}" @node)
  (#set! format.indent.before -1)
  (#set! format.newline.after true))

; empty block open, allowed shorthand form
((constructor_body
  "{" @node
  .
  "}")
  (#set! format.indent.after 1)
  (#set! format.space.before true))

; empty block close, allowed shorthand form
((constructor_body
  "{"
  .
  "}" @node)
  (#set! format.indent.before -1)
  (#set! format.newline.after true))

; empty block open, allowed shorthand form
((class_body
  "{" @node
  .
  "}")
  (#set! format.indent.after 1)
  (#set! format.space.before true))

; empty block close, allowed shorthand form
((class_body
  "{"
  .
  "}" @node)
  (#set! format.indent.before -1)
  (#set! format.newline.after true))

; annotation array init open, no spaces, nothing
(element_value_array_initializer
  "{" @node)

; annotation array init close, no spaces, nothing
(element_value_array_initializer
  "}" @node)

; array init open, no spaces, nothing
(array_initializer
  "{" @node)

; array init close, no spaces, nothing
(array_initializer
  "}" @node)

; no newline after lambda close
((lambda_expression
  (block
    "}" @node))
  (#set! format.indent.before -1))

; nor object creation expression
((object_creation_expression
  (class_body
    "}" @node))
  (#set! format.indent.before -1))

; extra switch block indentation
((switch_block
  "{" @node
  (switch_block_statement_group)+)
  (#set! format.indent.after 2)
  (#set! format.newline.after true)
  (#set! format.space.before true))

; extra switch block de-indentation
((switch_block
  (switch_block_statement_group)+
  "}" @node)
  (#set! format.indent.before -2)
  (#set! format.newline.after true)
  (#set! format.space.before true))

; open block: increase indent
("{" @node
  (#set! format.indent.after 1)
  (#set! format.newline.after true)
  (#set! format.space.before true))

; close block: reduce indent
("}" @node
  (#set! format.indent.before -1)
  (#set! format.newline.before true)
  (#set! format.newline.after true))

; keep on the same line if possible
((for_statement
  ";" @node)
  (#set! format.space.after true))

; keep on the same line if possible
((for_statement
  init: (local_variable_declaration
    ";" @node .))
  (#set! format.space.after true))

; marker annotation, newline
((marker_annotation
  name: (scoped_identifier
    name: (_) @node))
  (#set! format.newline.after true))

; marker annotation, newline
((marker_annotation
  name: (_) @node)
  (#terminal? @node)
  (#set! format.newline.after true))

; regular annotation, newline
((annotation_argument_list
  ")" @node .)
  (#set! format.newline.after true))

; newlines after enum constants
(enum_body
  "," @node
  (#set! format.newline.after true))

; space between type and the name
((formal_parameter
  name: (_) @node)
  (#set! format.space.before true))

; space between type and name
((catch_formal_parameter
  name: (_) @node)
  (#set! format.space.before true))

; space between type and name
((variable_declarator
  name: (_) @node)
  (#set! format.space.before true))

; space between type and name
((enhanced_for_statement
  name: (_) @node)
  (#set! format.space.before true))

; space between type and name
((resource
  name: (_) @node)
  (#set! format.space.before true))

; space between type and name
((receiver_parameter
  (type_identifier) @node)
  (#set! format.space.after true))

; space between type and name
((receiver_parameter
  (scoped_type_identifier
    (type_identifier) @node))
  (#set! format.space.after true))

; JEP 394
((instanceof_expression
  name: (_) @node)
  (#set! format.space.before true))

; JEP 440
((record_pattern_component
  (_) @node .)
  (#set! format.space.before true))

; JEP 441
((type_pattern
  (_) @node .)
  (#set! format.space.before true))

; no space between @interface
((annotation_type_declaration
  "@"
  "interface" @node)
  (#set! format.space.after true))

; no space after "new" when used as method reference
(method_reference
  "new" @node)

; no space before new
("new" @node
  (#set! format.space.after true))

; no space after single-statement forms
((break_statement
  "break" @node
  .
  ";")
  (#set! format.space.before true))

; no space after single-statement forms
((continue_statement
  "continue" @node
  .
  ";")
  (#set! format.space.before true))

; no space after single-statement forms
((return_statement
  "return" @node
  .
  ";")
  (#set! format.space.before true))

; space after modifiers
((modifier
  _ @node)
  (#set! format.space.after true))

; nothing special
(class_literal
  "class" @node)

; nothing special
(asterisk
  "*" @node)

; very rare, but possible annotation before
((wildcard
  [
    (annotation)
    (marker_annotation)
  ]
  "?" @node)
  (#set! format.space.before true))

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
  (#set! format.space.after true))

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
  (#set! format.space.after true))

; indent after the :
((switch_block_statement_group
  ":" @node)
  (#set! format.indent.after 1)
  (#set! format.newline.after true))

; de-indent on keyword
((switch_block_statement_group
  (switch_label
    "default" @node))
  (#set! format.indent.before -1))

; de-indent on keyword
((switch_block_statement_group
  (switch_label
    "case" @node))
  (#set! format.indent.before -1)
  (#set! format.space.after true))

; TODO: i don't like this, but its what google does i guess?
; space after, instead, makes for more readable code
((labeled_statement
  ":" @node)
  (#set! format.newline.after true))

; google inserts a newline here always, and indents
([
  (opens_module_directive
    "to" @node)
  (exports_module_directive
    "to" @node)
  (provides_module_directive
    "with" @node)
]
  (#set! format.indent.after 2)
  (#set! format.space.before true)
  (#set! format.newline.after true))

; newline after any ,
([
  (opens_module_directive
    "," @node)
  (exports_module_directive
    "," @node)
  (provides_module_directive
    "," @node)
]
  (#set! format.newline.after true))

; dedent after the ;
([
  (opens_module_directive
    "to"
    ";" @node)
  (exports_module_directive
    "to"
    ";" @node)
  (provides_module_directive
    "with"
    ";" @node)
]
  (#set! format.indent.after -2)
  (#set! format.newline.after true))

; end of statement, newline
(";" @node
  (#set! format.newline.after true))

; space after comma otherwise
("," @node
  (#set! format.space.after true))

; put spaces around this big pile of keywords and operators
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
  (#set! format.space.before true)
  (#set! format.space.after true))

; space between type and name
((method_declaration
  name: (identifier) @node)
  (#set! format.space.before true))

; space between type and name
((annotation_type_element_declaration
  name: (identifier) @node)
  (#set! format.space.before true))

; space after if condition
((if_statement
  condition: (parenthesized_expression
    ")" @node))
  (#set! format.space.after true))

; space after loop condition
((while_statement
  condition: (parenthesized_expression
    ")" @node))
  (#set! format.space.after true))

; space after loop condition
((for_statement
  ")" @node)
  (#set! format.space.after true))

; space after loop condition
((enhanced_for_statement
  ")" @node)
  (#set! format.space.after true))

; space after cast
((cast_expression
  ")" @node)
  (#set! format.space.after true))

; space after array init's ]
((array_creation_expression
  dimensions: (dimensions
    "]" @node)
  value: (array_initializer))
  (#set! format.space.after true))

; treat super as a keyword inside bounds
((wildcard
  (super) @node)
  (#set! format.space.before true)
  (#set! format.space.after true))

; insane corner case, treesitter allows it with a space
; but not with, so don't remove the space :)
((field_access
  object: [
    (decimal_integer_literal)
    (hex_integer_literal)
    (octal_integer_literal)
    (binary_integer_literal)
  ] @node)
  (#set! format.space.after true))

; insane corner case, due to the nature of extras
; don't treat these special in any way
(string_literal
  [
    (line_comment)
    (block_comment)
  ] @node)

; comments inside a switch block: de-indent then reindent
((switch_block
  [
    (line_comment)
    (block_comment)
  ] @node
  (switch_block_statement_group)+)
  (#set! format.indent.before -1)
  (#set! format.indent.after 1)
  (#set! format.comment true)
  (#set! format.newline.after true)
  (#set! format.space.before true))

; comments: indent them, newline them (for now)
([
  (line_comment)
  (block_comment)
] @node
  (#set! format.comment true)
  (#set! format.newline.after true)
  (#set! format.space.before true))

; any otherwise listed terminal node
; this could be avoided, but it is convenient to ensure everything is written.
; format-ignore
([_] @node
  (#terminal? @node))
