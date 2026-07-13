; always a keyword
([
  "case"
  "catch"
  "class"
  "continue"
  "default"
  "do"
  "else"
  "enum"
  "extends"
  "final"
  "finally"
  "for"
  "if"
  "implements"
  "import"
  "interface"
  "native"
  "new"
  "package"
  "private"
  "protected"
  "public"
  "return"
  "short"
  "static"
  "strictfp"
  (super)
  "switch"
  "synchronized"
  (this)
  "throw"
  "throws"
  "transient"
  (void_type)
  "volatile"
  "while"
] @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "reserved keyword")
  (#set! hover.spec.description "You can't name a variable with this word.")
  (#set! hover.spec.reference "3.9"))

; sometimes a keyword
([
  "exports"
  "module"
  "non-sealed"
  "open"
  "opens"
  "permits"
  "provides"
  "record"
  "requires"
  "sealed"
  "to"
  "transitive"
  "uses"
  ((type_identifier) @_type
    (#eq? @_type "var"))
  "when"
  "with"
  "yield"
] @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "contextual keyword")
  (#set! hover.spec.description "You probably shouldn't name a variable with this word.")
  (#set! hover.spec.reference "3.9"))

; abstract keyword on a class
((class_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "abstract class modifier")
  (#set! hover.spec.description "This class isn't concrete: only subclasses can be instantiated.")
  (#set! hover.spec.reference "8.1.1.1"))

; abstract keyword on a method
((method_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "abstract method modifier")
  (#set! hover.spec.description "This method isn't concrete: a subclass must implement it.")
  (#set! hover.spec.reference "8.4.3.1"))

; abstract keyword on an interface
; TODO: add a diagnostic?
((interface_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "abstract interface modifier")
  (#set! hover.spec.description "All interfaces are abstract: remove this keyword.")
  (#set! hover.spec.reference "9.1.1.1"))

; assertion keyword
("assert" @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "assert statement")
  (#set! hover.spec.description "Throws `AssertionError` on failure if assertions are enabled.")
  (#set! hover.spec.reference "14.10"))

; boolean keyword
((boolean_type) @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "boolean type")
  (#set! hover.spec.description "Boolean type: `true` or `false`")
  (#set! hover.spec.reference "4.2.5"))

; boolean used as a field
((field_declaration
  type: (boolean_type) @range)
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "boolean field")
  (#set! hover.spec.description
    "Boolean type: `true` or `false`. Probably uses a byte of space here.")
  (#set! hover.spec.reference "4.2.5"))

; byte
("byte" @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "byte type")
  (#set! hover.spec.description "8-bit signed integer: -128 .. 127.")
  (#set! hover.spec.reference "4.2.1"))

; char
("char" @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "char type")
  (#set! hover.spec.description "16-bit unsigned integer: 0 .. 65,535 (\\u0000-\\uFFFF).")
  (#set! hover.spec.reference "4.2.1"))

; short
("short" @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "short type")
  (#set! hover.spec.description "16-bit signed integer: -32,768 .. 32,767.")
  (#set! hover.spec.reference "4.2.1"))

; int
("int" @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "integer type")
  (#set! hover.spec.description "32-bit signed integer: -2,147,483,648 .. 2,147,483,647.")
  (#set! hover.spec.reference "4.2.1"))

; long
("long" @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "long integer type")
  (#set! hover.spec.description
    "64-bit signed integer: -9,223,372,036,854,775,808 .. 9,223,372,036,854,775,807.")
  (#set! hover.spec.reference "4.2.1"))

; float
("float" @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "single-precision floating-point type")
  (#set! hover.spec.description "32-bit IEEE binary32 float: 8-bit exponent.")
  (#set! hover.spec.reference "4.2.3"))

; float
("double" @range
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "double-precision floating-point type")
  (#set! hover.spec.description "64-bit IEEE binary64 float: 11-bit exponent.")
  (#set! hover.spec.reference "4.2.3"))

; instanceof
((instanceof_expression
  "instanceof" @range
  right: (_))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "type comparison operator")
  (#set! hover.spec.description "True if expression is non-null and compatible.")
  (#set! hover.spec.reference "15.20.2"))

; instanceof (non-record pattern)
((instanceof_expression
  "instanceof" @range
  name: (identifier))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "pattern match operator")
  (#set! hover.spec.description "True if expression is non-null and matches.")
  (#set! hover.spec.reference "15.20.2"))

; instanceof (record pattern)
((instanceof_expression
  "instanceof" @range
  pattern: (record_pattern))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "pattern match operator")
  (#set! hover.spec.description "True if expression is non-null and matches.")
  (#set! hover.spec.reference "15.20.2"))

; break (no label)
((break_statement
  "break" @range)
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "break statement")
  (#set! hover.spec.description "Breaks out of switch or loop.")
  (#set! hover.spec.reference "14.15"))

; break to label
((break_statement
  "break" @range
  (identifier))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "break statement")
  (#set! hover.spec.description "Breaks to target: it is a `goto`.")
  (#set! hover.spec.reference "14.15"))

; try-catch
((try_statement
  "try" @range)
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "try-catch statement")
  (#set! hover.spec.description "Try statement with `catch` block for exception handling.")
  (#set! hover.spec.reference "14.20.1"))

; try with a finally clause
((try_statement
  "try" @range
  (finally_clause))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "try-finally statement")
  (#set! hover.spec.description "Try statement with a `finally` block that is always executed.")
  (#set! hover.spec.reference "14.20.2"))

; try with catches and a finally clause
((try_statement
  "try" @range
  (catch_clause)+
  (finally_clause))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "try-catch-finally statement")
  (#set! hover.spec.description
    "Try statement with `catch` blocks for exception handling and a `finally` block that is always executed.")
  (#set! hover.spec.reference "14.20.2"))

; basic try-with-resources
((try_with_resources_statement
  "try" @range)
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "try-with-resources statement")
  (#set! hover.spec.description "Try statement with automatic resource closure.")
  (#set! hover.spec.reference "14.20.3.1"))

; extended try-with-resources with finally
((try_with_resources_statement
  "try" @range
  (finally_clause))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "extended try-with-resources statement")
  (#set! hover.spec.description
    "Try statement with automatic resource closure and a `finally` block that is always executed.")
  (#set! hover.spec.reference "14.20.3.2"))

; extended try-with-resources with catch
((try_with_resources_statement
  "try" @range
  (catch_clause)+)
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "extended try-with-resources statement")
  (#set! hover.spec.description
    "Try statement with automatic resource closure and `catch` blocks for resource exception handling.")
  (#set! hover.spec.reference "14.20.3.2"))

; extended try-with-resources with catch and finally
((try_with_resources_statement
  "try" @range
  (catch_clause)+
  (finally_clause))
  (#set! hover.kind "spec")
  (#set! hover.spec.summary "extended try-with-resources statement")
  (#set! hover.spec.description
    "Try statement with automatic resource closure, `catch` blocks for resource exception handling, and a `finally` block that is always executed.")
  (#set! hover.spec.reference "14.20.3.2"))

; identifiers
([
  (identifier)
  (type_identifier)
] @range
  (#set! hover.kind "reference"))

; but not these yet
((field_access
  field: (identifier) @range)
  (#set! hover.kind "bail"))
