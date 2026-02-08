((ERROR) @error
 (#set! name "syntax-error")
 (#set! title "Syntax Error")
 (#set! label "parse error here"))

((MISSING) @error
 (#set! name "syntax-missing")
 (#set! title "Syntax Missing")
 (#set! label "missing element here"))

((import_declaration
  (asterisk) @error)
 (#set! name "wildcard-import")
 (#set! title "Do not use wildcard imports")
 (#set! label "wildcard used here"))
