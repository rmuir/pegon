; adjacent imports
((import_declaration)+ @range
  (#set! kind "imports"))

; multiline javadoc /** comments */
((block_comment) @range
  (#match? @range "^/[*][*][\\s]*[\n].")
  (#set! kind "comment")
  (#set! lineoffset "1"))

; other block comments
((block_comment) @range
  (#not-match? @range "^/[*][*][\\s]*[\n].")
  (#set! kind "comment"))

; // comments
((line_comment)+ @range
  (#set! kind "comment"))

; class/function bodies
(class_body) @range

(annotation_type_body) @range

(enum_body) @range

(interface_body) @range

(class_body) @range

(constructor_body) @range

(compact_constructor_declaration
  body: (block) @range)

(method_declaration
  body: (block) @range)
