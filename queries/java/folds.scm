; Folds the three main types in the LSP standard.
;   * imports: blocks of import statements
;   * comment: folds boilerplate comments such as license headers and javadocs.
;   * region: method bodies and similar
;
; The folding is intended towards developers that really use folding.
; It can reduce the boilerplate of java substantially:
; For production java files, the first entire page of "code" is useless.
;
; If you follow the google style, your license headers will be folded.
; Blocks of import statements will be folded to a single line.
; Multi-line javadoc comments will be folded such that the first line is visible.
; This also works for markdown-style javadoc comments.
; ---
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
