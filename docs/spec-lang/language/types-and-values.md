# Types and values

The runtime value system of the Altium Spec Language: the kinds of values an expression
can evaluate to, how dimensions normalize to Altium internal coordinates, and the special
values produced by coordinates, colors, shapes, and imports.

The value model is the `Value` enum in
[`crates/altium-format-spec/src/eval.rs`](../../../crates/altium-format-spec/src/eval.rs);
units are defined in
[`src/diagnostic.rs`](../../../crates/altium-format-spec/src/diagnostic.rs).

**Related pages**

- [Syntax](syntax.md) — the literal forms that produce these values
- [Expressions](expressions.md) — operations over values
- [Altium mapping](../reference/altium-mapping.md)

## The `Value` enum

Every expression evaluates to one `Value`. Each value reports a `kind_name()` used in type
mismatch errors.

| Value | `kind_name()` | Produced by |
| ----- | ------------- | ----------- |
| `String(String)` | `string` | string literals; bare unresolved identifiers |
| `Integer(i32)` | `integer` | integer literals |
| `Float(f64)` | `float` | float literals |
| `Dim(i32)` | `dim` | dimension literals (stored in internal units) |
| `Color(u8, u8, u8)` | `color` | color literals |
| `Bool(bool)` | `bool` | `true` / `false` |
| `Null` | `null` | `null` |
| `CoordPoint(i32, i32)` | `coord` | tuple expressions `(x, y)` |
| `Array(Vec<Value>)` | `array` | array literals `[ … ]` |
| `Object(IndexMap<String, Value>)` | `object` | object literals `{ … }` |
| `SwapGroup(String)` | `swap_group` | a declared swap-group reference |
| `ImportObject { alias, entries }` | `import_object` | an `import … as alias` binding |
| `ImportRef { alias, name }` | `import_ref` | field access on an import object |
| `Shape(Shape)` | `shape` | `rect()`, `circle()`, `polygon()`, … builtins |
| `ContourArc { … }` | `contour_arc` | the `arc(...)` builtin |

## Scalars

### Strings

UTF-8 text. Note one important behaviour: a bare identifier that is **not** bound in scope
evaluates to `Value::String` of its own name rather than erroring (`eval.rs`, `Expr::Ident`).
This lets enum-like identifiers (`passive`, `rectangular`, `TopLayer`) flow through to the
compiler, which resolves them against the expected enum type of the field.

```
electrical: passive      // `passive` → Value::String("passive")
```

A `$`-prefixed name has no such fallback — an undefined `$name` is an `UndefinedBinding`
error.

### Integers and floats

`Integer` is `i32`; `Float` is `f64`. Arithmetic promotes mixed `integer`/`float` operands
to `float` (see [Expressions](expressions.md)).

### Booleans and null

`Bool` and `Null` are distinct value kinds. `null` carries no payload.

## Dimensions and units

A dimension literal is normalized **at evaluation time** to a single `i32` count of Altium
internal coordinate units and stored as `Value::Dim(i32)`. Altium uses **10,000 internal
units per mil**. The conversion is `unit_to_internal` in `eval.rs`:

| Unit | Literal | Internal units | Factor |
| ---- | ------- | -------------- | ------ |
| `Mil` | `1mil` | 10,000 | `value × 10_000` |
| `Mm` | `1mm` | 393,701 | `value × 393_701` |
| `Inch` | `1in` | 10,000,000 | `value × 10_000_000` |
| `Dxp` | `1dxp` | 100,000 | `value × 100_000` |
| `Raw` | `1raw` | 1 | `value` (rounded) |

```
100mil   →  Value::Dim(1_000_000)
2.54mm   →  Value::Dim(1_000_000)   // ≈ 100 mil
```

A **bare** number used where a dimension is needed defaults to mils: `Value::to_dim`
multiplies an `Integer` by 10,000 (with overflow checking) and a `Float` by 10,000 rounded.
So `length: 25` means 25 mil.

`Value::Dim` displays back as a `…mil` string (`display()`), e.g. `Dim(1_000_000)` →
`"100mil"`.

> **Invariant.** The compiler resolves *all* unit conversions to internal coords before
> storing anything in the SpecModel; downstream stages (executor, reconciler) never parse
> unit strings. See the crate
> [README invariants](../../../crates/altium-format-spec/README.md).

**Maps to Altium:** `Value::Dim` is exactly an Altium `Coord` raw value. 10,000 units = 1 mil
is the native fabrication grid.

## Coordinate points

A two-element tuple expression `(x, y)` evaluates to `CoordPoint(i32, i32)`. Each component
is coerced to internal units via `to_dim`, so both dimensions and bare numbers work:

```
(-20mil, -10mil)   →  CoordPoint(-200000, -100000)
(0, 0)             →  CoordPoint(0, 0)
```

A `CoordPoint` exposes `.x` and `.y` fields (each a `Dim`); accessing any other field is an
error (see [Expressions § field access](expressions.md#field-access)).

## Colors

`Color(r, g, b)` holds three `u8` channel values and displays as `#RRGGBB`.

**Maps to Altium:** see [Syntax § colors](syntax.md#colors) and
[Altium mapping](../reference/altium-mapping.md) for the on-disk BGR encoding.

## Arrays and objects

- **Array** — an ordered `Vec<Value>`. Indexable by integer (negative indices count from
  the end). Example: `skip: [H8, H9, J8, J9]`.
- **Object** — an insertion-ordered `IndexMap<String, Value>`, built from `{ key: value }`
  literals. Supports the spread operator and string/integer indexing. Let-bindings inside
  an object body are scoping aids and do **not** become object entries.

Objects are the payload of most block bodies; see [Blocks overview](blocks-overview.md).

## Shapes and contour arcs

The geometry builtins return `Value::Shape(Shape)` or `Value::ContourArc { … }`, both in
internal units:

- `Shape` variants: `Rect`, `RoundedRect`, `Circle`, `Polygon`. A shape exposes `.width`,
  `.height` (both `Dim`, full bounding-box extent), and `.center` (a `CoordPoint`).
- `ContourArc` carries `endpoint`, `center`, `radius`, `start_angle`, `end_angle` and is
  used inside `outline:` arrays for PCB regions and component bodies.

These are produced by builtin function calls (`rect`, `rounded_rect`, `circle`, `polygon`,
`arc`, `inset`, `outset`, `translate`, …) documented in [Expressions](expressions.md).

## Import objects and import refs

An `import "lib" as alias` binding evaluates to an `ImportObject { alias, entries }` whose
`entries` map entity names to their string names. Field or index access on an import object
returns a provenance-tracked `ImportRef { alias, name }` rather than a plain string:

```
import "mcus.schlib-spec" as mcu
// ... $mcu          → ImportObject { alias: "mcu", entries: { ESP32_C6: "ESP32_C6", … } }
// ... $mcu.ESP32_C6 → ImportRef { alias: "mcu", name: "ESP32_C6" }
```

The compiler recognizes an `ImportRef` in a `symbol:` property and emits a typed
`SymbolRef::Import { alias, name }`, validating the name against the imported SchLib at
compile time. All other field-access paths return `Value::String`. See
[Expressions § import references](expressions.md#import-references) and the crate README
rationale for `Value::ImportRef`.
