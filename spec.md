
# Formal Language Specification

## Table of Contents

1.  [Introduction](#introduction)
2.  [Lexical Elements](#lexical-elements)
3.  [Grammar](#grammar)
4.  [Type System](#type-system)
5.  [Expressions](#expressions)
6.  [Statements](#statements)
7.  [Declarations](#declarations)
8.  [Memory Model](#memory-model)
9.  [Compilation Model](#compilation-model)
10.  [Standard Library](#standard-library)

## 1. Introduction

This specification defines a systems programming language with the following design goals:

-   Zero-cost abstractions through compile-time monomorphization
-   Memory safety through compile-time checks and explicit mutability
-   Structural typing for data, nominal typing for methods
-   No garbage collection or runtime type information
-   Compilation to C as an implementation strategy (not a language requirement)

### 1.1 Notation

This specification uses the following notational conventions:

-   `monospace` for keywords, operators, and code
-   _italic_ for syntactic categories
-   [brackets] for optional elements
-   {braces} for zero or more repetitions
-   (parens) for grouping
-   | for alternatives

### 1.2 Predefined Identifiers

The core module automatically provides the following type names:

-   Numeric types: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `int`, `uint`, `float`
-   Other types: `bool`, `nil`, `never`
-   Type constructors: `str`, `array`

These can be shadowed by user declarations.

## 2. Lexical Elements

### 2.1 Source Encoding

Source files are UTF-8 encoded text files.

### 2.2 Whitespace and Comments

Whitespace characters are space (U+0020), tab (U+0009), newline (U+000A), and carriage return (U+000D).

Comments begin with `//` and extend to the end of the line.

```
comment = "//" {any_char_except_newline}

```

### 2.3 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.

```
identifier = (letter | "_") {letter | digit | "_"}
letter = "a"..."z" | "A"..."Z"
digit = "0"..."9"

```

Examples:

```
x
_internal
camelCase
snake_case
Point2D
MAX_SIZE

```

### 2.4 Keywords

The following identifiers are reserved keywords:

```
as          break       continue    else        enum
false       for         if          in          match
mut         nil         ptr         return      self
sizeof      static      struct      true        type
where       while

```

### 2.5 Operators and Punctuation

```
+    -    *    /    %    &    |    ^    ~    !
<    >    <=   >=   ==   !=   &&   ||   <<   >>
=    :=   +=   -=   *=   /=   %=   &=   |=   ^=
<<=  >>=  
(    )    [    ]    {    }    ,    ;    :    .

```

### 2.6 Literals

#### 2.6.1 Integer Literals

```
int_literal = decimal_literal | hex_literal | binary_literal
decimal_literal = digit {digit}
hex_literal = "0x" hex_digit {hex_digit}
binary_literal = "0b" ("0" | "1") {"0" | "1"}
hex_digit = digit | "a"..."f" | "A"..."F"

```

#### 2.6.2 Floating-Point Literals

```
float_literal = digit {digit} "." {digit} [exponent]
              | digit {digit} exponent
exponent = ("e" | "E") ["+" | "-"] digit {digit}

```

#### 2.6.3 Boolean Literals

```
bool_literal = "true" | "false"

```

#### 2.6.4 String Literals

```
string_literal = '"' {string_char | escape_sequence} '"'
string_char = any_unicode_char_except_quote_or_backslash
escape_sequence = "\n" | "\r" | "\t" | "\\" | '\"' | "\0"
                | "\x" hex_digit hex_digit
                | "\u{" hex_digit {hex_digit} "}"

```

## 3. Grammar

### 3.1 Program Structure

```
program = {top_level_item}

top_level_item = type_declaration
               | function_declaration
               | static_declaration

```

### 3.2 Declarations

```
type_declaration = identifier ":" "type" ":=" type_expression

function_declaration = identifier [generic_params] ":" function_type [where_clause] block

static_declaration = "static" identifier [generic_params] ":" type_expression 
                     [where_clause] ":=" expression

generic_params = "[" generic_param {"," generic_param} "]"
generic_param = identifier [":" type_expression]
              | "static" identifier ":" type_expression

where_clause = "where" constraint {"," constraint}
constraint = identifier ":" type_expression

```

### 3.3 Types

```
type_expression = primary_type {type_operator primary_type}
type_operator = "|" | "&"

primary_type = identifier [generic_args]
             | "ptr" "[" type_expression "]"
             | "mut" "ptr" "[" type_expression "]"
             | "[" type_expression "]"
             | "[" expression type_expression "]"
             | struct_type
             | enum_type
             | function_type
             | "(" type_expression ")"

generic_args = "[" generic_arg {"," generic_arg} "]"
generic_arg = type_expression | expression

struct_type = "struct" "{" {struct_member} "}"
struct_member = identifier ":" type_expression ","
              | method_declaration

enum_type = "enum" [":" numeric_type] "{" enum_variant {"," enum_variant} [","] "}"
enum_variant = identifier [(":" type_expression) | ("=" expression)]
numeric_type = "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "int" | "uint"

function_type = "(" [param_list] ")" type_expression
param_list = param {"," param}
param = ["mut"] identifier ":" type_expression
      | "self"
      | "mut" "self"

```

### 3.4 Expressions

```
expression = assignment_expression

assignment_expression = logical_or_expression [assignment_op assignment_expression]
assignment_op = "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="

logical_or_expression = logical_and_expression {"||" logical_and_expression}
logical_and_expression = bitwise_or_expression {"&&" bitwise_or_expression}
bitwise_or_expression = bitwise_xor_expression {"|" bitwise_xor_expression}
bitwise_xor_expression = bitwise_and_expression {"^" bitwise_and_expression}
bitwise_and_expression = equality_expression {"&" equality_expression}

equality_expression = relational_expression {("==" | "!=") relational_expression}
relational_expression = shift_expression {("<" | ">" | "<=" | ">=") shift_expression}
shift_expression = additive_expression {("<<" | ">>") additive_expression}
additive_expression = multiplicative_expression {("+" | "-") multiplicative_expression}
multiplicative_expression = unary_expression {("*" | "/" | "%") unary_expression}

unary_expression = ("+" | "-" | "~" | "!" | "*" | "^") unary_expression
                 | postfix_expression

postfix_expression = primary_expression {postfix_op}
postfix_op = "[" expression "]"
           | "(" [expression_list] ")"
           | "." identifier
           | generic_args

primary_expression = identifier
                   | literal
                   | "sizeof" (type_expression | expression)
                   | struct_literal
                   | function_literal
                   | if_expression
                   | match_expression
                   | "(" expression ")"

expression_list = expression {"," expression}

struct_literal = "{" [field_init {"," field_init}] "}"
field_init = identifier ":" expression

function_literal = "|" [param_list] "|" type_expression block

if_expression = "if" expression block "else" (if_expression | block)
match_expression = "match" ["type"] expression "{" {match_arm} "}"

```

### 3.5 Statements

```
statement = expression_statement
          | declaration_statement
          | block_statement
          | if_statement
          | while_statement
          | for_statement
          | match_statement
          | return_statement
          | break_statement
          | continue_statement
          | labeled_statement

expression_statement = expression ";"
declaration_statement = ["mut"] identifier ":" type_expression ":=" expression ";"
block_statement = block
if_statement = "if" expression block ["else" (if_statement | block)]
while_statement = [label ":"] "while" expression block
for_statement = [label ":"] ("for" ["mut"] identifier "in" expression block |
                            "for" [for_init] ";" [expression] ";" [expression] block)
for_init = ["mut"] identifier ":" type_expression ":=" expression
match_statement = "match" ["type"] expression "{" {match_arm} "}"
match_arm = pattern ":" (block | expression ",")
return_statement = "return" [expression] ";"
break_statement = "break" [label] ";"
continue_statement = "continue" [label] ";"
labeled_statement = label ":" statement

block = "{" {statement} "}"
pattern = identifier | type_pattern | literal_pattern | "_"
type_pattern = type_expression "." identifier
literal_pattern = literal
label = identifier

```

## 4. Type System

### 4.1 Type Categories

Types are divided into the following categories:

1.  **Predefined types**: Types automatically available in every program
    -   Numeric types: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `int`, `uint`, `float`
    -   Boolean type: `bool`
    -   Unit type: `nil`
    -   Bottom type: `never` (for functions that don't return)
2.  **Pointer types**: `ptr[T]`, `mut ptr[T]`
3.  **Array types**: `[T]`, `[N T]`
4.  **Struct types**: Named and anonymous structs
5.  **Enum types**: Tagged unions with optional numeric backing type
6.  **Function types**: Function signatures
7.  **Type unions**: Compile-time constraints (not runtime types)
8.  **Type intersections**: Compile-time constraints

### 4.1.1 Predefined Types

The following types are automatically available from the core module:

-   `u8`, `u16`, `u32`, `u64`: Unsigned integers of specified bit width
-   `i8`, `i16`, `i32`, `i64`: Signed integers of specified bit width
-   `int`: Platform-dependent signed integer (32 or 64 bits)
-   `uint`: Platform-dependent unsigned integer (32 or 64 bits)
-   `float`: 32-bit floating point
-   `bool`: Boolean (1 byte)
-   `nil`: Unit type (0 bytes)
-   `never`: Bottom type (uninhabitable)

The `never` type is used for:

-   Functions that never return (e.g., infinite loops, `panic`)
-   Unreachable code branches
-   Empty enum variants in generic contexts

### 4.2 Type Equivalence

#### 4.2.1 Structural Equivalence

Two types are structurally equivalent if:

-   They are the same primitive type
-   They are pointer types to structurally equivalent types
-   They are array types with the same element type (and same size for fixed arrays)
-   They are struct types with the same fields in the same order with structurally equivalent types
-   They are the same named type

Examples:

```
// These types are structurally equivalent
type1: type := struct { x: i32, y: i32 }
type2: type := struct { x: i32, y: i32 }

p1: type1 := { x: 10, y: 20 }
p2: type2 := p1  // OK: structural equivalence

// These are NOT equivalent (different field names)
vec: type := struct { dx: i32, dy: i32 }
// v: vec := p1  // Error: field names don't match

```

#### 4.2.2 Assignment Compatibility

A type `S` is assignment-compatible with type `T` if:

-   `S` and `T` are structurally equivalent
-   `S` is a struct type that contains all fields of `T` (structural subtyping)
-   `T` is a type union and `S` is assignment-compatible with at least one member
-   `S` satisfies all constraints if `T` is an intersection type

Examples:

```
// Structural subtyping
point2d: type := struct { x: i32, y: i32 }
point3d: type := struct { x: i32, y: i32, z: i32 }

p3: point3d := { x: 1, y: 2, z: 3 }
p2: point2d := p3  // OK: point3d has all fields of point2d

// Type union constraint
numeric: type := i32 | float

process_number[T]: (n: T) T 
where T: numeric {
    return n * 2
}

result1: i32 := process_number(42)     // OK: i32 matches numeric
result2: float := process_number(3.14)  // OK: float matches numeric

```

### 4.3 Type Annotations

All types must be explicitly specified. There is no type inference. This includes:

-   Variable declarations (even with `:=`)
-   Function parameters and return types
-   Function literal parameters and return types
-   Generic type arguments

### 4.4 Generic Types

Generic types are parameterized by:

-   Type parameters: `T`, `U`, etc.
-   Value parameters: `static N: int`, etc.

Generic instantiation creates a new concrete type for each unique combination of arguments.

### 4.5 Method Sets

The method set of a type consists of all methods declared within that type's definition. Methods are namespaced to their declaring type.

## 5. Expressions

### 5.1 Expression Evaluation

Expressions are evaluated in the order specified by operator precedence and associativity. All expressions have a type and produce a value (except `nil`-typed expressions).

### 5.2 Arithmetic Operations

Binary arithmetic operators (`+`, `-`, `*`, `/`, `%`) require operands of the same numeric type and produce a result of that type.

Integer division truncates toward zero. Integer modulo has the sign of the dividend.

Examples:

```
// Integer arithmetic
x: i32 := 10 / 3      // 3 (truncates toward zero)
y: i32 := -10 / 3     // -3
z: i32 := 10 % 3      // 1
w: i32 := -10 % 3     // -1 (sign of dividend)

// Floating point
f1: float := 10.0 / 3.0    // 3.333...
f2: float := 22.0 / 7.0    // 3.142...

// Type mismatch
// bad: i32 := 10 + 3.14   // Error: mismatched types

```

### 5.3 Comparison Operations

Comparison operators (`<`, `>`, `<=`, `>=`) are defined for numeric types and produce a `bool` result.

Equality operators (`==`, `!=`) are defined for all types except type unions and produce a `bool` result. Struct equality compares all fields.

Examples:

```
// Numeric comparisons
b1: bool := 5 < 10        // true
b2: bool := 3.14 > 2.71   // true

// Struct equality
p1: point := { x: 10, y: 20 }
p2: point := { x: 10, y: 20 }
p3: point := { x: 10, y: 30 }

b3: bool := p1 == p2      // true (all fields equal)
b4: bool := p1 == p3      // false (y differs)

// Array equality
arr1: [3 i32] := {1, 2, 3}
arr2: [3 i32] := {1, 2, 3}
b5: bool := arr1 == arr2  // true (element-wise comparison)

```

### 5.4 Logical Operations

Logical operators (`&&`, `||`, `!`) operate on `bool` values. `&&` and `||` use short-circuit evaluation.

### 5.5 Bitwise Operations

Bitwise operators (`&`, `|`, `^`, `~`, `<<`, `>>`) operate on integer types. Right shift is arithmetic (sign-extending) for signed types.

### 5.6 Pointer Operations

The address-of operator `^` takes the address of an lvalue. The result type is:

-   `ptr[T]` if the operand is immutable
-   `mut ptr[T]` if the operand is mutable and a mutable pointer is required

The dereference operator `*` dereferences a pointer. The result is an lvalue if the pointer is mutable.

Examples:

```
// Taking addresses
x: i32 := 42
px: ptr[i32] := ^x         // Immutable pointer

mut y: i32 := 100
py: mut ptr[i32] := ^y     // Mutable pointer
// pz: ptr[i32] := ^y      // Also OK - can take immutable ptr to mutable

// Dereferencing
val: i32 := *px            // Read through pointer
*py = 200                  // Write through mutable pointer
// *px = 50                // Error: px is not mutable

// Pointer arithmetic
arr: [5 i32] := {10, 20, 30, 40, 50}
p: ptr[i32] := ^arr[0]
p2: ptr[i32] := p + 2      // Points to arr[2] (30)
val2: i32 := *(p + 3)      // 40 (arr[3])
diff: int := p2 - p        // 2 (elements between pointers)

```

### 5.7 Array and Pointer Operations

#### 5.7.1 Array Indexing

Array indexing `a[i]` requires `a` to be an array type (`[T]` or `[N T]`) and `i` to be an integer. Arrays cannot be used in arithmetic operations.

Examples:

```
// Fixed array indexing
arr: [5 i32] := {10, 20, 30, 40, 50}
val: i32 := arr[2]         // 30
mut marr: [3 i32] := {1, 2, 3}
marr[1] = 99               // OK: array is mutable

// Indexable pointer (no bounds info)
data: [i32] := get_data()  // Returns indexable pointer
val2: i32 := data[10]      // No bounds check possible

// Cannot do arithmetic on arrays
// bad := arr + 1          // Error: arrays don't support arithmetic

```

#### 5.7.2 Pointer Arithmetic

Pointers support arithmetic operations:

-   `p + n`: Advances pointer by `n * sizeof(T)` bytes
-   `p - n`: Moves pointer back by `n * sizeof(T)` bytes
-   `p1 - p2`: Difference between pointers (result in elements)

Pointers cannot be indexed with `[]`. To access elements, use `*(p + n)`.

### 5.8 Struct Operations

Field access `s.f` accesses field `f` of struct `s`. Method calls `s.m()` desugar to `T.m(s)` or `T.m(^s)` depending on the method signature.

### 5.9 Function Calls

Function calls `f(args)` require the number and types of arguments to match the function signature. Arguments are evaluated left to right.

### 5.10 Type Casts

Explicit type casts use the syntax `expr as T`. Valid casts:

-   Between numeric types
-   Between pointer types (including to/from integers)
-   Integer to pointer: `0 as ptr[T]` for null pointers

### 5.12 Never Type

Expressions of type `never` never produce a value. They arise from:

-   Functions that never return (infinite loops, program termination)
-   Unreachable code after `return`, `break`, `continue`
-   Match arms that are statically known to be unreachable

The `never` type is a subtype of all types, allowing unreachable code to type-check:

```
x: int := if condition { 42 } else { panic("error") }
// The else branch has type never, which is compatible with int

```

## 6. Statements

### 6.1 Expression Statements

Any expression can be used as a statement. The value is discarded.

### 6.2 Variable Declarations

Variable declarations using `:=` create a new variable in the current scope:

```
[mut] identifier := expression

```

Variables are immutable by default. The `mut` keyword makes them mutable.

Examples:

```
// Immutable variables
x: i32 := 42
name: str := "Alice"
point: struct { x: i32, y: i32 } := { x: 10, y: 20 }

// Mutable variables
mut count: i32 := 0
mut buffer: [100 u8] := { }
mut current: ptr[node] := 0 as ptr[node]

// Type must be explicit even with :=
n := 42                    // Error: missing type
m: i32 := 42              // OK

// Shadowing is allowed
x: i32 := 10
if true {
    x: str := "hello"      // New variable shadows outer x
    // Here x is a string
}
// Here x is still i32

```

### 6.3 Assignment Statements

Assignment requires the left-hand side to be a mutable lvalue:

```
lvalue = expression
lvalue op= expression

```

Examples:

```
mut x: i32 := 10
x = 20                     // OK
x += 5                     // OK: x is now 25

y: i32 := 30
// y = 40                  // Error: y is immutable

mut arr: [3 i32] := {1, 2, 3}
arr[1] = 99                // OK: array element assignment
arr[0] *= 2                // OK: compound assignment

mut p: struct { x: i32, y: i32 } := { x: 0, y: 0 }
p.x = 10                   // OK: field assignment

```

### 6.4 Control Flow

#### 6.4.1 If Statements

```
if condition { 
    // then-block
} else {
    // else-block
}

```

The condition must be of type `bool`. The else clause is optional.

#### 6.4.2 While Loops

```
[label:] while condition {
    // body
}

```

The condition must be of type `bool`. The loop executes while the condition is true.

#### 6.4.3 For Loops

For-in loops (for fixed-size arrays `[N T]` only):

```
for x in array {
    // body - x is immutable copy of each element
}

for mut x in array {
    // body - x is mutable pointer to each element (type is mut ptr[T])
}

```

The type of `x` is inferred from the array element type.

Examples:

```
// For-in with fixed array
numbers: [5 i32] := {1, 2, 3, 4, 5}
mut sum: i32 := 0

for n in numbers {
    sum += n               // n is a copy
}

// Modifying elements
mut data: [10 i32] := { }
for mut p in data {
    *p = 42                // p has type mut ptr[i32]
}

// C-style for loop
mut factorial: i32 := 1
for i: i32 := 1; i <= 5; i += 1 {
    factorial *= i
}

// Infinite loop with break
mut count: i32 := 0
for ; ; {
    if count >= 10 { break }
    count += 1
}

// Nested loops with labels
outer: for i: i32 := 0; i < 3; i += 1 {
    inner: for j: i32 := 0; j < 3; j += 1 {
        if i == j { continue outer }
        // process i, j
    }
}

```

C-style for loops:

```
for i: int := 0; i < 10; i += 1 {
    // body
}

```

The initialization, condition, and increment expressions are all optional.

#### 6.4.4 Match Statements

Value match:

```
match expr {
    pattern1: { /* body1 */ }
    pattern2: { /* body2 */ }
}

```

Type match (for enums):

```
match type expr {
    Type.Variant1: { /* can access expr.Variant1 */ }
    Type.Variant2: { /* can access expr.Variant2 */ }
}

```

Matches must be exhaustive for enum types.

Examples:

```
// Value match
status: i32 := get_status()
match status {
    0: { handle_success() }
    1: { handle_warning() }
    -1: { handle_error() }
    n: { handle_other(n) }  // Catch-all
}

// Type match with enum
result[T, E]: type := enum {
    ok: T,
    err: E
}

parse_result: result[i32, str] := parse_number("42")
match type parse_result {
    result.ok: { 
        // In this branch, parse_result.ok has type i32
        value: i32 := parse_result.ok
        process(value)
    }
    result.err: {
        // In this branch, parse_result.err has type str
        msg: str := parse_result.err
        log_error(msg)
    }
}

// Numeric enum match
color: type := enum: u8 {
    red = 0,
    green = 1,
    blue = 2
}

c: color := color.green
match c {
    color.red: { set_red() }
    color.green: { set_green() }
    color.blue: { set_blue() }
}

```

### 6.5 Expression Forms

#### 6.5.1 If Expressions

If expressions must have an else branch and both branches must have the same type:

```
x: int := if condition { 42 } else { 0 }

```

Examples:

```
// Basic if expression
max: i32 := if a > b { a } else { b }

// Nested if expression
grade: str := if score >= 90 { "A" } 
              else if score >= 80 { "B" }
              else if score >= 70 { "C" }
              else { "F" }

// If expression in struct literal
point: struct { x: i32, y: i32 } := {
    x: if use_default { 0 } else { get_x() },
    y: if use_default { 0 } else { get_y() }
}

```

#### 6.5.2 Match Expressions

Match expressions must be exhaustive and all arms must have the same type:

```
x: int := match val {
    0: { 100 },
    1: { 200 },
    n: { n * 10 }
}

```

Examples:

```
// Match expression for calculation
fibonacci: i32 := match n {
    0: { 0 },
    1: { 1 },
    2: { 1 },
    m: { fib(m-1) + fib(m-2) }
}

// Type match expression
option[T]: type := enum {
    some: T,
    none
}

opt: option[i32] := get_option()
value: i32 := match type opt {
    option.some: { opt.some },
    option.none: { -1 }  // Default value
}

// Match in function return
sign: (n: i32) i32 {
    return match n {
        0: { 0 },
        m: { if m > 0 { 1 } else { -1 } }
    }
}

```

### 6.6 Jump Statements

-   `return [expr]` - Returns from the current function
-   `break [label]` - Exits the labeled (or innermost) loop (while or for)
-   `continue [label]` - Continues the labeled (or innermost) loop (while or for)

## 7. Declarations

### 7.1 Type Declarations

Type declarations create new named types:

```
name: type := type_expression

```

Type aliases create new names for existing types.

Examples:

```
// Simple type alias
int_ptr: type := ptr[i32]
byte: type := u8

// Struct type
point: type := struct {
    x: i32,
    y: i32
}

// Generic type
pair[T, U]: type := struct {
    first: T,
    second: U
}

// Complex type with methods
vector2d: type := struct {
    x: float,
    y: float,
    
    // Static method (no self)
    new: (x: float, y: float) vector2d {
        return { x: x, y: y }
    }
    
    // Instance method
    length: (self) float {
        return sqrt(self.x * self.x + self.y * self.y)
    }
    
    // Mutable method
    normalize: (mut self) {
        len: float := self.length()
        if len > 0.0 {
            self.x = self.x / len
            self.y = self.y / len
        }
    }
}

// Usage
v1: vector2d := vector2d.new(3.0, 4.0)
mut v2: vector2d := { x: 5.0, y: 12.0 }
v2.normalize()

```

### 7.2 Function Declarations

```
name[generic_params]: (params) return_type where constraints {
    body
}

```

Functions without a body are external declarations.

Examples:

```
// Simple function
add: (a: i32, b: i32) i32 {
    return a + b
}

// Generic function
swap[T]: (mut a: ptr[T], mut b: ptr[T]) {
    tmp: T := *a
    *a = *b
    *b = tmp
}

// Function with constraints
sum[T]: (arr: array[T]) T where T: i32 | i64 | float {
    mut total: T := 0
    for i: int := 0; i < arr.len; i += 1 {
        total += arr.data[i]
    }
    return total
}

// Function returning never
panic: (msg: str) never {
    // Implementation prints message and exits
    exit(1)  // Never returns
}

// External function declaration
sqrt: (x: float) float  // No body

```

### 7.3 Static Declarations

Static declarations create compile-time constants:

```
static name[generic_params]: type where constraints := expression

```

The expression must be evaluable at compile time.

### 7.5 Enum Declarations

Enums can be declared with or without a numeric backing type:

```
// Tagged union enum (no backing type)
result[T, E]: type := enum {
    ok: T,
    err: E
}

// Simple enum without backing type
option: type := enum {
    some,
    none
}

// Numeric enum with backing type
color: type := enum: u8 {
    red,    // = 0
    green,  // = 1
    blue    // = 2
}

// Numeric enum with explicit values
flags: type := enum: u32 {
    read = 1,
    write = 2,
    execute = 4
}

```

For enums without backing type:

-   Each variant is its own unique type
-   Variants are only equal to themselves
-   Cannot be used in numeric contexts
-   Can have associated data per variant
-   Underlying representation is implementation-defined

For numeric enums (with backing type):

-   Variants without explicit values get the previous value + 1 (starting at 0)
-   Can be used in numeric contexts
-   Cannot have associated data (only simple variants)
-   Can be cast to/from their backing type

Examples:

```
// Using tagged union enum
parse_int: (s: str) result[i32, str] {
    // ... parsing logic ...
    if valid {
        return result.ok(value)
    } else {
        return result.err("Invalid integer format")
    }
}

res: result[i32, str] := parse_int("42")
match type res {
    result.ok: { 
        print("Got value: ", res.ok)
    }
    result.err: {
        print("Error: ", res.err)
    }
}

// Using numeric enum
permissions: type := enum: u8 {
    none = 0,
    read = 1,
    write = 2,
    read_write = 3
}

// Can use in numeric contexts
perm: permissions := permissions.read_write
if (perm as u8) & 1 != 0 {
    // Has read permission
}

// Complex enum example
expr: type := enum {
    literal: i32,
    add: struct { left: ptr[expr], right: ptr[expr] },
    mul: struct { left: ptr[expr], right: ptr[expr] },
    
    // Method on enum
    eval: (self) i32 {
        match type self {
            expr.literal: { return self.literal }
            expr.add: { 
                return (*self.add.left).eval() + (*self.add.right).eval()
            }
            expr.mul: {
                return (*self.mul.left).eval() * (*self.mul.right).eval()
            }
        }
    }
}

```

## 8. Memory Model

### 8.1 Memory Layout

-   All values have a size and alignment
-   Structs are laid out in declaration order with padding for alignment
-   Arrays are contiguous sequences of elements
-   Enums have a tag followed by the variant data

### 8.2 Stack and Heap

-   Local variables and parameters are stack-allocated
-   Heap allocation is explicit through library functions
-   No garbage collection

### 8.3 Lifetime Rules

-   Values have the lifetime of their containing scope
-   Pointers must not outlive the values they point to
-   The compiler performs escape analysis to prevent dangling pointers

### 8.4 Mutability

-   Values are immutable by default
-   Mutability is transitive through structs but not pointers
-   Mutable pointers can only be created to mutable values

## 9. Compilation Model

### 9.1 Compilation Units

Each source file is a compilation unit. All declarations in a file are visible throughout the file.

### 9.2 Name Resolution

Names are resolved in the following order:

1.  Local variables and parameters
2.  Type parameters
3.  Struct/enum members (for method calls)
4.  Global declarations

### 9.3 Generic Instantiation

Generic functions and types are monomorphized at compile time. Each unique instantiation generates separate code.

### 9.4 C Code Generation

When compiling to C:

-   Each function becomes a C function with mangled name
-   Each struct becomes a C struct
-   Each enum without backing type becomes a C struct with tag and union
-   Each numeric enum becomes typedef'd constants
-   Generic instantiations generate separate C definitions
-   Methods become regular functions with explicit self parameter

Example:

```
// Source
point: type := struct {
    x: i32, y: i32,
    distance: (self) i32 { 
        return sqrt(self.x * self.x + self.y * self.y) 
    }
}

color: type := enum: u8 {
    red,
    green,
    blue
}

option: type := enum {
    some,
    none
}

// Generated C
typedef struct {
    int32_t x;
    int32_t y;
} point;

int32_t point_distance(point self) {
    return sqrt(self.x * self.x + self.y * self.y);
}

typedef uint8_t color;
#define color_red ((color)0)
#define color_green ((color)1)
#define color_blue ((color)2)

typedef struct {
    enum { option_some, option_none } tag;
} option;

```

## 10. Module System

All code is organized into modules. There are no global built-in functions or types.

### 10.1 Module Structure

Each source file represents a module. The exact mechanism for imports, exports, and module naming is implementation-defined and subject to future specification.

### 10.2 Core Module

The core module is implicitly available and provides fundamental types:

#### 10.2.1 String Type

```
str: type := struct {
    data: ptr[u8],
    len: int
}

```

String literals create `str` values with static storage.

#### 10.2.2 Array Type

```
array[T]: type := struct {
    data: ptr[T],
    len: int
}

```

### 10.3 Other Modules

All functionality including memory allocation (e.g., `alloc`, `free`), I/O, math, etc. is provided by separate library modules that must be explicitly imported.

## Appendix A: Operator Precedence

From highest to lowest precedence:

1.  Postfix: `()` `[]` `.`
2.  Unary: `+` `-` `~` `!` `*` `^` `sizeof`
3.  Multiplicative: `*` `/` `%`
4.  Additive: `+` `-`
5.  Shift: `<<` `>>`
6.  Relational: `<` `>` `<=` `>=`
7.  Equality: `==` `!=`
8.  Bitwise AND: `&`
9.  Bitwise XOR: `^`
10.  Bitwise OR: `|`
11.  Logical AND: `&&`
12.  Logical OR: `||`
13.  Assignment: `=` `+=` `-=` etc.

## Appendix B: Memory Sizes

Sizes are platform-dependent but typical values are:

Fixed-size types:

-   `bool`: 1 byte
-   `u8`, `i8`: 1 byte
-   `u16`, `i16`: 2 bytes
-   `u32`, `i32`: 4 bytes
-   `u64`, `i64`: 8 bytes
-   `float`: 4 bytes
-   `nil`: 0 bytes

Platform-dependent types:

-   `int`, `uint`: 4 bytes (32-bit) or 8 bytes (64-bit)
-   `ptr[T]`, `mut ptr[T]`: 4 bytes (32-bit) or 8 bytes (64-bit)

Composite types:

-   `[N T]`: N × sizeof(T) bytes
-   Structs: Sum of field sizes plus padding for alignment
-   Enums: Tag size (typically 4 bytes) plus size of largest variant

## Appendix C: Undefined Behavior

The following operations have undefined behavior:

-   Dereferencing null or invalid pointers
-   Array access out of bounds
-   Integer overflow (wrap-around is implementation-defined)
-   Use of uninitialized variables
-   Returning pointers to local variables
-   Data races in concurrent access
