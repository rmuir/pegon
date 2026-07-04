; license
((program
  .
  [
    (block_comment)
    (line_comment)
  ]+ @range)
  (#set! fold.kind "comment")
  (#set! fold.lineoffset 1))

; adjacent imports
((import_declaration)+ @range
  (#set! fold.kind "imports"))

; multiline javadoc comments: /** */
; summarize with first real line
((block_comment) @range
  (#match? @range "^/[*][*][\\s]*[\n].")
  (#set! fold.kind "comment")
  (#set! fold.lineoffset 1))

; markdown javadoc comment blocks
((line_comment)+ @range
  (#match? @range "^///")
  (#not-match? @range "^///[^\\s]")
  (#set! fold.kind "comment"))

; regions
; function-like bodies
((constructor_body) @range
  (#match? @range "[\\n]")
  (#set! fold.kind "region"))

((compact_constructor_declaration
  body: (block) @range)
  (#match? @range "[\\n]")
  (#set! fold.kind "region"))

((method_declaration
  body: (block) @range)
  (#match? @range "[\\n]")
  (#set! fold.kind "region"))

((static_initializer
  (block) @range)
  (#match? @range "[\\n]")
  (#set! fold.kind "region"))
