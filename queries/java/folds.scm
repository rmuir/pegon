; adjacent imports
((import_declaration)+
  (#set! kind "imports")) @range

; /** comments */
((block_comment)
  (#set! kind "comment")) @range

; // comments
((line_comment)+
  (#set! kind "comment")) @range

; class-like things
(object_creation_expression
  (class_body)) @range

(annotation_type_declaration) @range

(class_declaration) @range

(enum_declaration) @range

(interface_declaration) @range

(record_declaration) @range

; function-like things
(compact_constructor_declaration) @range

(constructor_declaration) @range

(enum_constant
  body: (_)) @range

(method_declaration) @range
