> **Related docs**: [ops-design.md](ops-design.md) | [ops-lang-spec.md](ops-lang-spec.md) | [schlib-ops.md](schlib-ops.md) | [schdoc-ops.md](schdoc-ops.md) | [ops-e2e-gaps.md](ops-e2e-gaps.md) | [ops-lang-checklist.md](ops-lang-checklist.md)

# Altium Ops Language Specification

Version: 0.2 (draft)
File extension: `.ops`

## 1. Overview

The Altium Ops Language is a domain-specific language for describing operations on Altium
Designer files (SchDoc, SchLib, PcbDoc, PcbLib). It is designed for LLM agents: minimal
tokens, self-describing operations, zero ambiguity, and first-class support for document
references, arithmetic, units, and queries.

The language replaces YAML/JSON specs with a purpose-built format that:

- Makes every value position an **expression** (no prefix needed)
- Uses **bare identifiers** for enums (resolved by field type)
- Requires **quoted strings** (eliminates YAML's type ambiguity)
- Supports **dimensional scalars** with unit suffixes (`20mm`, `100mil`)
- Supports **Coords** as `(x, y)` tuples
- Embeds the **Altium Query Language (AQL)** for selectors in mutation ops
- Chains op results via **`$` references** (`$last`, `$name.field`)
- Provides **bindings** (`name = expr`) with object spread for deduplication
- Supports **`assert`** for pre-condition checking (anti-hallucination)
- Carries **source spans** on every AST node for precise error reporting

### Relationship to other docs

- `docs/ops-design.md` — architecture, lowering pipeline, field mapping tables
- `docs/query-lang.md` — AQL reference (selectors, combinators, pseudo-classes)
- `docs/schlib-ops.md` — SchLib low-level op inventory


## 2. Design Goals

1. **Token-minimal.** Every syntactic element earns its place. No `op:` field, no `=`
   prefix, no `<>` wrappers, no `-` list markers.
2. **Self-describing.** The op name IS the syntax: `add_component { ... }`.
3. **Unambiguous.** Strings are always quoted. Enums are bare identifiers resolved by
   field type. Numbers and dimensional values are syntactically distinct.
4. **Expression-native.** Every value position is an expression. References and arithmetic
   work everywhere without special markers.
5. **Agent-friendly.** Schema introspection, structured error messages with source spans,
   and a result model that feeds forward into subsequent ops.
6. **Anti-hallucination.** `assert` validates assumptions before mutation. Bindings + spread
   eliminates copy-paste errors. Typed values catch wrong-type fields at parse time.
   Every AST node carries a `Span` for precise diagnostics.


## 3. File Structure

An `.ops` file is a sequence of **statements** separated by whitespace. Statements are
either operations, bindings, or `assert` checks. No list markers, no top-level
delimiters — JSONL-style.

```
// Layout parameters
pin_defaults = { electrical: passive, length: 25 }
spacing = 300

// Verify document state before mutating
assert U1.lib_reference == "LM358", `expected LM358, got {U1.lib_reference}`

// Two resistors and a wire connecting them
r1 = add_component {
    designator: "R1", lib_reference: "R", value: "10K"
    location: (1000, 800)
    pins: [
        { designator: "1", ...pin_defaults, offset: (-50, 0) }
        { designator: "2", ...pin_defaults, offset: (50, 0) }
    ]
}

add_component {
    designator: "R2", lib_reference: "R", value: "10K"
    location: ($r1.location.x + spacing, $r1.location.y)
    pins: [
        { designator: "1", ...pin_defaults, offset: (-50, 0) }
        { designator: "2", ...pin_defaults, offset: (50, 0) }
    ]
}

add_wire { points: [R1.pin[2].location, R2.pin[1].location] }
```

The parser recognizes statement boundaries by the leading token: `assert` keyword,
IDENT followed by `=` (binding), or IDENT followed by `{`/AQL tokens (bare op).


## 4. Op Syntax

Each op has a **shape** determined by its name. The op name tells the parser what
grammar to expect next.

### 4.1 Create Ops

```
OP_NAME { fields }
NAME = OP_NAME { fields }
OP_NAME TARGET { fields }
NAME = OP_NAME TARGET { fields }
```

Place or create a record. The body contains field assignments parsed as expressions.
If `NAME =` is provided, the op result is bound to `$NAME` for later reference.

**Parent targeting:** Some create ops accept a target expression before the body.
This specifies the parent record for child entities. The target is an expression
(typically a `$ref`) — not a selector.

```
r1 = add_component {
    designator: "R1"
    lib_reference: "R"
    value: "10K"
    location: (U1.location.x + 400, U1.location.y)
    orientation: 0
    pins: [
        { designator: "1", electrical: passive, offset: (-50, 0), length: 25 }
        { designator: "2", electrical: passive, offset: (50, 0), length: 25 }
    ]
}

// Add a pin to an existing component via parent target
add_pin $r1 { designator: "3", electrical: input, offset: (0, -50), length: 25 }

add_wire { points: [U1.pin[14].location, R1.pin[1].location] }

add_net_label { name: "DATA_BUS", location: (500, 1200), orientation: 0 }

add_power_port {
    name: "VCC", style: bar
    location: (U1.pin[14].location.x, U1.pin[14].location.y + 100)
    orientation: 90
}

add_junction { location: (500, 800), color: #0000FF }

add_track {
    start: U1.pad[1].location
    end: (U1.pad[1].location.x + 2.54mm, U1.pad[1].location.y)
    width: 0.254mm, layer: Top, net: "VCC"
}

add_via { location: (1000, 500), hole_size: 0.3mm, diameter: 0.6mm, net: "GND" }
```

**Available create ops:**

| Op | Target | Document types | Description |
|----|--------|---------------|-------------|
| `add_component` | — | SchDoc, SchLib | Place component / define symbol |
| `add_pin` | component ref | SchDoc, SchLib | Add pin to existing component |
| `add_wire` | — | SchDoc | Add wire segment |
| `add_net_label` | — | SchDoc | Add net label |
| `add_power_port` | — | SchDoc | Add power port |
| `add_junction` | — | SchDoc | Add junction |
| `add_track` | — | PcbDoc | Add PCB track |
| `add_via` | — | PcbDoc | Add via |
| `add_pad` | footprint ref | PcbLib | Add pad to existing footprint |
| `add_footprint` | — | PcbLib | Define footprint |

See `docs/ops-design.md` §Layer 2 for full field tables per op.

**Target parsing:** The parser checks whether the op accepts a target (op-name-driven).
If so, it parses a path expression before `{`. Targets are always simple path
expressions (`$ref`, `$ref.field`, etc.) — never objects or arrays — so `{` is
an unambiguous body boundary.

### 4.1.1 Placement Op (`place`)

```
place TARGET { placement_fields }
NAME = place TARGET { placement_fields }
```

Place an existing entity relative to an anchor entity without requiring raw
coordinates in the op body.

```
comp = add_component { designator: "U1", lib_reference: "IC" }
rect = add_rectangle $comp { from: (-200mil, -100mil), to: (200mil, 100mil) }
pin1 = add_pin $comp { designator: "1" }
pin2 = add_pin $comp { designator: "2" }
pin3 = add_pin $comp { designator: "3" }

place_defaults = { gap: 20mm, side: outside, orientation: auto }

place $pin1 { on: $rect.top, at: start }
place $pin2 { ...place_defaults, on: $rect.top, after: $pin1 }
place $pin3 { on: $rect.left, at: center }
```

Placement fields:

- `on` (required): anchor reference (for example `$rect.top`, `$rect.left`, `$track.start`)
- exactly one of: `at` (`start`|`center`|`end`), `after`, `before`
- `gap` (optional dim): spacing for `after`/`before`
- `offset` (optional coord): post-placement translation
- `side` (optional): `inside`|`outside`|`center`
- `orientation` (optional): `auto`|`0`|`90`|`180`|`270`
- `mode` (optional): `translate`|`project`

Geometry-class semantics:

- point objects (pin, label, junction, via, pad, text): resolve final point and set location
- segment objects (line, wire, bus, track): default `translate`; `project` snaps nearest endpoint
- box objects (rectangle, round-rectangle, text-frame, image, fill): translate box preserving size
- center+radii objects (arc, ellipse, pie): move center, preserve radii unless explicitly edited
- vertex-list objects (polyline, polygon, bezier, region outlines): translate all vertices

Non-placeable objects (metadata/container records such as implementation list/map, map-definer,
parameter list, aliases) must fail typechecking with a placement error.

Anchor style: placement anchors use member access (`$rect.top`) rather than quoted index keys.

### 4.2 Edit Ops

```
edit SELECTOR { patch_fields }
NAME = edit SELECTOR { patch_fields }
```

Modify all records matching the AQL selector. The body contains field assignments
applied as a patch to each matched record.

```
// Edit by designator
edit component[designator=R1] {
    value: "20K"
}

// Edit with self-reference (relative move)
edit R* {
    location.x: $self.location.x + 200
}

// Edit op results from a prior query
caps = query C*
edit $caps[value>100nF] {
    footprint: "0805"
}
```

**Selector parsing:** The parser reads AQL tokens after `edit` until it encounters
`{`. AQL syntax never contains bare `{`, so this boundary is unambiguous.

**`$self` in patch fields:** Inside the edit body, `$self` refers to the current
record being patched. This enables relative modifications.

**Dotted keys:** Patch fields support dotted paths for nested field access:
`location.x: $self.location.x + 100`.

**Returns:** `OpResult` with `modified` (list of modified entity refs) and `count`.

### 4.3 Remove Ops

```
remove SELECTOR
NAME = remove SELECTOR
```

Delete all records (and their children) matching the AQL selector.

```
remove component[designator=R5]
remove R*[value<100]
remove $caps[value<10nF]
```

**Selector parsing:** The parser reads AQL tokens after `remove` until it reaches a
newline followed by a root-level identifier (the next statement) or EOF.

**Returns:** `OpResult` with `removed` (list of removed entity refs) and `count`.

### 4.4 Query Ops

```
query SELECTOR
NAME = query SELECTOR
```

Select records matching the AQL selector without modifying the document. Results are
stored in the result table and accessible via `$NAME` or `$last`.

```
power_pins = query pin:power
small_r = query R*[value<1K]
all_caps = query C*

// Use query results in later ops
add_wire {
    start: $power_pins.first.location
    end: (500, 200)
}

// Edit query results
edit $small_r { value: "1K" }
```

**Selector parsing:** Same as `remove` — AQL tokens until next statement or EOF.

**Returns:** `OpResult` with `refs` (all matching entity refs), `count`, `first`,
and `last`.

### 4.5 Bindings

Both expression values and op results use the same binding syntax:

```
// Expression bindings
spacing = 300
pin_defaults = { electrical: passive, length: 25 }

// Op result bindings
r1 = add_component { ... }       // result accessible as $r1
caps = query C*                   // result accessible as $caps
```

Unnamed ops still execute and their result is accessible via `$last`:

```
add_wire { ... }                  // unnamed, result in $last
```

The parser distinguishes bindings from ops by look-ahead: IDENT followed by `=`
is a binding; IDENT followed by `{` or AQL tokens is a bare op.

**What follows `=`:** If the RHS starts with an op name (`add_component`, `place`, `edit`,
`remove`, `query`, etc.), it's an op result binding. Otherwise it's an expression
binding (lazy — evaluated at each use site, §4.7).

Binding names follow identifier rules: `[a-zA-Z_][a-zA-Z0-9_]*`. Names must
be unique within their scope. `$last` always refers to the most recent op's
result regardless of naming.

### 4.6 Assert

```
assert CONDITION
assert CONDITION, "plain message"
assert CONDITION, `template with {expr} interpolation`
```

Validate assumptions about the document before mutating it. If the condition is false,
execution halts with an error **before any subsequent ops execute**. This is the
primary anti-hallucination mechanism: an LLM can assert its mental model of the
document and fail safely if wrong.

**Condition forms:**

```
// Comparison — two expressions and an operator
assert U1.lib_reference == "LM358"
assert U1.pin[14].electrical == power
assert $r1.count > 0
assert R1.location.x >= 500
assert R1.value != "DNP"

// Existence — expression is non-null
assert U1
assert U1.pin[14]
assert $r1.count
```

**Comparison operators:** `==`, `!=`, `>`, `<`, `>=`, `<=`

**Existence checks:** A bare expression without a comparison operator asserts that
the value is non-null. `assert U1` checks that a component with designator U1
exists. `assert U1.pin[14]` checks that pin 14 exists on U1.

**Assert messages:**

The optional message can be a plain string (`"..."`) or a template string (`` `...` ``)
with `{expr}` interpolation. Use plain strings for static messages and template strings
when you need to include runtime values.

```
assert U1.lib_reference == "LM358",
    `expected LM358 at U1, got {U1.lib_reference}`

assert U1.pin[14].name == "VCC",
    `pin 14 is {U1.pin[14].name} (electrical: {U1.pin[14].electrical}), expected VCC`

assert $r1.count > 0,
    `query matched {$r1.count} components, expected at least 1`
```

**Assert does not return an OpResult.** It is a check, not an operation.

### 4.7 Expression Bindings

```
NAME = expr
```

Bind a name to an expression. The value can be a scalar, array, object, or any
expression. Bindings are **immutable** and **lazy** — the expression AST is stored
at definition and evaluated at each use site. This means `$last`, `$self`, and
document references resolve against the state at the point of use, not definition.

#### File-Level Scope

Bindings at the top level (between ops/asserts) are visible to all subsequent
statements:

```
spacing = 300
base_y = 800
pin_defaults = { electrical: passive, length: 25 }

add_component {
    designator: "R1"
    location: (1000, base_y)
    pins: [
        { designator: "1", ...pin_defaults, offset: (-50, 0) }
        { designator: "2", ...pin_defaults, offset: (50, 0) }
    ]
}

add_component {
    designator: "R2"
    location: (1000 + spacing, base_y)
    pins: [
        { designator: "1", ...pin_defaults, offset: (-50, 0) }
        { designator: "2", ...pin_defaults, offset: (50, 0) }
    ]
}
```

#### Block-Level Scope

Bindings inside an object body `{ }` are scoped to that block. The parser
distinguishes them from fields by `=` vs `:`:

```
add_component {
    half_pitch = 50            // binding (=), scoped to this block
    designator: "U1"          // field (:)
    pins: [
        { designator: "1", offset: (-half_pitch, 0), electrical: input }
        { designator: "2", offset: (half_pitch, 0), electrical: output }
    ]
}
// half_pitch is not visible here
```

#### Shadowing

Inner scopes shadow outer bindings of the same name:

```
x = 100
add_component {
    x = 200                   // shadows file-level x within this block
    location: (x, 0)         // uses 200
}
// x is 100 again
```

#### Expressions in Bindings

Binding values are full expressions — they can reference document entities, op
results, other bindings, and perform arithmetic:

```
ic_x = U1.location.x
resistor_offset = 400
r1_x = ic_x + resistor_offset

add_component {
    designator: "R1"
    location: (r1_x, U1.location.y)
}
```

#### Lazy Evaluation

Expression bindings store the expression AST and evaluate it at each use site.
This means `$last`, `$self`, and document references resolve against the state
at the point of use:

```
connect_to_last = $last.pin[1].location

r1 = add_component { ... }
add_wire { start: connect_to_last }  // $last = r1's result

r2 = add_component { ... }
add_wire { start: connect_to_last }  // $last = r2's result

edit U1 { location: (0, 0) }
add_wire { start: (U1.location.x, 100) }  // sees updated U1 location
```

Since there is no mutation of bindings and no control flow, lazy evaluation
is deterministic — the result depends only on the document state and result table
at the point of use.

#### Object Bindings

Bindings can hold entire objects for use with spread:

```
smd_resistor = {
    lib_reference: "R"
    value: "10K"
    footprint: "0805"
}

passive_pin = { electrical: passive, length: 25 }

add_component {
    ...smd_resistor
    designator: "R1"
    location: (1000, 800)
    pins: [
        { designator: "1", ...passive_pin, offset: (-50, 0) }
        { designator: "2", ...passive_pin, offset: (50, 0) }
    ]
}

add_component {
    ...smd_resistor
    designator: "R2"
    value: "20K"        // overrides the spread value
    location: (1300, 800)
    pins: [
        { designator: "1", ...passive_pin, offset: (-50, 0) }
        { designator: "2", ...passive_pin, offset: (50, 0) }
    ]
}
```


## 5. Expression Language

**Every value position is an expression.** There is no special prefix (`=`) or
delimiter (`<>`) to mark expressions — the Pratt parser runs on every value.

Literals (strings, numbers, booleans) are trivial expressions. References,
arithmetic, and function-like constructs compose into compound expressions.

### 5.1 Value Literals

| Syntax | Type | Examples |
|--------|------|---------|
| `"..."` | String | `"R1"`, `"10K"`, `"0805"` |
| `` `...` `` | Template string | `` `expected {U1.value}` `` |
| `42`, `-5` | Integer | Bare digits, optional sign |
| `3.14`, `-0.5` | Float | Has decimal point |
| `20mm`, `100mil` | Dim (scalar with units) | Number immediately followed by unit suffix |
| `true` / `false` | Bool | |
| `null` | Null | |
| `#FF0000` | Color | `#` + exactly 6 hex digits |

**Strings** are always quoted. Escape sequences: `\"`, `\\`, `\n`, `\t`, `\r`.
`{` and `}` have no special meaning inside regular `"..."` strings.

**Template strings** use backticks (`` ` ``). They support `{expr}` interpolation
where `{expr}` is replaced by the evaluated expression value at runtime. Literal
`{` and `}` inside template strings are escaped as `{{` and `}}`. Escape sequences:
`` \` ``, `\\`, `\n`, `\t`, `\r`, `\{`, `\}`. Template strings evaluate to `String`
at runtime.

**Integers and floats** are plain numbers without unit suffixes. In a field that
expects a dimensional value, a bare number defaults to mils. The type checker
handles this, not the parser.

**Dimensional scalars** ("dims") are numbers with a unit suffix. The suffix is lexed
as part of the token — `20mm` is one token, not `20` followed by identifier `mm`.

**Colors** start with `#` followed by exactly 6 hex digits. Named colors (`red`,
`blue`, etc.) are bare identifiers resolved by the enum registry when the field
type is `Color`.

### 5.2 Operators & Precedence

The Pratt parser handles expression parsing with these binding powers:

| Precedence | Operators | Associativity | Description |
|------------|-----------|---------------|-------------|
| 90 | `.` `[expr]` | left | Field access, index |
| 70 | unary `-` | prefix | Negation |
| 60 | `*` `/` | left | Multiply, divide |
| 50 | `+` `-` | left | Add, subtract |

Comparison operators (`==`, `!=`, `>`, `<`, `>=`, `<=`) are **not** general
expression operators. They only appear in `assert` conditions (§4.6).

**Type rules for arithmetic:**

| Left | Op | Right | Result |
|------|----|-------|--------|
| dim | `+` `-` | dim | dim |
| dim | `*` | number | dim |
| dim | `/` | number | dim |
| number | `*` | dim | dim |
| number | `+` `-` `*` `/` | number | number |

Dim + dim works even with mixed units: `100mil + 2.54mm` evaluates correctly
because both convert to internal units before arithmetic.

Coord (point) arithmetic is NOT supported at the expression level. Coords are
constructed via tuple syntax `(x_expr, y_expr)` where each component is a scalar
expression.

### 5.3 Path Expressions (References)

Path expressions navigate document entities, bindings, and op results:

```
// Document references (designator lookup)
U1                          // component record
U1.location                 // Coord (x, y)
U1.location.x               // dim (single axis)
U1.location.y               // dim
U1.pin[1]                   // pin record (by designator)
U1.pin[1].location          // Coord
U1.pin[1].location.x        // dim
U1.pin[VCC]                 // pin by name
U1.pad[A1].location         // PCB pad

// Let bindings
spacing                     // file-level let value
pin_defaults                // let-bound object (for spread)

// Op result references (from let NAME = op ...)
$r1                         // OpResult from r1 =op ...
$r1.location                // primary entity's location
$r1.location.x              // x component
$r1.top                     // named anchor (when provided by entity/op result)
$r1.ref                     // primary EntityRef
$r1.refs[0]                 // first secondary entity
$r1.count                   // number of entities

// Built-in aliases
$last                       // most recent op's result
$last.location              // navigate it
$self                       // current record (in edit ops)
$self.location.x            // current record's x
$self.value                 // current record's value
$sheet                      // sheet-level properties
$sheet.width                // sheet width
$sheet.height               // sheet height
```

**Path syntax:**

```
path       = root { step }
root       = '$' IDENT          // op result ref ($r1) or alias ($last, $self, $sheet)
           | IDENT              // binding, enum, or document entity
step       = '.' IDENT          // field access
           | '[' key ']'        // index access
key        = INTEGER            // numeric index: pin[1], refs[0]
           | IDENT              // named index: pin[VCC], pad[A1]
           | STRING             // quoted: pin["1"]
```

**Resolution order** (when evaluating a bare identifier like `R1`):

1. Built-in keywords: `true`, `false`, `null`
2. Bindings: innermost scope first, then outer scopes
3. Enum registry: if field expects an enum, check for match
4. Batch-placed entities: records placed earlier in this `.ops` file
5. Existing document entities: records already in the document

`$`-prefixed identifiers always resolve against the result table and built-in
aliases (`$last`, `$self`, `$sheet`).

### 5.4 Coords (Tuples)

Coords are 2D points constructed with tuple syntax:

```
(x_expr, y_expr)
```

Each component is a scalar expression (dim, number, or reference to a dim).

```
location: (1000, 800)                                  // literal
location: (U1.location.x + 400, U1.location.y)        // expressions
location: (20mm, 0mm)                                  // with units
location: ($r1.location.x + 300, $r1.location.y)      // op refs
```

**Single-element tuples:** `(expr)` is parenthesized grouping, not a 1-tuple.
Coords always have exactly 2 elements.

**Coord from reference:** When a field expects a Coord and the expression evaluates
to a Coord (e.g., `U1.location`, `$r1.location`, `U1.pin[1].location`), the bare
reference is valid without tuple syntax:

```
// These are equivalent when the reference resolves to a Coord:
start: U1.pad[1].location
start: (U1.pad[1].location.x, U1.pad[1].location.y)
```

The type checker validates that the resolved type matches the expected field type.

### 5.5 Arrays

Arrays use bracket syntax:

```
[expr, expr, ...]
```

```
points: [U1.pin[14].location, R1.pin[1].location]
points: [(100, 200), (300, 200), (300, 400)]
pins: [
    { designator: "1", electrical: passive }
    { designator: "2", electrical: passive }
]
```

Elements can be any expression. Element types should be homogeneous (enforced
by the type checker, not the parser).

### 5.6 Nested Objects and Spread

Objects use brace syntax with optional bindings and spread:

```
{ [bindings...] [spread...] key: expr, key: expr, ... }
```

Objects appear as op bodies, array elements (pin specs), and nested fields.

#### Basic objects

```
// Op body
add_component { designator: "R1", value: "10K" }

// Array of objects
pins: [
    { designator: "1", electrical: passive, offset: (-50, 0) }
    { designator: "2", electrical: passive, offset: (50, 0) }
]

// Nested object
footprint: { model_name: "0805", map: [{ pin: "1", pad: "1" }] }
```

#### Spread operator (`...`)

The spread operator `...expr` expands an object expression's fields into the
enclosing object. The expression must evaluate to an object.

```
base ={ lib_reference: "R", value: "10K", footprint: "0805" }

add_component {
    ...base                          // expands to lib_reference, value, footprint
    designator: "R1"
    location: (1000, 800)
}
```

**Last-wins rule:** Explicit fields override spread fields. Later spreads override
earlier spreads. This enables an override pattern:

```
defaults ={ value: "10K", footprint: "0805" }

add_component {
    ...defaults
    value: "20K"       // overrides defaults.value, keeps defaults.footprint
    designator: "R1"
}
```

**Multiple spreads:**

```
physical ={ footprint: "0805", layer: Top }
electrical ={ value: "10K", lib_reference: "R" }

add_component {
    ...physical
    ...electrical
    designator: "R1"
    location: (1000, 800)
}
```

**Spread with bindings:**

```
passive_pin ={ electrical: passive, length: 25 }

add_component {
    designator: "R1"
    pins: [
        { designator: "1", ...passive_pin, offset: (-50, 0) }
        { designator: "2", ...passive_pin, offset: (50, 0) }
    ]
}
```

**Spread sources:** The expression after `...` can be:
- A bound object: `...pin_defaults`
- An op result field: `...$r1.fields`
- Any expression that evaluates to an object

**Spread does NOT work in arrays.** `[...arr1, ...arr2]` is not supported.
Use separate array elements.


## 6. Selector Language (AQL)

Ops that target existing records (`edit`, `remove`, `query`) use the Altium Query
Language for selection. AQL is a separate grammar from expressions, activated by
the op name.

See `docs/query-lang.md` for the full AQL reference. This section summarizes the
syntax and documents the extensions for op result integration.

### 6.1 Pattern Selectors

Quick lookup patterns:

| Pattern | Matches | Example |
|---------|---------|---------|
| `DESIG` | Exact designator | `U1` |
| `PREFIX*` | Wildcard suffix | `R*` |
| `PREFIX?` | Single char wildcard | `U?` |
| `PREFIX??` | Two char wildcard | `C??` |
| `~NET` | Net name | `~VCC` |
| `@VALUE` | Component value | `@10K` |
| `%PART` | Library part number | `%LM358` |
| `COMP:PIN` | Component pin | `U1:VCC` |

**Note:** Part number prefix is `%` (not `$` as in AQL v1) to avoid conflict
with `$` op result references.

### 6.2 Type Selectors

Select by record type (case-insensitive):

```
component   pin   wire   bus   port   power   label   netlabel
junction    sheet   parameter   line   arc   text   polygon
rectangle   pad   via   track   fill   region   rule   net
```

### 6.3 Attribute Selectors

Filter by field values:

```
component[value=10K]            // exact match
component[footprint*=0603]      // contains
component[designator^=R]        // starts with
pin[electrical=power]           // enum match
track[width>=10mil]             // comparison with units
component[x>1000][y<2000]       // chained (AND)
component[designator=/^U\d+$/]  // regex
```

**Operators:** `=` `!=` `*=` `^=` `$=` `~=` `>` `<` `>=` `<=`

**Values in selectors:** Strings (quoted or bare), numbers, booleans, dims
with units (`10mil`, `2.54mm`), regexes (`/pattern/`).

### 6.4 Pseudo-classes

```
pin:power          pin:passive          pin:input
pin:output         pin:io               pin:hiz
net:power          net:ground           net:signal
component:placed   component:locked     component:virtual
:selected          :visible             :on-grid
```

### 6.5 Combinators

| Combinator | Meaning | Example |
|------------|---------|---------|
| (space) | Descendant | `component pin` |
| `>` | Direct child | `U1 > pin` |
| `+` | Adjacent sibling | `wire + junction` |
| `~` | General sibling | `component ~ component` |
| `,` | Union (OR) | `R*, C*` |

### 6.6 Logical Operators

```
component AND [value=10K] AND :placed
R* OR C* OR L*
component AND NOT :virtual
```

Precedence (high to low): `NOT` → `AND` → `OR` → `,`

### 6.7 Op Result References in Selectors

AQL is extended with `$ref` selectors that match entities from prior op results:

```
$name       // all entities from let name = op ...
$last       // all entities from most recent op
```

These integrate with attribute selectors and combinators:

```
caps =query C*
edit $caps[value>100nF] { footprint: "0805" }
remove $caps[value<10nF]

ics =query component[designator^=U]
edit $ics > pin:power { ... }
```

`$name` in selector position resolves to the entity set from that op's `OpResult.refs`.
It behaves like a type selector — you can chain attribute selectors, pseudo-classes,
and combinators after it.


## 7. Type System

The parser produces a typed AST based on syntactic delimiters. The type checker
validates that AST types match expected field types. Every AST node carries a
`Span` (§13.4).

### 7.1 Scalar Types

| AST type | Syntax | Maps to (Rust) |
|----------|--------|----------------|
| String | `"..."` | `String` |
| TemplateString | `` `...{expr}...` `` | `String` (after interpolation) |
| Integer | `42` | `i32` |
| Float | `3.14` | `f64` |
| Dim | `20mm` | `Coord` (single-axis) |
| Bool | `true` | `bool` |
| Null | `null` | `Option::None` |
| Color | `#FF0000` | `Color` |
| Ident | `passive` | Resolved by type checker |

### 7.2 Coord Type

| AST type | Syntax | Maps to (Rust) |
|----------|--------|----------------|
| Coord | `(x, y)` | `CoordPoint` |

Coords are always 2-element tuples. Each element is a scalar expression that
resolves to a dimensional value.

A reference that resolves to a `CoordPoint` is valid in Coord position:
`U1.location` produces a Coord without tuple syntax.

### 7.3 Unit Suffixes

Dimensional scalars carry a unit suffix that determines conversion to internal units:

| Suffix | Meaning | Internal units per 1 |
|--------|---------|---------------------|
| *(none)* | Mils (default) | 10,000 |
| `mil` | Mils (explicit) | 10,000 |
| `mm` | Millimeters | 393,701 |
| `in` | Inches | 10,000,000 |
| `dxp` | DXP units (10 mils) | 100,000 |
| `raw` | Raw internal units | 1 |

Bare numbers in dimensional fields default to mils. This is resolved at type-check
time, not parse time.

### 7.4 Enum Resolution

Bare identifiers in typed fields are resolved against the field's expected enum type.
Resolution is **case-insensitive** and **underscore-insensitive**.

```
electrical: passive          // PinElectricalType::Passive
electrical: open_collector   // PinElectricalType::OpenCollector
style: bar                   // PowerObjectStyle::Bar
style: gnd_power             // PowerObjectStyle::GndPower
layer: Top                   // V6Layer::TopLayer
shape: round                 // PadShape::Round
orientation: 0               // RotationBy90::Rotate0
color: red                   // Color::from_name("red")
```

If the field does not expect an enum, a bare identifier is resolved as a
binding first, then as a path expression (document reference). If it's none of
those, the type checker reports an error.

### 7.5 Type Coercion Rules

The type checker applies these coercions at field boundaries:

| Field expects | Expression produces | Coercion |
|---------------|-------------------|----------|
| Dim | Integer | Apply default unit (mils) |
| Dim | Float | Apply default unit (mils) |
| Coord | 2-tuple of dims | Construct CoordPoint |
| Coord | Path → CoordPoint | Pass through |
| String | *(no coercion)* | Must be quoted string |
| Enum | Ident | Look up in enum registry |
| Color | `#RRGGBB` | Parse hex |
| Color | Ident | Named color lookup |


## 8. Reference & Result Model

Every op returns a structured `OpResult`. Results are stored in a result table
keyed by name (from `let name = op`) and accessible via `$` references in
subsequent statements.

### 8.1 `$` Aliases

| Alias | Resolves to | Available |
|-------|-------------|-----------|
| `$last` | Most recent op's result | Always (after first op) |
| `$self` | Current record being edited | Inside `edit` patch body |
| `$sheet` | Sheet-level properties | Always |
| `$NAME` | Result of `let NAME = op ...` | After that op executes |

`$` references are recognized in both expression context and selector context.

### 8.2 Document References

Bare identifiers (without `$`) that are not bindings and not enums resolve
to document entities:

```
U1              // component with designator "U1"
R1.pin[2]       // pin 2 of R1
U1.location.x   // x-coordinate of U1
```

Resolution searches:
1. Batch-placed entities (placed earlier in this `.ops` file)
2. Existing document entities (already in the document before this file)

This means a later op can reference records created by earlier ops using their
designator, without needing `$` tags.

### 8.3 OpResult

Every op returns:

```rust
pub struct OpResult {
    pub kind: String,                            // "add_component", "edit", etc.
    pub ref_: Option<EntityRef>,                 // primary entity
    pub refs: Vec<EntityRef>,                    // all entities (matches/children)
    pub fields: IndexMap<String, ResolvedValue>, // typed outputs
    pub warnings: Vec<String>,
}
```

**Navigable fields per op type:**

| Op | `.ref` | `.refs` | `.location` | `.count` | `.first` / `.last` |
|----|--------|---------|-------------|----------|-------------------|
| `add_component` | created component | children (pins, etc.) | placement coord | 1 | — |
| `add_wire` | created wire | wire points | — | 1 | — |
| `add_track` | created track | — | start coord | 1 | — |
| `place` | placed entity | — | resulting location/anchor | 1 | — |
| `edit` | — | modified entities | — | modified count | first/last modified |
| `remove` | — | removed entities | — | removed count | first/last removed |
| `query` | — | matched entities | — | match count | first/last match |

**EntityRef** carries domain, entity type, internal ID, and display path:
```rust
pub struct EntityRef {
    pub domain: Domain,          // SchDoc, SchLib, PcbDoc, PcbLib
    pub entity_type: EntityType, // component, pin, track, ...
    pub id: String,              // internal ID token
    pub display_path: String,    // "R1", "R1.pin[1]", etc.
}
```

Navigating into an EntityRef resolves its fields from the (possibly mutated)
document state.

### 8.4 Resolution Order

When the expression evaluator encounters a path:

1. **`$` prefix → result table.** `$r1` looks up the result of `r1 =...`,
   `$last` returns most recent result, `$self` returns current edit target,
   `$sheet` returns sheet metadata.
2. **Bare identifier → let, then enum, then document.** `spacing` checks
   bindings first (innermost scope outward), then enum registry (if
   field is typed), then batch-placed entities, then existing document.


## 9. Lexical Rules

### 9.1 Tokens

The lexer produces these token types:

| Token | Pattern | Examples |
|-------|---------|---------|
| `IDENT` | `[a-zA-Z_][a-zA-Z0-9_]*` | `add_component`, `passive`, `R1` |
| `STRING` | `"` (escape \| [^"\\])* `"` | `"R1"`, `"10K"` |
| `TEMPLATE` | `` ` `` { char \| `{` expr `}` } `` ` `` | `` `got {U1.value}` `` |
| `INTEGER` | `-`? `[0-9]+` | `42`, `-5`, `0` |
| `FLOAT` | `-`? `[0-9]+` `.` `[0-9]+` | `3.14`, `-0.5` |
| `DIM` | (`INTEGER` \| `FLOAT`) `UNIT` | `20mm`, `2.54mm`, `100mil` |
| `COLOR` | `#` `[0-9a-fA-F]{6}` | `#FF0000`, `#00ff00` |
| `DOLLAR_IDENT` | `$` `IDENT` | `$r1`, `$last`, `$self` |
| `UNIT` | `mm` \| `mil` \| `in` \| `dxp` \| `raw` | |
| `DOTDOTDOT` | `...` | Spread operator |
| Punctuation | `: , . + - * / ( ) [ ] { }` | |
| Comparison | `==` `!=` `>=` `<=` `>` `<` | Only in `assert` |
| Keywords | `true` `false` `null` `assert` | |
| Noise | `let` `;` | Optional, ignored (§9.5) |
| AQL keywords | `AND` `OR` `NOT` | Only in selector context |
| Line comment | `//` ... newline | |
| Block comment | `/*` ... `*/` | Nesting allowed |

**Lexer disambiguation:**

- `#` followed by 6 hex digits → `COLOR`. `#` is only used for colors.
- Number immediately followed by a unit suffix (no whitespace) → `DIM`.
  `20 mm` is `INTEGER` `IDENT`, not `DIM`.
- `$` followed by identifier → `DOLLAR_IDENT`. Always.
- `-` is unary negation in prefix position, subtraction in infix position.
- `...` (three dots) is always the spread operator.
- `==`, `!=`, `>=`, `<=` are always comparison (assert context). Single `=` only
  appears inside AQL attribute selectors.

### 9.2 Separators

Values, fields, and array elements are separated by:

- **Comma** (`,`) — on the same line or across lines
- **Newline** — implicit separator when not inside `()`/`[]`

Commas are required between values on the same line. Newlines are sufficient
across lines. Trailing commas are always allowed.

```
// All valid:
{ a: 1, b: 2, c: 3 }         // inline, commas
{                              // block, newlines
    a: 1
    b: 2
    c: 3
}
{ a: 1, b: 2                  // mixed
  c: 3 }
{ a: 1, b: 2, c: 3, }        // trailing comma
```

**Newline suppression:** Newlines inside `()`, `[]`, and `{}` do NOT act as
separators for the *enclosing* context. This allows multi-line arrays, tuples,
and objects without continuation markers.

### 9.3 Comments

Line comments start with `//` and extend to end of line.
Block comments use `/* ... */` and may span multiple lines. Block comments nest.

```
// Line comment
add_component {    // inline comment
    designator: "R1"
    /* This field is temporarily disabled:
    value: "10K"
    */
}

/* Nesting works:
add_component {
    /* inner comment */
    designator: "R2"
}
*/
```

### 9.4 Whitespace

Spaces and tabs are insignificant (not indentation-sensitive). Newlines are
significant only as separators (§9.2).

### 9.5 Optional Noise Tokens (LLM Tolerance)

Since this language is primarily generated by LLMs, the parser accepts certain
tokens that have no semantic meaning. These are silently ignored, making the
parser tolerant of common hallucinations from models trained on other languages:

| Token | Where accepted | Why LLMs emit it |
|-------|---------------|-------------------|
| `;` | After any statement or field | C/Rust/JS muscle memory |
| `let` | Before a binding (`let x = ...`) | Rust/JS/Python habit |
| Trailing `,` | After last element in `[]`, `{}` | Already valid, but worth noting |
| Extra `,` between lines | Between array/object items | Defensive comma usage |
| `()` around assert condition | `assert (x == 1)` | C/Java `assert()` pattern |

**All of the following are equivalent:**

```
// Minimal (canonical)
spacing = 300
r1 = add_component { designator: "R1" }

// With noise tokens (also valid)
let spacing = 300;
let r1 = add_component { designator: "R1", };
```

The parser strips noise tokens during lexing/parsing. They never appear in the AST.

**Not accepted as noise** (would create ambiguity):
- `:` after op names (`add_component: { ... }` — conflicts with field syntax)
- `=` for field assignments (`designator = "R1"` — conflicts with bindings)
- `var`/`const`/`def` before bindings (only `let` is accepted as noise)


## 10. Formal Grammar

```ebnf
(* ================================================================ *)
(* File structure                                                    *)
(* ================================================================ *)

file            = { statement [";"] } ;

statement       = binding
                | assert_stmt
                | op ;

binding         = ["let"] IDENT "=" ( op | expr ) ;

assert_stmt     = "assert" ["("] assert_cond [")"] [ "," ( STRING | template ) ] ;
assert_cond     = expr [ compare_op expr ] ;
(* Parentheses around condition are optional noise — both forms are identical *)
compare_op      = "==" | "!=" | ">" | "<" | ">=" | "<=" ;

(* Template string — backtick-delimited with {expr} interpolation *)
template        = '`' { char | '{' expr '}' | '{{' | '}}' } '`' ;

op              = create_op
                | place_op
                | edit_op
                | remove_op
                | query_op ;

create_op       = IDENT [ target ] object ;
place_op        = "place" target object ;
edit_op         = "edit" selector object ;
remove_op       = "remove" selector ;
query_op        = "query" selector ;

target          = path_expr ;                   (* parent ref: $r1, $r1.field *)

(* ================================================================ *)
(* AQL selector (separate grammar, see §6)                          *)
(* Parsed after edit/remove/query until '{' or end-of-statement.    *)
(* ================================================================ *)

selector        = aql_or ;

aql_or          = aql_and { ("OR" | ",") aql_and } ;
aql_and         = aql_not { "AND" aql_not } ;
aql_not         = "NOT" aql_not | aql_compound ;

aql_compound    = aql_simple { aql_filter } { aql_combinator } ;
aql_simple      = DOLLAR_IDENT                      (* op ref: $caps *)
                | IDENT [ "*" | "?" | "??" ]        (* designator pattern *)
                | "~" IDENT                          (* net: ~VCC *)
                | "@" aql_value                      (* value: @10K *)
                | "%" IDENT [ "*" ]                  (* part: %LM358 *)
                | IDENT ":" IDENT                    (* pin: U1:VCC *)
                | TYPE_KEYWORD ;                     (* type: component *)

aql_filter      = "[" IDENT [ "." IDENT ] aql_op aql_value "]"
                | ":" IDENT ;                        (* pseudo-class *)

aql_op          = "=" | "!=" | "*=" | "^=" | "$=" | "~="
                | ">" | "<" | ">=" | "<=" ;

aql_value       = STRING | INTEGER | FLOAT | DIM | BOOL
                | IDENT                              (* bare value *)
                | "/" { regex_char } "/" ;            (* regex *)

aql_combinator  = ">" aql_compound                   (* child *)
                | "+" aql_compound                   (* adjacent *)
                | "~" aql_compound                   (* sibling *)
                | aql_compound ;                     (* descendant: space *)

TYPE_KEYWORD    = "component" | "pin" | "wire" | "bus" | "port"
                | "power" | "label" | "netlabel" | "junction"
                | "sheet" | "parameter" | "line" | "arc" | "text"
                | "polygon" | "rectangle" | "pad" | "via" | "track"
                | "fill" | "region" | "rule" | "net" ;

(* ================================================================ *)
(* Expression (Pratt parser, every value position)                  *)
(* ================================================================ *)

expr            = pratt_expr ;

(* Pratt with binding powers — see §5.2 for precedence table       *)
pratt_expr      = prefix_expr { infix_op pratt_expr } ;

prefix_expr     = STRING
                | template                              (* `text {expr}` *)
                | INTEGER
                | FLOAT
                | DIM
                | COLOR
                | BOOL
                | "null"
                | DOLLAR_IDENT path_tail            (* $ref.field *)
                | IDENT path_tail                   (* R1.pin[1].x or let var *)
                | "-" pratt_expr                    (* unary negate *)
                | "(" expr "," expr ")"             (* coord tuple *)
                | "(" expr ")"                      (* grouping *)
                | "[" [ expr_list ] "]"             (* array *)
                | object ;                          (* nested object *)

infix_op        = "+" | "-" | "*" | "/"             (* arithmetic *)
                | "." IDENT                          (* field access *)
                | "[" expr "]" ;                     (* index access *)

path_tail       = { "." IDENT | "[" expr "]" } ;

expr_list       = expr { sep expr } ;

(* ================================================================ *)
(* Object / fields (with bindings and spread)                       *)
(* ================================================================ *)

object          = "{" [ object_body ] "}" ;
object_body     = object_item { sep object_item } ;
object_item     = binding                            (* block-scoped binding *)
                | spread                             (* ...expr *)
                | field ;                            (* key: value *)
spread          = "..." expr ;
field           = key ":" expr ;
key             = IDENT { "." IDENT } ;             (* dotted keys *)

(* ================================================================ *)
(* Separators                                                        *)
(* ================================================================ *)

sep             = "," | NEWLINE ;                    (* one or more *)
```


## 11. Complete Examples

### Example 1: Place a resistor next to an existing IC

```
r1 =add_component {
    designator: "R1", lib_reference: "R", value: "10K", footprint: "0805"
    location: (U1.location.x + 400, U1.location.y)
    orientation: 0
    pins: [
        { designator: "1", electrical: passive, offset: (-50, 0), length: 25 }
        { designator: "2", electrical: passive, offset: (50, 0), length: 25 }
    ]
}
```

### Example 1b: Relative placement without explicit coordinates

```
comp = add_component { designator: "U1", lib_reference: "IC" }
body = add_rectangle $comp { from: (-300mil, -150mil), to: (300mil, 150mil) }
p1 = add_pin $comp { designator: "1" }
p2 = add_pin $comp { designator: "2" }
p3 = add_pin $comp { designator: "3" }

place_defaults = { gap: 100mil, side: outside, orientation: auto }

place $p1 { on: $body.top, at: start }
place $p2 { ...place_defaults, on: $body.top, after: $p1 }
place $p3 { on: $body.left, at: center }
```

### Example 2: Wire from one pin to another

```
add_wire { points: [U1.pin[14].location, R1.pin[1].location] }
```

### Example 3: PCB track with mm units

```
add_track {
    start: U1.pad[1].location
    end: (U1.pad[1].location.x + 2.54mm, U1.pad[1].location.y)
    width: 0.254mm, layer: Top, net: "VCC"
}
```

### Example 4: Let bindings + spread for deduplication

```
passive_pin ={ electrical: passive, length: 25 }
spacing = 300

add_component {
    designator: "R1", lib_reference: "R", value: "10K"
    location: (1000, 800)
    pins: [
        { designator: "1", ...passive_pin, offset: (-50, 0) }
        { designator: "2", ...passive_pin, offset: (50, 0) }
    ]
}

add_component {
    designator: "R2", lib_reference: "R", value: "10K"
    location: ($last.location.x + spacing, $last.location.y)
    pins: [
        { designator: "1", ...passive_pin, offset: (-50, 0) }
        { designator: "2", ...passive_pin, offset: (50, 0) }
    ]
}

add_wire { points: [R1.pin[2].location, R2.pin[1].location] }
```

### Example 5: Assert before mutation (anti-hallucination)

```
// Verify the IC is what we think it is
assert U1.lib_reference == "LM358", `expected LM358 at U1, got {U1.lib_reference}`
assert U1.pin[14], "pin 14 does not exist on U1"
assert U1.pin[14].electrical == power,
    `pin 14 is {U1.pin[14].electrical}, expected power`

// Safe to wire now
add_wire { points: [U1.pin[14].location, (U1.pin[14].location.x, U1.pin[14].location.y + 100)] }
```

### Example 6: Edit existing components

```
// Simple value edit
edit component[designator=R1] {
    value: "20K"
}

// Relative move using $self
edit R* {
    location.x: $self.location.x + 200
}

// Edit components in a region
edit component[x>=1000][x<=2000][y>=500][y<=1500] {
    value: "DNP"
}
```

### Example 7: Query + edit pipeline

```
// Find all small capacitors
small_caps =query C*[value<100nF]

// Assert we found some
assert $small_caps.count > 0, "no small caps found"

// Change their footprint
edit $small_caps { footprint: "0402" }

// Find all remaining large caps
large_caps =query C*[value>=100nF]

// Change those too
edit $large_caps { footprint: "0805" }
```

### Example 8: Complex reuse with object spread

```
smd_resistor ={
    lib_reference: "R"
    footprint: "0805"
}

passive_pin ={ electrical: passive, length: 25 }

two_pin_passive =[
    { designator: "1", ...passive_pin, offset: (-50, 0) }
    { designator: "2", ...passive_pin, offset: (50, 0) }
]

add_component {
    ...smd_resistor
    designator: "R1", value: "10K"
    location: (1000, 800)
    pins: two_pin_passive
}

add_component {
    ...smd_resistor
    designator: "R2", value: "4.7K"
    location: ($last.location.x + 300, $last.location.y)
    pins: two_pin_passive
}

add_component {
    ...smd_resistor
    designator: "R3", value: "100K"
    location: ($last.location.x + 300, $last.location.y)
    pins: two_pin_passive
}
```

### Example 9: Power port + net label with block-scoped let

```
add_power_port {
    let pin_loc = U1.pin[14].location
    name: "VCC", style: bar
    location: (pin_loc.x, pin_loc.y + 100)
    orientation: 90
}

add_net_label { name: "DATA_BUS", location: (500, 1200), orientation: 0 }
```

### Example 10: Complex wiring with named ops

```
ic =add_component {
    designator: "U1", lib_reference: "LM358", value: "LM358"
    location: (2000, 1500)
    pins: [
        { designator: "1", electrical: output, offset: (50, 20) }
        { designator: "2", electrical: input, offset: (-50, 20) }
        { designator: "3", electrical: input, offset: (-50, -20) }
        { designator: "4", electrical: power, offset: (0, -50) }
        { designator: "8", electrical: power, offset: (0, 50) }
    ]
}

vcc =add_power_port {
    name: "VCC", style: bar
    location: ($ic.pin[8].location.x, $ic.pin[8].location.y + 100)
    orientation: 90
}

add_power_port {
    name: "GND", style: gnd_power
    location: ($ic.pin[4].location.x, $ic.pin[4].location.y - 100)
    orientation: 270
}

add_wire { points: [$ic.pin[8].location, $vcc.location] }
add_wire { points: [$ic.pin[4].location, $last.location] }
```

### Example 11: Remove and verify

```
// Assert before removing
assert R*[value<100], "no low-value resistors to remove"

// Remove low-value resistors
remove R*[value<100]

// Verify they're gone
remaining =query R*
assert $remaining.count < 10, `expected fewer than 10 remaining, got {$remaining.count}`
```

### Example 12: SchLib symbol definition with reuse

```
passive_pin ={ electrical: passive, name: "", length: 25 }

r = add_component {
    lib_reference: "R"
    designator: "R?"
    value: ""
    pins: [
        { designator: "1", ...passive_pin }
        { designator: "2", ...passive_pin }
    ]
    footprint: { model_name: "0805", map: [{ pin: "1", pad: "1" }, { pin: "2", pad: "2" }] }
}
```


## 12. CLI Integration

```bash
# Apply ops file to a document
altium ops apply library.SchLib --spec add-components.ops

# Dry run (parse, resolve refs, type-check, but don't save)
altium ops apply library.SchLib --spec add-components.ops --dry-run

# Report results as JSON (full result table)
altium ops apply library.SchLib --spec add-components.ops --report-json

# Schema introspection for agents
altium schema add_component             # field table, types, enums
altium schema add_component --json      # JSON Schema output
altium schema --list                    # all available operations
```


## 13. Implementation Notes

### 13.1 Span Model

Every AST node carries a `Span` recording its source location. Spans are
byte-offset pairs into the original source text:

```rust
/// Byte-offset range in source text. Used by every AST node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}
```

**Spanned wrapper** for attaching spans to any type:

```rust
/// A value with its source span attached.
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}
```

**AST nodes that carry spans:**

```rust
pub struct OpsFile {
    pub statements: Vec<Spanned<Statement>>,
}

pub enum Statement {
    Binding(Binding),
    Assert(AssertStmt),
    Op(Op),
}

pub struct Binding {
    pub name: Spanned<String>,
    pub value: Spanned<BindingValue>,
}

pub enum BindingValue {
    Expr(Expr),    // x = expression (lazy)
    Op(Op),        // r1 = add_component { ... } (op result)
}

pub struct AssertStmt {
    pub condition: Spanned<AssertCondition>,
    pub message: Option<Spanned<Expr>>,  // String or TemplateString
}

pub enum AssertCondition {
    Existence(Spanned<Expr>),
    Comparison {
        left: Spanned<Expr>,
        op: Spanned<CompareOp>,
        right: Spanned<Expr>,
    },
}

/// Backtick-delimited template string with `{expr}` interpolation.
/// Used in assert messages and as a general-purpose expression.
pub struct TemplateString {
    pub parts: Vec<Spanned<TemplatePart>>,
}

pub enum TemplatePart {
    Literal(String),
    Interpolation(Spanned<Expr>),
}

pub struct Op {
    pub name: Spanned<String>,
    pub target: Option<Spanned<Expr>>,       // parent ref for create-child ops
    pub selector: Option<Spanned<Selector>>, // AQL for edit/remove/query
    pub body: Option<Spanned<Object>>,
}

pub struct Object {
    pub items: Vec<Spanned<ObjectItem>>,
}

pub enum ObjectItem {
    Binding(Binding),               // block-scoped: name = expr
    Spread(Spanned<Expr>),
    Field(Field),                   // key: value
}

pub struct Field {
    pub key: Spanned<Key>,
    pub value: Spanned<Expr>,
}

pub enum Expr {
    String(String),
    TemplateString(TemplateString),
    Integer(i32),
    Float(f64),
    Dim(f64, Unit),
    Color(u8, u8, u8),
    Bool(bool),
    Null,
    Ident(String),
    DollarIdent(String),
    Path(Box<Spanned<Expr>>, Spanned<String>),       // expr.field
    Index(Box<Spanned<Expr>>, Box<Spanned<Expr>>),    // expr[key]
    BinOp(Box<Spanned<Expr>>, Spanned<BinOp>, Box<Spanned<Expr>>),
    UnaryNeg(Box<Spanned<Expr>>),
    Tuple(Box<Spanned<Expr>>, Box<Spanned<Expr>>),    // (x, y)
    Array(Vec<Spanned<Expr>>),
    Object(Object),
}

pub enum BinOp { Add, Sub, Mul, Div }
pub enum CompareOp { Eq, Ne, Gt, Lt, Ge, Le }
pub enum Unit { Mil, Mm, Inch, Dxp, Raw }
```

**Span propagation rules:**

- The parser sets spans during construction — every `Spanned<T>` records the
  byte range of the source text that produced it.
- Composite nodes span from the start of their first token to the end of their
  last token. E.g., `(U1.x + 400, U1.y)` spans from `(` to `)`.
- Error messages reference spans to produce `rustc`-style diagnostics (§13.4).

### 13.2 Two-Pass Architecture

```
Pass 1: Parse (syntax only)
  ├── Lexer: source text → token stream (each token has Span)
  ├── Root parser: tokens → Vec<Spanned<Statement>> (recursive descent)
  ├── Expression parser: Pratt parser for value positions
  ├── Selector parser: AQL parser for selector positions
  └── Output: fully-spanned AST
  Errors: syntax errors with exact spans

Pass 2: Type-check + Evaluate (schema + document aware)
  ├── Scope stack for bindings (push on block entry, pop on exit)
  ├── For each statement in sequence:
  │   ├── Binding: store expr AST, bind name in current scope (lazy)
  │   ├── Assert: evaluate condition, halt if false (with formatted message)
  │   ├── Op:
  │   │   ├── Resolve $refs from result table
  │   │   ├── Resolve bare idents (let → enum → document)
  │   │   ├── Expand spread operators
  │   │   ├── Type-check: field types vs expression types
  │   │   ├── Coerce: bare numbers → mils in dim fields, etc.
  │   │   ├── Lower to low-level ops (see ops-design.md §lowering)
  │   │   ├── Execute low-level ops against document
  │   │   └── Capture OpResult in result table
  └── Output: modified document + result table
  Errors: type errors, unknown references, enum mismatches, assert failures
          (all with Span → source position)
```

Pass 1 knows types from **syntax alone**: `"..."` → String, `20mm` → Dim,
`(a, b)` → Coord, `[...]` → Array, `passive` → Ident (enum candidate),
`...expr` → Spread, `name =` → binding. No schema needed.

Pass 2 validates against schema and resolves semantic ambiguity.

### 13.3 Pratt Parser for Expressions

Hand-rolled Pratt parser (~200 lines). Benefits:

- **Domain-specific error messages.** "Unknown unit 'mx' at column 15, did you
  mean 'mm'?" — agents parse these errors.
- **Exact source spans.** Every AST node carries `Span` (§13.1).
- **Simple grammar.** 4 binding power levels, unary prefix, field/index postfix.
- **No dependencies.** The parser is the most stable part of the codebase.

The outer format (ops, fields, selectors, let, assert) is a recursive descent
parser. The Pratt parser is invoked when a value position is reached.

### 13.4 Error Reporting Strategy

Errors reference spans and provide actionable context. Because every AST node
carries a `Span`, errors from any phase (parse, type-check, evaluation) can
point to the exact source location:

```
error[E0201]: unknown unit suffix 'mx'
  --> add-resistors.ops:3:15
   |
 3 |     width: 20mx
   |               ^^ unknown unit
   |
   = help: valid units: mm, mil, in, dxp, raw

error[E0301]: type mismatch in field 'electrical'
  --> add-resistors.ops:7:26
   |
 7 |     { designator: "1", electrical: "passive" }
   |                                    ^^^^^^^^^ expected enum, got string
   |
   = help: use bare identifier: electrical: passive

error[E0401]: unresolved reference 'R3'
  --> add-resistors.ops:12:16
   |
12 |     location: (R3.location.x + 400, R3.location.y)
   |                ^^ not found
   |
   = note: available designators: R1, R2
   = note: available op bindings: $r1

error[E0501]: assertion failed
  --> add-resistors.ops:2:1
   |
 2 | assert U1.lib_reference == "LM358", `expected LM358 at U1, got {U1.lib_reference}`
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: left:  "LM7805"
   = note: right: "LM358"
   = note: expected LM358 at U1, got LM7805

error[E0601]: spread source is not an object
  --> add-resistors.ops:5:5
   |
 5 |     ...pin_defaults
   |     ^^^^^^^^^^^^^^^ expected object, got array
   |
   = note: pin_defaults was bound at add-resistors.ops:1:5
```


## 14. Scope Boundaries

### What We Build

- Every value position is an expression (Pratt parser)
- Dimensional scalars with unit suffixes (mil, mm, in, dxp, raw)
- Coords as `(x, y)` tuples
- Enum resolution (context-dependent, case-insensitive, underscore-insensitive)
- Path expressions for document, binding, and op result navigation
- `$` references: `$last`, `$self`, `$sheet`, `$name` (from `let name = op ...`)
- AQL selectors in `edit`/`remove`/`query` ops (with `$ref` extension)
- Structured `OpResult` from every op, feeding forward via result table
- Bindings (`name = expr`): file-level and block-scoped, with object values for spread
- Object spread (`...expr`) for deduplication
- `assert` with comparison operators and message interpolation
- `Span` on every AST node for precise diagnostics across all phases
- Line comments (`//`) and nesting block comments (`/* */`)
- Schema introspection for agent discovery
- Actionable error messages with source spans

### What We Don't Build

- **No control flow.** No if/else, no loops. Generate N ops for N placements.
- **No functions.** No sin(), sqrt(), min(). Complex geometry pre-computed by agent.
- **Template strings are not regular strings.** Template strings (`` `...` ``) support
  `{expr}` interpolation. Regular strings (`"..."`) do not. Both evaluate to `String`.
- **No Coord arithmetic.** No `point + point`. Compose via `(x_expr, y_expr)`.
- **No nested ops.** Each op is top-level. No sub-blocks or op-inside-op.
- **No array spread.** `[...a, ...b]` is not supported. Arrays are constructed
  element-by-element or bound whole via binding.

The expression language is deliberately minimal: **paths + arithmetic + units +
bindings + spread**. Anything more complex, the agent computes before generating
the `.ops` file.

### Future Extensions

These are explicitly deferred — not in v0.2:

- **`$param` / CLI variables.** Template parameters injected via `--var key=value`
  for human-reviewed, reusable `.ops` templates.
- **`params` block.** Declare expected parameters with types and defaults at file top.
- **Python bindings.** PyO3 wrappers around the ops layer for complex generation,
  conditional logic, and integration with external tools. The Python API would call
  the same Rust ops — `altium.add_component()` does the same thing as
  `add_component { }` in `.ops`.
