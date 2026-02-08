((ERROR) @error
 (#set! name "syntax-error"))

((MISSING) @error
 (#set! name "syntax-missing"))

((import_declaration
  (asterisk) @error)
 (#set! name "wildcard-import"))
