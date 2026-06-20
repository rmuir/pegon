; highlight switch/case/default
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

; highlight try/catch/finally
((try_statement
  "try" @range @reference
  [
    (catch_clause
      "catch" @range @reference)
    (finally_clause
      "finally" @range @reference)
  ]*)
  (#set! highlight.kind 2))

; highlight try/catch/finally
((try_with_resources_statement
  "try" @range @reference
  [
    (catch_clause
      "catch" @range @reference)
    (finally_clause
      "finally" @range @reference)
  ]*)
  (#set! highlight.kind 2))
