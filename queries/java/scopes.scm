; fields can be declared at the very bottom of the file and accessed anywhere from inside class's body
((class_body
  (field_declaration
    declarator: (variable_declarator
      name: (identifier) @variable))) @start @end
  (#set! scope.kind "property")
  (#set! scope.start.inclusive true))

; local variables can only be accessed after they are declared. they may shadow fields
; TODO: there are more cases than this like single-statement situations, deal with them
((block
  (local_variable_declaration
    declarator: (variable_declarator
      name: (identifier) @variable)) @start
  "}" @end)
  (#set! scope.kind "variable"))

; parameters can be accessed
