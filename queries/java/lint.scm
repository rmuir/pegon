; P0000: hard parsing error
((ERROR) @error
  (#set! name "syntax-error")
  (#set! title "Syntax Error")
  (#set! label "parse error here"))

; P0001: soft parsing error
((MISSING) @error
  (#set! name "missing-syntax")
  (#set! title "Missing syntax element")
  (#set! label "missing element here"))

; P0002: special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  ; BS, TAB, NL
  (#any-of? @error "\\u0008" "\\010" "\\10" "\\u0009" "\\011" "\\11" "\\u000A" "\\012" "\\12")
  (#set! name "raw-special-escape")
  (#set! title "Raw special escape sequence in octal/hex form")
  (#set! label "raw escape here"))

; P0003: wildcard imports
; https://google.github.io/styleguide/javaguide.html#s3.3.1-wildcard-imports
((import_declaration
  (asterisk) @error)
  (#set! name "wildcard-import")
  (#set! title "Do not use wildcard imports")
  (#set! label "wildcard used here"))
