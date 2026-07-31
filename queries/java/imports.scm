; Each pattern represents an import "group" in order
; Some of these groups aren't allowed in the google style but we give them an order anyway
; The group is the primary sort, and the @text is the secondary sort
; ---
; module imports first
(program
  (import_declaration
    "module"
    [
      (identifier)
      (scoped_identifier)
    ] @text) @node)

; static imports next
(program
  (import_declaration
    "static"
    [
      (identifier)
      (scoped_identifier)
    ] @text
    (asterisk)?) @node)

; regular imports
((program
  (import_declaration
    [
      (identifier)
      (scoped_identifier)
    ] @text
    (asterisk)?) @node)
  (#not-match? @node "^import\\s+static"))
