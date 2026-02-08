;; P0000: hard parsing error
((ERROR) @error
 (#set! name "syntax-error")
 (#set! title "Syntax Error")
 (#set! label "parse error here"))

;; P0001: soft parsing error
((MISSING) @error
 (#set! name "missing-syntax")
 (#set! title "Missing syntax element")
 (#set! label "missing element here"))

;; P0002: wildcard imports
;; https://google.github.io/styleguide/javaguide.html#s3.3.1-wildcard-imports
((import_declaration
  (asterisk) @error)
 (#set! name "wildcard-import")
 (#set! title "Do not use wildcard imports")
 (#set! label "wildcard used here"))
