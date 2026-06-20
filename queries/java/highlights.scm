; hover over "try" to highlight catch
((try_statement
  "try" @range @reference
  [
    (catch_clause
      "catch" @range @reference)
    (finally_clause
      "finally" @range @reference)
  ]*)
  (#set! highlight.kind 2))

((try_with_resources_statement
  "try" @range @reference
  [
    (catch_clause
      "catch" @range @reference)
    (finally_clause
      "finally" @range @reference)
  ]*)
  (#set! highlight.kind 2))
