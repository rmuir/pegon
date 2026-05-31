; adjacent imports
((import_declaration)+ @range
  (#set! fold.kind "imports"))

; multiline block /* comments */ or /** comments */
((block_comment) @range
  (#match? @range "^/[*][*]?[\\s]*[\n].")
  (#set! fold.kind "comment")
  (#set! fold.lineoffset 1))

; other block comments
((block_comment) @range
  (#not-match? @range "^/[*][*]?[\\s]*[\n].")
  (#set! fold.kind "comment"))

; // comments
((line_comment)+ @range
  (#set! fold.kind "comment"))

; function-like bodies
(constructor_body) @range

(compact_constructor_declaration
  body: (block) @range)

(method_declaration
  body: (block) @range)

(static_initializer
  (block) @range)
