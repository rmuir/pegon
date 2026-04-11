; adjacent imports
((import_declaration)+
  (#set! kind "imports")) @range

; /** comments */
((block_comment)
  (#set! kind "comment")) @range

; // comments
((line_comment)+
  (#set! kind "comment")) @range

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
