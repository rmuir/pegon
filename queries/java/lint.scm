; TS parsing error
([(ERROR)(MISSING)] @error
  (#set! name "parse-error")
  (#set! severity "warning")
  (#set! title "Parse Error")
  (#set! label "parse error here"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  ; BS, TAB, NL
  (#any-of? @error "\\u0008" "\\010" "\\10" "\\u0009" "\\011" "\\11" "\\u000A" "\\012" "\\12")
  (#set! severity "warning")
  (#set! name "raw-special-escape")
  (#set! title "Raw special escape sequence in octal/hex form")
  (#set! label "raw escape here"))

; wildcard imports
; https://google.github.io/styleguide/javaguide.html#s3.3.1-wildcard-imports
((import_declaration
  (asterisk) @error)
  (#set! severity "warning")
  (#set! name "wildcard-import")
  (#set! title "Do not use wildcard imports")
  (#set! label "wildcard used here"))
