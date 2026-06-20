; if-else
((if_statement
  "if" @range @reference
  "else"? @range @reference)
  (#set! highlight.kind 2))

; do-while
((do_statement
  "do" @range @reference
  "while" @range @reference)
  (#set! highlight.kind 2))

; switch/case/default
((switch_expression
  "switch" @range @reference
  body: (switch_block
    [
      (switch_block_statement_group
        (switch_label
          [
            "case"
            "default"
          ] @range @reference))
      (switch_rule
        (switch_label
          [
            "case"
            "default"
          ] @range @reference))
    ]*))
  (#set! highlight.kind 2))

; try/catch/finally
((try_statement
  "try" @range @reference
  [
    (catch_clause
      "catch" @range @reference)
    (finally_clause
      "finally" @range @reference)
  ]*)
  (#set! highlight.kind 2))

; try/catch/finally
((try_with_resources_statement
  "try" @range @reference
  [
    (catch_clause
      "catch" @range @reference)
    (finally_clause
      "finally" @range @reference)
  ]*)
  (#set! highlight.kind 2))
