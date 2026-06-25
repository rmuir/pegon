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
  (#set! hover.description "You can't name a variable with this word.")
  (#set! hover.kind "reserved keyword")
  (#set! hover.spec "3.9"))

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
  (#set! hover.description "You probably shouldn't name a variable with this word.")
  (#set! hover.kind "contextual keyword")
  (#set! hover.spec "3.9"))

; abstract keyword on a class
((class_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.description "This class isn't concrete: only subclasses can be instantiated.")
  (#set! hover.kind "abstract class modifier")
  (#set! hover.spec "8.1.1.1"))

; abstract keyword on a method
((method_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.description "This method isn't concrete: a subclass must implement it.")
  (#set! hover.kind "abstract method modifier")
  (#set! hover.spec "8.4.3.1"))

; abstract keyword on an interface
; TODO: add a diagnostic?
((interface_declaration
  (modifiers
    (modifier
      "abstract") @range))
  (#set! hover.description "All interfaces are abstract: remove this keyword.")
  (#set! hover.kind "abstract interface modifier")
  (#set! hover.spec "9.1.1.1"))

; assertion keyword
("assert" @range
  (#set! hover.description "Throws `AssertionError` on failure if assertions are enabled.")
  (#set! hover.kind "assert statement")
  (#set! hover.spec "14.10"))

; boolean keyword
((boolean_type) @range
  (#set! hover.description "Boolean type: `true` or `false`")
  (#set! hover.kind "boolean type")
  (#set! hover.spec "4.2.5"))

; boolean used as a field
((field_declaration
  type: (boolean_type) @range)
  (#set! hover.description "Boolean type: `true` or `false`. Probably uses a byte of space here.")
  (#set! hover.kind "boolean field")
  (#set! hover.spec "4.2.5"))

; byte
("byte" @range
  (#set! hover.description "8-bit signed integer: -128 .. 127.")
  (#set! hover.kind "byte type")
  (#set! hover.spec "4.2.1"))

; char
("char" @range
  (#set! hover.description "16-bit unsigned integer: 0 .. 65,535 (\\u0000-\\uFFFF).")
  (#set! hover.kind "char type")
  (#set! hover.spec "4.2.1"))

; short
("short" @range
  (#set! hover.description "16-bit signed integer: -32,768 .. 32,767.")
  (#set! hover.kind "short type")
  (#set! hover.spec "4.2.1"))

; int
("int" @range
  (#set! hover.description "32-bit signed integer: -2,147,483,648 .. 2,147,483,647.")
  (#set! hover.kind "integer type")
  (#set! hover.spec "4.2.1"))

; long
("long" @range
  (#set! hover.description
    "64-bit signed integer: -9,223,372,036,854,775,808 .. 9,223,372,036,854,775,807.")
  (#set! hover.kind "long integer type")
  (#set! hover.spec "4.2.1"))

; float
("float" @range
  (#set! hover.description "32-bit IEEE binary32 float: 8-bit exponent.")
  (#set! hover.kind "single-precision floating-point type")
  (#set! hover.spec "4.2.3"))

; float
("double" @range
  (#set! hover.description "64-bit IEEE binary64 float: 11-bit exponent.")
  (#set! hover.kind "double-precision floating-point type")
  (#set! hover.spec "4.2.3"))

; instanceof
((instanceof_expression
  "instanceof" @range
  right: (_))
  (#set! hover.description "True if expression is non-null and compatible.")
  (#set! hover.kind "type comparison operator")
  (#set! hover.spec "15.20.2"))

; instanceof (non-record pattern)
((instanceof_expression
  "instanceof" @range
  name: (_))
  (#set! hover.description "True if expression is non-null and matches.")
  (#set! hover.kind "pattern match operator")
  (#set! hover.spec "15.20.2"))

; instanceof (record pattern)
((instanceof_expression
  "instanceof" @range
  pattern: (_))
  (#set! hover.description "True if expression is non-null and matches.")
  (#set! hover.kind "pattern match operator")
  (#set! hover.spec "15.20.2"))

; break (no label)
((break_statement
  "break" @range)
  (#set! hover.description "Breaks out of switch or loop.")
  (#set! hover.kind "break statement")
  (#set! hover.spec "14.15"))

; break to label
((break_statement
  "break" @range
  (identifier))
  (#set! hover.description "Breaks to target: it is a `goto`.")
  (#set! hover.kind "break statement")
  (#set! hover.spec "14.15"))

; try-catch
((try_statement
  "try" @range)
  (#set! hover.description "Try statement with `catch` block for exception handling.")
  (#set! hover.kind "try-catch statement")
  (#set! hover.spec "14.20.1"))

; try with a finally clause
((try_statement
  "try" @range
  (finally_clause))
  (#set! hover.description "Try statement with a `finally` block that is always executed.")
  (#set! hover.kind "try-finally statement")
  (#set! hover.spec "14.20.2"))

; try with catches and a finally clause
((try_statement
  "try" @range
  (catch_clause)+
  (finally_clause))
  (#set! hover.description
    "Try statement with `catch` blocks for exception handling and a `finally` block that is always executed.")
  (#set! hover.kind "try-catch-finally statement")
  (#set! hover.spec "14.20.2"))

; basic try-with-resources
((try_with_resources_statement
  "try" @range)
  (#set! hover.description "Try statement with automatic resource closure.")
  (#set! hover.kind "try-with-resources statement")
  (#set! hover.spec "14.20.3.1"))

; extended try-with-resources with finally
((try_with_resources_statement
  "try" @range
  (finally_clause))
  (#set! hover.description
    "Try statement with automatic resource closure and a `finally` block that is always executed.")
  (#set! hover.kind "extended try-with-resources statement")
  (#set! hover.spec "14.20.3.2"))

; extended try-with-resources with catch
((try_with_resources_statement
  "try" @range
  (catch_clause)+)
  (#set! hover.description
    "Try statement with automatic resource closure and `catch` blocks for resource exception handling.")
  (#set! hover.kind "extended try-with-resources statement")
  (#set! hover.spec "14.20.3.2"))

; extended try-with-resources with catch and finally
((try_with_resources_statement
  "try" @range
  (catch_clause)+
  (finally_clause))
  (#set! hover.description
    "Try statement with automatic resource closure, `catch` blocks for resource exception handling, and a `finally` block that is always executed.")
  (#set! hover.kind "extended try-with-resources statement")
  (#set! hover.spec "14.20.3.2"))
