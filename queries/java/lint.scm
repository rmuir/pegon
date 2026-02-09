; TS parsing error
([(ERROR)(MISSING)] @error
  (#set! name "parse-error")
  (#set! severity "warning")
  (#set! title "Parse Error")
  (#set! label "Parse error here")
  (#set! help "Correct the invalid java syntax"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0008" "\\010" "\\10")
  (#set! severity "warning")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape here")
  (#set! help "Replace the raw escape with \\b")
  (#set! fix "\\b"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0009" "\\011" "\\11")
  (#set! severity "warning")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape here")
  (#set! help "Replace the raw escape with \\t")
  (#set! fix "\\t"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000a" "\\u000A" "\\012" "\\12")
  (#set! severity "warning")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape here")
  (#set! help "Replace the raw escape with \\n")
  (#set! fix "\\n"))

; wildcard imports
; https://google.github.io/styleguide/javaguide.html#s3.3.1-wildcard-imports
((import_declaration
  (asterisk) @error)
  (#set! severity "warning")
  (#set! name "wildcard-import")
  (#set! title "Do not use wildcard imports")
  (#set! label "Wildcard used here")
  (#set! help "Replace the wildcard import with standard import(s)"))
