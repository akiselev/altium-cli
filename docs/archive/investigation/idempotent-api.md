# Declarative Spec Format — Design Notes

## Motivation

LLM agents should be able to define entire Altium projects declaratively, iterate
on the definition, and rebuild the Altium files from scratch each time. Separate
validation agents should then be able to run measurement/query ops against the
built files to verify correctness (pad pitch, clearance, footprint dimensions, etc.).

This gives us three distinct layers:

```
┌─────────────────────────────────────────────────────────┐
│  .spec files  (declarative: WHAT the project contains)  │   ← LLM authors this
├─────────────────────────────────────────────────────────┤
│  .ops files   (imperative: HOW to mutate a document)    │   ← escape hatch for patches
├─────────────────────────────────────────────────────────┤
│  query/measure ops  (validation: IS it correct?)        │   ← validation agents use this
└─────────────────────────────────────────────────────────┘
```

The `.spec` format sidesteps idempotency entirely — you always build from
scratch, so there's no duplicate-creation problem.

## Why Not Just Use .ops From Scratch?

The `.ops` format is imperative: `add_component`, `add_pin`, `edit_record`. Even
when run from scratch, it has quirks that make it awkward as a source-of-truth:

1. **Ordering matters**: ops execute sequentially; reordering can break `$ref` chains
2. **Imperative noise**: `add_` prefix, `component_ref` back-references, `opid` plumbing
3. **Mixed concerns**: creation, mutation, and queries interleaved in one file
4. **No structure**: a flat list of operations, not a tree matching the document

A `.spec` file describes the *shape* of the document, not the *steps* to build it.
The compiler decides how to sequence the operations.

## .spec Format Design

### Core Idea

A `.spec` file is a **tree of declarations** that mirrors the document structure.
Each declaration names an entity and specifies its properties. The compiler
lowers the tree into a sequence of ops that build the document from scratch.

### Syntax

Reuses the existing ops lexer (same literals, dimensions, colors, strings,
comments). New grammar at the statement level.

```spec
# Declares a SchLib document
schlib "MyComponents" {

  component "R_0603" {
    description: "0603 chip resistor"
    designator: "R"

    pin "1" { electrical: passive, at: (0mil, 0mil) }
    pin "2" { electrical: passive, at: (100mil, 0mil) }

    # Symbol graphics (body rectangle + pin stubs)
    rectangle { from: (10mil, -10mil), to: (90mil, 10mil), line_width: 1mil }

    parameter "Value"     { text: "" }
    parameter "Footprint" { text: "0603", is_hidden: true }

    footprint "0603" {
      map: [
        { pin: "1", pad: "1" }
        { pin: "2", pad: "2" }
      ]
    }
  }

  component "C_0402" {
    description: "0402 MLCC"
    designator: "C"

    pin "1" { electrical: passive }
    pin "2" { electrical: passive }

    rectangle { from: (10mil, -10mil), to: (90mil, 10mil) }
  }
}
```

```spec
# Declares a PcbLib document
pcblib "MyFootprints" {

  footprint "0603" {
    description: "0603 metric (1608) chip"

    pad "1" {
      at: (0mil, 0mil)
      size: (30mil, 35mil)
      shape: rounded_rectangle
      layer: top
    }
    pad "2" {
      at: (65mil, 0mil)
      size: (30mil, 35mil)
      shape: rounded_rectangle
      layer: top
    }

    # Courtyard
    line { from: (-20mil, -25mil), to: (85mil, -25mil), layer: courtyard }
    line { from: (85mil, -25mil),  to: (85mil, 25mil),  layer: courtyard }
    line { from: (85mil, 25mil),   to: (-20mil, 25mil), layer: courtyard }
    line { from: (-20mil, 25mil),  to: (-20mil, -25mil), layer: courtyard }

    # Silkscreen
    line { from: (-20mil, -25mil), to: (-20mil, 25mil), layer: overlay, line_width: 5mil }
    line { from: (85mil, -25mil),  to: (85mil, 25mil),  layer: overlay, line_width: 5mil }
  }
}
```

```spec
# Declares a SchDoc document (a schematic sheet)
schdoc "PowerSupply" {

  # Place components from library
  place "U1" {
    lib_reference: "LM7805"
    value: "LM7805"
    at: (2000mil, 1500mil)
  }

  place "C1" {
    lib_reference: "C_Polarized"
    value: "10uF"
    at: (1500mil, 1500mil)
  }

  place "C2" {
    lib_reference: "C_Polarized"
    value: "100uF"
    at: (2500mil, 1500mil)
  }

  # Wires (nets)
  wire { from: (1500mil, 1500mil), to: (2000mil, 1500mil), net: "VIN" }
  wire { from: (2000mil, 1200mil), to: (2000mil, 1000mil), net: "GND" }
}
```

### Grammar Overview

```ebnf
spec_file   = doc_decl ;
doc_decl    = doc_type STRING "{" decl* "}" ;
doc_type    = "schlib" | "pcblib" | "schdoc" | "pcbdoc" ;

decl        = entity_decl | property_stmt ;
entity_decl = entity_type STRING? "{" (property_stmt | entity_decl)* "}" ;
entity_type = "component" | "footprint" | "pin" | "pad" | "place"
            | "wire" | "bus" | "port" | "power_port"
            | "line" | "rectangle" | "arc" | "circle" | "polyline"
            | "polygon" | "text" | "region" | "via" | "track"
            | "parameter" | "footprint_map" | "alias" ;
property_stmt = key ":" expr ("," | NEWLINE) ;

(* Expressions reuse the existing ops lexer: *)
expr        = literal | array | object | tuple | ident ;
literal     = STRING | INTEGER | FLOAT | DIM | COLOR | BOOL ;
array       = "[" (expr ("," expr)*)? "]" ;
object      = "{" (property_stmt)* "}" ;
tuple       = "(" expr "," expr ")" ;
```

### Key Differences from .ops

| Aspect          | `.ops` (imperative)              | `.spec` (declarative)             |
|-----------------|----------------------------------|-----------------------------------|
| Mental model    | "Do these steps"                 | "This is what exists"             |
| Structure       | Flat list of operations          | Nested tree matching doc structure|
| Identity        | OpId references (`$r1`)          | Named entities (`"R1"`, `"pad1"`)|
| Ordering        | Execution order matters          | Order is cosmetic (compiler sorts)|
| Verbs           | `add_component`, `add_pin`       | `component`, `pin` (nouns)        |
| Back-references | `component_ref: $create_comp`    | Implicit via nesting              |
| Idempotency     | Not idempotent (duplicates)      | Always from-scratch (trivially idempotent) |
| Use case        | Patching existing documents      | Defining documents from scratch   |

### Compilation Pipeline

```
.spec file
    ↓  parse
Spec AST (tree of declarations)
    ↓  lower_spec_to_high_ops()
Vec<HighOp> (existing ops pipeline)
    ↓  existing pipeline
Vec<ComposedOp> → Vec<LowOp> → document mutations
    ↓  save
.SchLib / .PcbLib / .SchDoc / .PcbDoc file
```

The spec compiler is a **new front-end** that emits the same `HighOp` types the
ops pipeline already consumes. Zero changes needed to the ops engine, composed
ops, low ops, or document mutation code.

**Lowering examples:**

```
spec:  component "R_0603" { pin "1" { electrical: passive } }
  ↓
ops:   r_0603 = add_component { lib_reference: "R_0603", pins: [{ designator: "1", electrical: passive }] }

spec:  footprint "0603" { pad "1" { at: (0,0), size: (30mil, 35mil) } }
  ↓
ops:   fp = add_footprint { name: "0603" }
       add_pad $fp { designator: "1", at: (0,0), size_x: 30mil, size_y: 35mil }
```

## Query and Measurement Ops

Validation agents need to inspect built files and verify correctness. This is a
separate concern from building — the agent reads the file and runs queries.

### Current Query Capabilities

Already implemented:
- `query` — select entities by CSS-like selector
- `query_components` — list components with metadata
- `query_pins` — list pins on a component
- `query_records` — list records by type

These return `OpResult` with `refs` and `fields` maps.

### Proposed Measurement Ops

New ops that compute geometric properties, always read-only:

#### `measure` — Single-entity measurements

```ops
# Footprint bounding box
m1 = measure footprint[name="0603"] {
  metrics: [bounding_box, center, pad_count]
}
# Result:
#   bounding_box: { min: (-20mil, -25mil), max: (85mil, 25mil) }
#   center: (32.5mil, 0mil)
#   pad_count: 2
```

#### `measure_distance` — Between two entities

```ops
# Pad-to-pad pitch
m2 = measure_distance {
  from: pad[designator="1"]
  to:   pad[designator="2"]
  metric: center_to_center
}
# Result:
#   distance: 65mil
```

#### `measure_clearance` — Minimum gap between entities

```ops
# Pad edge-to-pad edge clearance
m3 = measure_clearance {
  from: pad[designator="1"]
  to:   pad[designator="2"]
  metric: edge_to_edge
}
# Result:
#   clearance: 35mil  (65mil center-to-center minus 15mil half-pad minus 15mil half-pad)
```

#### `check` — Constraint validation (pass/fail)

```ops
# Verify footprint matches datasheet
check footprint[name="0603"] {
  pad_pitch:     { expected: 65mil, tolerance: 2mil }
  pad_width:     { min: 25mil, max: 40mil }
  pad_height:    { min: 30mil, max: 45mil }
  courtyard:     { min_clearance: 15mil }
}
# Result:
#   status: pass
#   violations: []
```

```ops
# Or fail with details
check footprint[name="0603"] {
  pad_pitch: { expected: 50mil, tolerance: 1mil }
}
# Result:
#   status: fail
#   violations: [
#     { rule: "pad_pitch", expected: 50mil, actual: 65mil, tolerance: 1mil }
#   ]
```

### Validation Workflow

```
     ┌──────────────┐
     │  LLM Agent   │
     │  (builder)   │
     └──────┬───────┘
            │ writes/edits
            ▼
     ┌──────────────┐        ┌──────────────┐
     │  .spec file  │───────→│ altium build │──→ .PcbLib / .SchLib / etc.
     └──────────────┘  build └──────────────┘           │
                                                         │
     ┌──────────────┐        ┌──────────────┐           │
     │  LLM Agent   │←───────│ altium check │←──────────┘
     │  (validator) │ report └──────────────┘
     └──────┬───────┘
            │ feedback
            ▼
     ┌──────────────┐
     │  .check file │  (measurement queries + assertions)
     └──────────────┘
```

Two-agent loop:
1. **Builder agent** maintains `.spec` files, runs `altium build` to create Altium files
2. **Validator agent** runs `.check` files against built files, reports violations
3. Builder agent incorporates feedback, edits `.spec`, rebuilds

Or single-agent loop:
1. Agent writes `.spec` with inline assertions at the bottom
2. `altium build` compiles spec + runs assertions in one pass
3. Agent reads report, edits spec, repeats

### Inline Assertions in .spec

For the single-agent workflow, `.spec` files can include trailing assertions:

```spec
pcblib "MyFootprints" {
  footprint "0603" {
    pad "1" { at: (0mil, 0mil), size: (30mil, 35mil) }
    pad "2" { at: (65mil, 0mil), size: (30mil, 35mil) }
  }
}

# Validation section — runs after build
check {
  assert measure_distance(pad[designator="1"], pad[designator="2"]).distance == 65mil
  assert measure(footprint[name="0603"]).pad_count == 2
  assert measure(footprint[name="0603"]).bounding_box.width >= 80mil
}
```

## CLI Interface

### Build from spec

```bash
# Build a PcbLib from spec
altium build my-footprints.spec -o MyFootprints.PcbLib

# Build with JSON report (for LLM consumption)
altium build my-footprints.spec -o MyFootprints.PcbLib --report-json

# Dry run (parse + validate spec, don't write file)
altium build my-footprints.spec --dry-run
```

### Validate existing file

```bash
# Run measurement queries against a built file
altium check MyFootprints.PcbLib --check-file validations.check

# Or inline in the spec (build + check in one pass)
altium build my-footprints.spec -o MyFootprints.PcbLib --check
```

### Ops still available for patching

```bash
# Patch an existing file (imperative, non-idempotent)
altium ops apply existing.SchDoc --spec-file patches.ops -o modified.SchDoc
```

## Implementation Plan

### Phase 1: Spec Parser + Build Command

**New code:**
- `crates/altium-format-ops/src/spec/` — spec parser module
  - `lexer.rs` — reuse existing ops lexer (import, don't fork)
  - `parser.rs` — new recursive descent parser for spec grammar
  - `ast.rs` — spec AST types (DocDecl, EntityDecl, PropertyStmt)
  - `lower.rs` — `lower_spec_to_high_ops()`: spec AST → Vec<HighOp>
  - `mod.rs` — public API: `compile_spec(source: &str, doc_type) -> Result<Vec<HighOp>>`

**Modified code:**
- `crates/altium-cli/src/main.rs` — add `altium build` subcommand

**No changes to:**
- ops pipeline (HighOp → Composed → Low → document)
- altium-format (document types, parsing, saving)
- existing `altium ops apply` command

The spec compiler is a thin translation layer: `spec AST → Vec<HighOp>`. All
the heavy lifting (lowering, execution, validation, save) is already done.

### Phase 2: Measurement Ops

**New code:**
- `crates/altium-format-ops/src/ops/measure.rs` — measurement operation types
- `crates/altium-format/src/pcb_ops_core.rs` — add `measure_*` low ops
  - Bounding box computation (reuse existing `BoundingBox` type)
  - Center-to-center distance
  - Edge-to-edge clearance
- `crates/altium-format-ops/src/ops/model.rs` — new HighOp variants:
  `Measure`, `MeasureDistance`, `MeasureClearance`, `Check`

**Modified code:**
- ops pipeline to handle new HighOp variants
- CLI to support `altium check` subcommand

### Phase 3: LLM Agent Integration

- Structured JSON report format for `--report-json`
- Error messages optimized for LLM consumption (actionable, with line numbers)
- Example `.spec` files for common patterns (resistor lib, capacitor lib, etc.)
- Agent prompt templates that explain the spec format

### Future: Multi-File Project Specs

```spec
# project.spec — defines an entire Altium project
project "PowerSupply" {

  schlib "Components" {
    component "LM7805" { ... }
    component "C_Polarized" { ... }
  }

  pcblib "Footprints" {
    footprint "TO-220-3" { ... }
    footprint "0805" { ... }
  }

  schdoc "Main" {
    place "U1" { lib_reference: "LM7805", ... }
    place "C1" { lib_reference: "C_Polarized", ... }
  }

  // pcbdoc could reference the schdoc for netlist
}
```

This is a stretch goal. Start with single-file specs.

## Tradeoffs Summary

### .spec vs .ops for LLM authoring

| Dimension          | .spec (declarative)                    | .ops (imperative from-scratch)        |
|--------------------|----------------------------------------|---------------------------------------|
| Readability        | High — mirrors document structure      | Medium — flat list of commands        |
| LLM learnability   | High — just describe what you want    | Medium — must know op names + refs    |
| Idempotency        | Trivial — always from scratch          | Trivial — always from scratch         |
| Expressiveness     | Lower — no conditionals/loops/refs     | Higher — bindings, assertions, chains |
| Error messages     | Clear — "component 'R1' pin '3': ..." | Clear — "op create_r1: ..."          |
| Impl complexity    | Medium — new parser, thin lowering     | Zero — already exists                 |
| Patching existing  | Not supported (separate concern)       | Supported via ensure_* (future)       |

**Verdict**: .spec is better for the "LLM defines a project" use case. .ops
remains available for the "patch an existing file" use case. They compose well.

### Measurement ops: where to draw the line

We should implement measurements that help validate **fabrication-critical**
properties:

| Measurement              | Priority | Why                                      |
|--------------------------|----------|------------------------------------------|
| Pad pitch                | P0       | Wrong pitch = component won't fit         |
| Pad dimensions           | P0       | Wrong pad = solder defects                |
| Footprint bounding box   | P0       | Basic sanity check                        |
| Pad count                | P0       | Missing pad = open circuit                |
| Pad-to-pad clearance     | P1       | Too close = solder bridges                |
| Courtyard dimensions     | P1       | Assembly spacing                          |
| Silkscreen-to-pad gap    | P2       | Cosmetic but important                    |
| Copper area / fill check | P3       | Complex, less common in LLM workflow      |
| Trace length matching    | P3       | Signal integrity, advanced                |

Start with P0 (pad pitch, pad dimensions, bounding box, pad count). These cover
the most common LLM footprint-authoring errors.

## Open Questions

### Q1: Should .spec support variables/computation?

```spec
# Option A: Pure literals only
pad "1" { at: (0mil, 0mil), size: (30mil, 35mil) }
pad "2" { at: (65mil, 0mil), size: (30mil, 35mil) }

# Option B: Variables and expressions
let pad_pitch = 65mil
let pad_size = (30mil, 35mil)
pad "1" { at: (0mil, 0mil), size: pad_size }
pad "2" { at: (pad_pitch, 0mil), size: pad_size }
```

Option B is more DRY and less error-prone for parametric footprints. The lexer
already supports this. The question is whether the spec parser should too.

**Recommendation**: Yes, support `let` bindings and arithmetic expressions.
The lexer/expression evaluator already exist. Parametric specs are too useful
for footprint families (0402, 0603, 0805 sharing the same template with
different dimensions).

### Q2: Should .spec support includes?

```spec
# Option A: Single file
pcblib "MyLib" {
  footprint "0402" { ... 20 lines ... }
  footprint "0603" { ... 20 lines ... }
  footprint "0805" { ... 20 lines ... }
}

# Option B: Includes
pcblib "MyLib" {
  include "footprints/0402.spec"
  include "footprints/0603.spec"
  include "footprints/0805.spec"
}
```

Includes help with large libraries but add complexity (relative paths, circular
includes, error reporting across files).

**Recommendation**: Defer. Start with single-file specs. Add includes only if
LLMs consistently hit context-window limits with large specs.

### Q3: Templates / footprint families?

```spec
# Parametric footprint template
template chip_resistor(pitch, pad_w, pad_h) {
  pad "1" { at: (0mil, 0mil), size: (pad_w, pad_h), shape: rounded_rectangle }
  pad "2" { at: (pitch, 0mil), size: (pad_w, pad_h), shape: rounded_rectangle }
}

pcblib "Resistors" {
  footprint "0402" { chip_resistor(25mil, 20mil, 25mil) }
  footprint "0603" { chip_resistor(65mil, 30mil, 35mil) }
  footprint "0805" { chip_resistor(100mil, 45mil, 50mil) }
}
```

**Recommendation**: Defer to Phase 3+. `let` bindings cover the simple case.
Full templates are a language design exercise that shouldn't block the MVP.
