# Expressions

Expressions compute values: arithmetic over dimensions and numbers, references to bindings
and imports, field and index access, template interpolation, spread, and builtin geometry
functions. This page describes each expression form and how it evaluates.

Expressions are the `Expr` enum in
[`src/ast.rs`](../../../crates/altium-format-spec/src/ast.rs); evaluation is in
[`src/eval.rs`](../../../crates/altium-format-spec/src/eval.rs).

**Related pages**

- [Syntax](syntax.md) — the tokens expressions are built from
- [Types and values](types-and-values.md) — the values expressions produce
- [Blocks overview](blocks-overview.md) — where expressions appear (property values)
- [Annotations](annotations.md)

## Literals

String, template, integer, float, dimension, color, boolean, and null literals are
expressions that evaluate to the corresponding value (see
[Types and values](types-and-values.md)).

## Arithmetic operators

Four binary operators (`BinOp`): `+`, `-`, `*`, `/`. Multiplication and division bind
tighter than addition and subtraction:

```
x = 2 + 3 * 4     // parses as 2 + (3 * 4)
```

Unary negation (`-expr`) is supported on `integer`, `float`, and `dim`.

Evaluation rules (`eval_binop` in `eval.rs`):

| Left | Op | Right | Result |
| ---- | -- | ----- | ------ |
| `dim` | `+` `-` | `dim` | `dim` (checked, overflow → error) |
| `dim` | `*` | `integer`/`float` | `dim` (scaled) |
| `integer`/`float` | `*` | `dim` | `dim` (scaled) |
| `dim` | `/` | `integer`/`float` | `dim` (div-by-zero → error) |
| `integer` | `+` `-` `*` `/` | `integer` | `integer` (checked) |
| `float` and mixed `integer`/`float` | any | | `float` |

```
100mil + 2.54mm     // dim + dim → dim
0.8mm * 8           // dim * int → dim
```

Any other operand combination (e.g. `string + integer`) is a `TypeMismatch` error. Integer
overflow is `ArithmeticOverflow`; division by zero is `DivisionByZero`.

## References

### Bare identifiers

A bare identifier is looked up in the scope stack. If bound, its value is returned; if
**not** bound, it evaluates to `Value::String` of its own name (so enum-like tokens reach
the compiler). See [Types and values § strings](types-and-values.md#strings).

### `$`-references

A `$name` resolves a binding (or import alias). Unlike bare identifiers, an undefined
`$name` is a hard `UndefinedBinding` error. `$`-references are the normal way to refer to
entities you bound earlier:

```
body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
pin 1 { on: $body.left, at: center }     // $body refers to the binding above
```

## Field access

`expr.field` (`Expr::Path`) reads a named member of the base value (`eval_field_access`):

| Base value | Valid fields |
| ---------- | ------------ |
| `object` | any present key |
| `coord` | `x`, `y` (each a `dim`) |
| `shape` | `width`, `height` (each a `dim`), `center` (a `coord`) |
| `import_object` | any imported entity name → an `import_ref` |

```
$body.width        // dim
$body.center       // coord
$mcu.ESP32_C6      // import_ref
```

Accessing a missing or invalid field is an `InvalidFieldAccess` error.

## Index access

`expr[index]` (`Expr::Index`):

- `array[integer]` — element access; negative indices count from the end.
- `object[string]` or `object[integer]` — keyed access (`integer` is stringified).
- `import_object[string]` — like field access, yields an `import_ref`.

```
x = $fp["SOT-23"]      // object index by string key
```

Out-of-bounds array access is `IndexNotArray`; a missing object key is `InvalidFieldAccess`.

## Tuples (coordinates)

A parenthesized pair `(x, y)` evaluates to a `CoordPoint`, coercing each component to
internal units. See [Types and values § coordinate points](types-and-values.md#coordinate-points).

## Arrays and objects

- Array literal `[a, b, c]` → `Array`.
- Object literal `{ key: value, ... }` → `Object` (insertion-ordered). Objects may contain
  `let` bindings (scoping only — not emitted as entries) and spreads.

## Spread

The `...expr` spread (`ObjectItem::Spread`) merges another object's entries into the
enclosing object. The spread source must evaluate to an object (or import object); later
keys overwrite earlier ones:

```
let passive_pin = { electrical: passive, length: 25, side: outside }
pin 1 { ...passive_pin, on: $body.left, at: center }

x = { ...defaults, shape: rectangular }
```

A non-object spread source is a `SpreadNotObject` error.

## Template strings

A backtick template interpolates expressions inside `{ … }`. Each hole is parsed and
evaluated as an expression, and the result is rendered with `Value::display()` and
concatenated with the literal segments:

```
`R{index}`               // → "R3" when index = 3
`prefix {$body.width} suffix`
```

See [Syntax § template strings](syntax.md#template-strings) for the lexical form and escape
rules.

## `let` bindings and scopes

A `let name = expr` introduces a binding in the current scope. Bindings are evaluated in
source order (left-to-right), so a later binding may reference an earlier one. Scopes nest:
each block body pushes a new `Scope` frame onto the `ScopeStack`, and lookup searches from
innermost to outermost.

```
let passive_pin = { electrical: passive, length: 25, side: outside }
let two_pin_body = { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }

component R {
    body = rectangle { ...two_pin_body }
    pin 1 { ...passive_pin, on: $body.left }
}
```

Circular references are detected with a "currently evaluating" sentinel and reported as
`CircularBinding`. (The `let` keyword may be omitted in some positions; see
[Blocks overview](blocks-overview.md).)

## Import references

When the base of a field/index access is an `ImportObject`, the result is an
`ImportRef { alias, name }` carrying which import the symbol came from. The compiler uses
this provenance to emit a typed `SymbolRef::Import` and to validate the referenced symbol
against the imported library at compile time:

```
import "mcus.schlib-spec" as mcu

sheet { }
component U1 {
    symbol: $mcu.ESP32_C6        // ImportRef → SymbolRef::Import { alias: "mcu", name: "ESP32_C6" }
}
```

All other field-access paths return a plain `Value::String`. See
[Types and values § import objects and refs](types-and-values.md#import-objects-and-import-refs)
and the crate README rationale.

## Builtin function calls

A call `name(arg, key: arg, ...)` (`Expr::Call`) dispatches to a fixed set of builtins
(`eval_builtin_call`). Arguments may be positional or named (`CallArg`). An unknown
function name is a `NotSupported` error.

| Function | Form | Returns |
| -------- | ---- | ------- |
| `rect` | `rect(w, h)`, `rect(from:, to:)`, or `rect(at:, width:, height:)` | `shape` (`Rect`) |
| `rounded_rect` | `rounded_rect(w, h, r)` | `shape` (`RoundedRect`) |
| `circle` | `circle(r)` | `shape` (`Circle`) |
| `polygon` | `polygon([pts])` (≥ 3 points) | `shape` (`Polygon`) |
| `arc` | `arc(endpoint:, center:, radius:, start_angle:, end_angle:)` | `contour_arc` |
| `inset` | `inset(shape, amount)` | shrunk `shape` |
| `outset` | `outset(shape, amount)` | grown `shape` |
| `translate` | `translate(shape, …)` | moved `shape` |
| `width` / `height` / `center` | `width(shape)` etc. | `dim` / `dim` / `coord` |
| `min` / `max` / `clamp` / `abs` | numeric helpers | number/dim |

All shape coordinates are in internal units. `center` defaults to `(0, 0)` when omitted.
`inset`/`outset` require non-negative amounts and reject polygon shapes.

```
body = rect(from: (-3.5mm, -3.5mm), to: (3.5mm, 3.5mm))
outline: [ arc(endpoint: (0, 0), center: (0, 0), radius: 1mm, start_angle: 0, end_angle: 90) ]
```
