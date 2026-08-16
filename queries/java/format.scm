; open block: increase indent
((block
  "{" @node)
  (#set! "format.indent.delta" 1)
  (#set! "format.newline.after" true))

; close block: reduce indent
((block
  "}" @node)
  (#set! "format.indent.delta" -1)
  (#set! "format.newline.after" true))

; comments: indent them, newline them (for now)
([
  (line_comment)
  (block_comment)
] @node
  (#set! "format.indent.before" true)
  (#set! "format.newline.after" true))
