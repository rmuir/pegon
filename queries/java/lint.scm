; TS parsing error
([(ERROR)(MISSING)] @error
  (#set! name "parse-error")
  (#set! severity "warning")
  (#set! title "Parse Error")
  (#set! label "Parse error here")
  (#set! help "Correct the invalid Java syntax"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0008" "\\010" "\\10")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with \\b")
  (#set! fix "\\b"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0009" "\\011" "\\11")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with \\t")
  (#set! fix "\\t"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000a" "\\u000A" "\\012" "\\12")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with \\n")
  (#set! fix "\\n"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000c" "\\u000C" "\\014" "\\14")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with \\f")
  (#set! fix "\\f"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u000d" "\\u000D" "\\015" "\\15")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with \\r")
  (#set! fix "\\r"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0022" "\\042" "\\42")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with \\\"")
  (#set! fix "\\\""))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u0027" "\\047" "\\47")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with \\'")
  (#set! fix "\\'"))

; special escape sequences encoded as octal/hex
; https://google.github.io/styleguide/javaguide.html#s2.3.2-special-escape-sequences
((escape_sequence) @error
  (#any-of? @error "\\u005c" "\\u005C" "\\134")
  (#set! severity "error")
  (#set! name "raw-special-escape")
  (#set! title "Special escape sequence in octal/hex form")
  (#set! label "Raw escape used here")
  (#set! help "Replace the raw escape with \\\\")
  (#set! fix "\\\\"))

; wildcard imports
; https://google.github.io/styleguide/javaguide.html#s3.3.1-wildcard-imports
((import_declaration
  (asterisk) @error)
  (#set! severity "error")
  (#set! name "wildcard-import")
  (#set! title "Do not use wildcard imports")
  (#set! label "Wildcard used here")
  (#set! help "Replace the wildcard import with standard import(s)"))
