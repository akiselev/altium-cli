# Altium Spec Language Documentation Outline

Three documentation tiers: **Tutorials** (learning by doing), **Guides** (task-oriented
how-tos), and **Reference** (exhaustive specification). Tutorials build skill
progressively; guides answer "how do I..."; reference is where you look things up.

---

## Part 1: Tutorials

Progressive, hands-on lessons. Each tutorial produces a working spec file that the
reader applies with `altium apply`. Start simple, layer concepts one at a time.

### Tutorial 1: Your First Spec — A Resistor

**Goal**: Write a complete schlib-spec for a single 2-pin passive component, apply it,
and see the result in Altium Designer.

**Concepts introduced**:
- What the spec language is and why it exists (declarative, idempotent, ECO-grade)
- File extensions (`.schlib-spec` -> `.SchLib`)
- `component` declaration, entity names (unquoted identifiers)
- Basic properties: `designator`, `description`
- `pin` declaration with absolute placement (`at: (x, y)`)
- Dimensional literals: `100mil`, `20mm` — explain that bare numbers default to mils
- `electrical` type as a bare identifier enum (`passive`)
- `orientation` values (0/90/180/270 — explain what they mean for pins: "pin points
  right and connects on its left side")
- `rectangle` graphic for the component body
- Running `altium plan` to see the ECO, then `altium apply` to create the file

**Sample code progression**:
```
// 1. Minimal (absolute placement)
component R {
    designator: "R?"
    description: "Resistor"
    rectangle { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }
    pin 1 { at: (-30mil, 0), orientation: 0, electrical: passive, length: 25 }
    pin 2 { at: (30mil, 0), orientation: 180, electrical: passive, length: 25 }
}
```

**Pitfalls to call out**:
- Pin orientation is confusing: `orientation: 0` means the pin stub points RIGHT
  and the connection point is on the LEFT. Draw a diagram.
- `length: 25` without a unit suffix means 25 mils (default unit for dim fields).
  Mention this explicitly because it's a common source of "why is my pin invisible"
  bugs (someone writes `length: 25mm` thinking it's 25 mils).
- The coordinate system: origin is component center, positive X is right, positive Y
  is up (NOT down like screen coordinates).

**Exercise**: Modify the resistor to have 4 pins (resistor network). Add a
`description` change and re-run `altium plan` to see the ECO shows "update" vs "add".


### Tutorial 2: Anchor-Based Placement — Stop Counting Pixels

**Goal**: Rewrite the Tutorial 1 resistor using anchor-based placement instead of
hardcoded coordinates. Then add a capacitor and inductor to see how anchors scale.

**Concepts introduced**:
- Binding names: `body = rectangle { ... }`
- `$body` reference syntax (dollar-prefix for bound entities)
- Anchor edges: `$body.left`, `$body.right`, `$body.top`, `$body.bottom`
- `on:` + `at: center` placement mode (vs absolute `at: (x, y)`)
- `side: outside` / `inside` / `center`
- Why `orientation` becomes `auto` with anchors (inferred from edge)
- Forward references: pin can reference `$body` even if `body =` is declared after it

**Sample code progression**:
```
// Step 1: bind the body
body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }

// Step 2: place pins on edges
pin 1 { on: $body.left, at: center, side: outside, electrical: passive, length: 25 }
pin 2 { on: $body.right, at: center, side: outside, electrical: passive, length: 25 }
```

**Key insight to hammer home**: With anchors, if you resize the rectangle body, the
pins automatically stay on the correct edges. With absolute placement, you'd have to
manually recalculate every pin position.

**Pitfalls to call out**:
- `at:` is overloaded: with `on:`, it means edge position (`start`/`center`/`end`);
  without `on:`, it's a coordinate pair. This is the #1 source of confusion. Show
  the error message you get if you write `on: $body.left, at: (10, 20)`.
- Corner anchors (`$body.top_left`) exist for coordinate references but CANNOT be
  used with `on:` for placement. Explain why: corners are points, not edges, so
  `at: start/center/end` and sequencing are meaningless.
- `side: outside` vs `side: inside`: draw a diagram showing where the pin stub
  goes relative to the rectangle edge.

**Exercise**: Create a 6-pin component (e.g., SOT-23 symbol) with pins on three
edges using anchors.


### Tutorial 3: Templates and Spread — DRY Specs

**Goal**: Reduce repetition using `let` bindings and the spread operator.

**Concepts introduced**:
- `let` bindings at file level and inside component blocks
- Object literals: `{ key: value, key: value }`
- Spread operator: `...template_name`
- Override semantics: explicit fields beat spread fields (last-wins)
- Multiple spreads: `{ ...physical, ...sizing, at: ... }`
- Scope rules: file-level `let` visible everywhere, component-level `let` visible
  within that component only

**Sample code progression**:
```
// File-level templates
let passive_pin = { electrical: passive, length: 25, side: outside }
let two_pin_body = { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }

component R {
    designator: "R?"
    body = rectangle { ...two_pin_body }
    pin 1 { ...passive_pin, on: $body.left, at: center }
    pin 2 { ...passive_pin, on: $body.right, at: center }
}

component C {
    designator: "C?"
    body = rectangle { ...two_pin_body }
    pin 1 { ...passive_pin, on: $body.left, at: center }
    pin 2 { ...passive_pin, on: $body.right, at: center }
}
```

**Key insight**: Show how changing `passive_pin`'s `length` from `25` to `30` updates
ALL pins across ALL components. Then show how one pin can override: `{ ...passive_pin,
length: 40 }`.

**Pitfalls to call out**:
- Spread only works on objects, not arrays. `[...arr1, ...arr2]` is NOT supported.
- Circular references are caught at evaluation time, not parse time. Show the error:
  `let a = { ...b }` / `let b = { ...a }`.
- `let` keyword is optional (`x = { ... }` and `let x = { ... }` are identical). The
  keyword exists for readability / LLM comfort. Mention this once and pick one style
  for the rest of the docs.
- Bare identifiers in spread resolve as bindings, not strings. `...passive` looks up
  the binding named `passive`, it doesn't mean the string "passive".


### Tutorial 4: Multi-Part Components — Dual Op-Amps and Quad Gates

**Goal**: Build a dual op-amp (LM358) with proper part blocks, shared power pins,
and per-part graphics.

**Concepts introduced**:
- `part N { ... }` blocks for multi-part components
- `owner_part_id` concept (part 0 = shared across all parts)
- Scope isolation between parts (each part has its own `body =`)
- Same binding name in different parts (`body` in part 1 and part 2 are independent)
- Hidden power pins with `is_hidden: true` and `hidden_net_name`
- `part_count` inference vs explicit override
- Relative placement with `after:` and `before:` (pin sequencing on an edge)
- `gap:` for spacing between sequenced pins

**Sample code**: Full LM358 from spec-lang.md Example 4.

**Pitfalls to call out**:
- Pins at component level (outside any `part` block) are shared across all parts
  (owner_part_id = 0). This is where power pins go.
- Part-level bindings are isolated. `$body` in `part 1` is NOT visible in `part 2`.
  But component-level bindings ARE visible inside parts (lexical scoping).
- `after: $p2` requires `$p2` to be on the SAME anchor edge. Cross-edge `after:`
  is an error. Show the error message.
- Pin designators CAN repeat across parts (e.g., both parts could have a pin named
  "IN+"), but within a single part, designators must be unique.

**Exercise**: Extend to a quad NAND gate (74HC00) with 4 parts of 3 pins each, plus
shared VCC/GND.


### Tutorial 5: Your First Footprint — SOT-23

**Goal**: Write a pcblib-spec for a 3-pad SMD footprint.

**Concepts introduced**:
- `.pcblib-spec` file extension -> `.PcbLib`
- `footprint` declaration
- `pad` declaration with absolute placement
- Pad properties: `shape`, `x_size`, `y_size`, `layer`, `hole_size`, `is_plated`
- SMD vs through-hole pads (hole_size: 0 = SMD)
- PCB layer names as strings (`"TopLayer"`, `"MultiLayer"`, `"TopOverlay"`)
- PCB graphics: `track` (instead of `line`), `polyline` with `closed: true`
- Silkscreen outline on TopOverlay
- Metric dimensions from datasheets (`0.95mm`, `0.6mm`)

**Sample code**: SOT-23 from spec-lang.md section 5.

**Pitfalls to call out**:
- PCB layer names are strings, not enums. `layer: "TopLayer"` not `layer: TopLayer`.
  (Actually, enum resolution IS supported for layers — clarify which way the
  implementation actually works and document accordingly.)
- `pad_mode: simple` is the default. Only mention `top_middle_bottom` as "exists for
  advanced use" without going deep.
- Through-hole pads need `layer: "MultiLayer"` + `hole_size > 0` + `is_plated: true`.
  SMD pads need `layer: "TopLayer"` + `hole_size: 0`.
- Pin 1 marking convention: add a small arc or dot on the silkscreen layer.


### Tutorial 6: Row/Column/Grid Layouts — QFP and BGA

**Goal**: Use layout primitives to generate pad patterns for IC packages.

**Concepts introduced**:
- `row` block with `on:`, `pitch`, `count`, `start`, `side`, `pad:` template
- `direction: forward` vs `reverse` — per-edge meaning table (critical!)
- Absolute rows with `at:` coordinate and `direction: up`/`down`/`left`/`right`
- `column` (same as row, provided for readability)
- `grid` block with `origin`, `rows`, `cols`, `pitch`, `naming: alphanumeric`
- `skip` for omitting pads (thermal void in BGA, irregular QFN)
- `perimeter_only` for perimeter-only BGA
- Per-pad override: explicit `pad N { shape: rectangular }` overrides row template
- Exposed/thermal pad added as explicit `pad EP { ... }` alongside grid

**Sample code**: QFP32 (4 rows), DIP8 (2 absolute rows), BGA256 (grid).

**Pitfalls to call out**:
- Row direction semantics are the hardest thing in the language. The per-edge
  `forward` direction table MUST be memorized or referenced:
  - left edge: forward = top-to-bottom (decreasing Y)
  - right edge: forward = bottom-to-top (increasing Y)
  - top: forward = left-to-right
  - bottom: forward = right-to-left
  This matches IC pin numbering conventions (counter-clockwise).
- `up`/`down`/`left`/`right` direction values are ONLY for absolute-positioned rows.
  Using them with `on:` anchor is an error.
- `skip` matches against pad NAMES (after numbering), not positional indices. In a
  row with `start: 5`, `skip: [7]` skips the pad named "7", which is the 3rd pad.
- Grid `naming: alphanumeric` skips letters I, O, Q, S, X, Z (industry convention
  for BGAs — verify this against the implementation!).
- Overriding a row-generated pad is NOT a duplicate identity error. It's a
  declarative merge.


### Tutorial 7: Imports and Composition

**Goal**: Split a library across multiple spec files and link components to footprints.

**Concepts introduced**:
- `import "file.pcblib-spec" as fp` (named import for cross-domain references)
- `import "other.schlib-spec"` (bare import for merging into one output)
- `$alias.EntityName` and `$alias["Name With Spaces"]` path syntax
- Footprint linking: `footprint $fp.DIP8 { map { pin: 1, pad: 1 } ... }`
- `map` entries and pin-to-pad mapping validation
- Cross-domain import rules (schlib can import pcblib named, not bare)
- Bare import collision detection
- Cycle detection

**Sample code**: Multi-file project:
```
footprints.pcblib-spec  (DIP8, SOT23, QFP32)
passives.schlib-spec    (R, C, L — imports footprints.pcblib-spec as fp)
ics.schlib-spec         (LM358 — imports footprints.pcblib-spec as fp)
my-library.schlib-spec  (bare imports passives + ics)
```

**Pitfalls to call out**:
- `let` bindings from bare imports are NOT merged. Only entity declarations
  (component, footprint) are. If you need shared templates across files, each file
  must define its own or use named imports.
- Import paths are relative to the importing file's directory.
- Binary Altium files can also be imported:
  `import "vendor-parts.SchLib" as vendor`. This is important for adoption — users
  can reference existing libraries without spec-ifying them.
- Unmapped pads are allowed (thermal pads, mounting holes) but emit a note. Unmapped
  pins are NOT currently validated (spec currently validates at apply time).


### Tutorial 8: Dump and Roundtrip — Adopting Existing Libraries

**Goal**: Take an existing Altium SchLib/PcbLib, dump it to spec, inspect the output,
and re-apply it.

**Concepts introduced**:
- `altium dump my-parts.SchLib` -> generates `my-parts.schlib-spec`
- The dump output uses absolute placement (never anchors)
- Coordinate formatting: prefers mm when values are "clean", falls back to mils
- Roundtrip workflow: dump -> edit spec -> plan -> apply
- `--target` flag for applying spec to a specific existing file
- ECO as a review artifact: run `plan` before `apply` to review changes

**Key workflows to show**:
1. "I have an existing SchLib, I want to version-control it as spec":
   `dump` -> commit spec -> edit -> `plan` -> review -> `apply`
2. "I want to add a new component to an existing library":
   Write spec with just the new component -> `apply --target existing.SchLib`
3. "I want to see what changed between spec and binary":
   `altium plan my-parts.schlib-spec` (shows ECO diff)

**Pitfalls to call out**:
- Dump output is verbose because it uses absolute coordinates. You can refactor
  it to use anchors and templates afterward.
- The spec is ADDITIVE. Components in the binary but NOT in the spec are preserved
  when you `apply`. This is a feature, not a bug — it means you can manage a subset
  of a library with specs.
- Re-dumping after apply should produce a spec that, when planned against the same
  binary, shows "all unchanged" (idempotency proof).


---

## Part 2: Guides (How-To)

Task-oriented. Each guide answers a specific "How do I...?" question. No assumed
reading order. Cross-reference tutorials and reference sections.

### Guide: Choosing Between Absolute and Anchor Placement

**When to use absolute placement**:
- Footprint pads from datasheets (exact coordinates given)
- Dumped specs (always absolute)
- Very simple 2-pin components where anchors are overkill

**When to use anchor placement**:
- Schematic symbols with pins on body edges
- Any component where body size might change
- Multi-part components with identical body shapes

**Decision table and tradeoff discussion.** Mixing is fine within one footprint/component.

### Guide: Managing Pad Properties for Manufacturing

- Which pad properties are supported: shape, size, hole, layer, rotation, plating
- Mask expansion (solder mask, paste mask) — when to override defaults
- Thermal relief settings (plane_connection, relief entries, conductor width, air gap)
- Pad mode (simple vs top_middle_bottom)
- Through-hole vs SMD checklist
- Common gotcha: forgetting `is_plated: false` for NPTH mounting holes

### Guide: Idempotency and Additive Semantics

**Core concept**: "The spec is a subset assertion." Explain deeply with examples:
- Running `apply` twice is a no-op (show `plan` output with "all unchanged")
- Components NOT in the spec are left alone
- Renaming an entity in the spec creates a NEW entity (old one persists)
- Removing an entity from the spec does NOT delete it from the binary
- What `purge` semantics would look like (future feature)
- Practical implications: you can spec-manage 3 components in a 50-component library

### Guide: Reading and Acting on ECO Output

- ECO text format walkthrough (the box diagram, summary table, change tree)
- Change types: `+ ADD`, `~ UPDATE`, `= unchanged`
- Property change format: `field: "old" -> "new"`
- Using `--json` for scripted workflows
- How to use ECOs in a hardware review process (print/paste into design review doc)
- Collapsed unchanged entries

### Guide: Working with the Expression Language

- Dimensional arithmetic: `100mil + 2.54mm` works because everything converts to
  internal units
- When to use which unit suffix (mil for schematic, mm for PCB from datasheets)
- Template strings for dynamic text: `` `prefix {expr} suffix` ``
- Path expressions: `$ref.field`, `$ref["key"]`
- Gotcha: `20 mm` (with space) is `INTEGER IDENT`, not `DIM`. The unit suffix must
  be immediately adjacent to the number.
- Gotcha: bare numbers in dim fields default to mils. `length: 25` = `length: 25mil`.
  But `length: 25mm` = 25 millimeters = 250x larger. Double-check your units.

### Guide: Designing Component Symbols for Schematic Libraries

**Altium-specific schematic symbol conventions**:
- Body sizing guidelines (100mil grid alignment)
- Pin length conventions (typically 25mil = 250,000 internal units)
- Pin electrical types and when to use each (passive, input, output, power, hi_z, etc.)
- Hidden power pins: when and why
- Designator patterns ("R?", "U?", "C?")
- Parameter "Value" with `text: "{VALUE}"` for Altium substitution
- Alias declarations for alternate part numbers

### Guide: Designing Footprints from IC Datasheets

**Step-by-step from datasheet to spec**:
1. Read the package dimensions table (all in mm usually)
2. Calculate pad positions from pin pitch and body dimensions
3. Choose between individual pads vs row/grid layout
4. Add silkscreen outline on TopOverlay
5. Add courtyard if needed
6. Mark pin 1 with a dot/arc
7. Set `height` for 3D clearance checking
8. Common packages walkthrough: SOT-23, SOIC-8, QFP-32, BGA, DIP

### Guide: Organizing Multi-File Library Projects

**Project structure patterns**:
```
my-project/
  footprints/
    smd-passives.pcblib-spec
    ic-packages.pcblib-spec
    connectors.pcblib-spec
  symbols/
    passives.schlib-spec      (imports footprints/smd-passives as fp)
    ics.schlib-spec           (imports footprints/ic-packages as fp)
    connectors.schlib-spec    (imports footprints/connectors as fp)
  library.schlib-spec         (bare imports all symbols/*.schlib-spec)
```

- One spec file per output binary (1:1 mapping rule)
- Bare imports for merging, named imports for cross-referencing
- When to split vs keep together
- Importing existing binary Altium files alongside spec files

### Guide: Version Control Workflow for Altium Libraries

- Spec files are text -> git-friendly diffs
- Suggested gitignore (don't track .SchLib/.PcbLib if generated from spec)
- Branching strategy: edit spec on branch -> `plan` to review ECO -> merge -> `apply`
- CI pipeline: `altium plan --json` to validate specs in CI
- When to commit binaries alongside specs (for Altium Designer users who don't have
  the CLI)

### Guide: Error Messages and Troubleshooting

**Categorized error guide with examples**:

**Parse errors (E1xxx)**:
- E1001: Unexpected token — show common causes (missing comma, wrong bracket type)
- E1002: Unterminated string
- E1003: Invalid number format
- E1004: Unknown escape sequence

**Compilation errors (E2xxx)**:
- E2001: Unknown property name
- E2002: Type mismatch (expected dim, got string)
- E2003: Duplicate identity key
- E2004: Unknown enum value
- E2005: Circular binding reference

**Placement errors**:
- Cross-edge reference (pin on $body.left referencing pin on $body.right with `after:`)
- Corner anchor used with `on:` (corners are points, not edges)
- `at:` type confusion (coord vs enum when `on:` is/isn't present)

**Import errors**:
- Circular import
- Cross-domain violation (pcblib importing schlib)
- Duplicate alias
- Bare import collision

For each: show the exact error message, explain what it means, show how to fix it.


---

## Part 3: Language Reference

Exhaustive, precise, alphabetically/structurally organized. Every keyword, every
property, every type, every rule. This is the "man page."

### Reference: File Structure

- File extensions and their meaning
- Top-level items: `import`, `let`, `component`, `footprint`
- Statement separators (comma, newline, semicolon)
- Comment syntax (line `//`, block `/* */`, nesting)
- Whitespace rules (not indentation-sensitive)
- LLM tolerance tokens (`let`, `;` are optional noise)

### Reference: Import Declarations

- `import "path" as alias` — named import
- `import "path"` — bare import (merge)
- Path resolution (relative to importing file)
- Cross-domain import matrix table
- Bare import collision rules
- Named import alias uniqueness
- Cycle detection
- Binary file imports (`.SchLib`, `.PcbLib`)

### Reference: Let Bindings

- Syntax: `[let] name = expr`
- Scope rules: file-level, component-level, part-level
- Forward reference support (lazy evaluation)
- Circular reference detection
- `let` keyword is optional (noise token)

### Reference: Component Declaration (SchLib)

Full property table with types, defaults, descriptions:
- `designator` (String, required)
- `description` (String, default "")
- `component_kind` (Enum: standard, mechanical, graphical, ...)
- `part_count` (Int, inferred or explicit)
- `show_hidden_pins` (Bool, default false)

Child declarations: `pin`, `parameter`, `alias`, `footprint`, graphics, `part` blocks.

### Reference: Part Block

- Syntax: `part N { ... }`
- owner_part_id semantics
- Scope isolation rules
- Inferred part_count
- Allowed children (pins, graphics, let bindings)

### Reference: Pin Declaration

**Full property table**:
- `at` — overloaded: Coord (absolute) or Enum (anchor: start/center/end)
- `on` — anchor edge reference
- `after` / `before` — relative sequencing
- `gap` — spacing (default 100mil)
- `offset` — post-placement translation
- `side` — inside/outside/center
- `orientation` — 0/90/180/270/auto
- `electrical` — full enum table with all 8 types
- `length` — dim
- `name` — string
- `is_hidden` — bool
- `hidden_net_name` — string

**Placement mode rules**:
- Absolute mode: `at: (x, y)` without `on:`
- Anchor mode: `on:` + `at: start|center|end`
- Mutual exclusivity constraints
- Error cases

### Reference: Parameter Declaration

- `name` (identity key, from entity name)
- `text` (String)
- `is_hidden` (Bool)
- Note about `{PARAM_NAME}` Altium substitution vs spec template strings

### Reference: Alias Declaration

- Syntax: `alias NAME`
- No body
- Identity key = alias name

### Reference: Footprint Map Declaration

- `footprint NAME { map { pin: N, pad: N } ... }`
- `footprint $import.Name { ... }` (imported footprint reference)
- Validation rules (duplicate maps, missing pins/pads, unmapped pads)
- Implementation chain in Altium (SchImplementation -> SchImplMap)

### Reference: Schematic Graphics

Full property table for each graphic type:
- `line` — from, to, color, line_width
- `rectangle` — from, to, is_solid, color, area_color
- `arc` — center, radius, start_angle, end_angle
- `elliptical_arc` — center, radius, secondary_radius, start_angle, end_angle
- `ellipse` — center, radius, secondary_radius, is_solid
- `polyline` — points, color, line_width
- `polygon` — points, is_solid, color, area_color
- `bezier` — points (4 control points)
- `pie` — center, radius, start_angle, end_angle, is_solid
- `round_rectangle` — from, to, corner_x_radius, corner_y_radius
- `label` — at, text, font_id, color
- `text_frame` — from, to, text, is_solid, show_border
- `image` — from, to, file_name, image_data

Binding name as identity (`body = rectangle { ... }` -> unique_id = "spec:component:body").
Unnamed graphics get auto-generated (unstable) identity.

### Reference: Footprint Declaration (PcbLib)

Property table:
- `description` (String)
- `height` (Dim)
- `pattern` (String, defaults to display_name)

Child declarations: `pad`, `row`, `column`, `grid`, PCB graphics.

### Reference: Pad Declaration

Full property table (every supported field, type, default, description).
Both absolute and anchor placement.

### Reference: Row, Column, Grid Layouts

**Row/Column**:
- Complete property table
- Per-edge forward direction table
- Absolute vs anchor mode
- `skip` semantics (name matching, not positional)
- Override semantics (explicit `pad N` merges with generated pad)
- Direction values (forward, reverse, up, down, left, right) and their restrictions

**Grid**:
- Complete property table
- `naming` schemes (numeric, alphanumeric)
- `perimeter_only`
- `skip` semantics
- Origin and centering

### Reference: PCB Graphics

Full property table for each:
- `track` — start, end, width, layer
- `arc` — center, radius, start_angle, end_angle, width, layer
- `fill` — corner1, corner2, rotation, layer
- `region` — outline, holes, kind, layer
- `text` — at, text, height, rotation, layer, font
- `via` — at, diameter, hole_size, start_layer, end_layer
- `component_body` — model_name, standoff_height, overall_height, body_opacity
- `line` — from, to, width, layer (alias for track with schlib naming)
- `polyline` — points, width, layer, closed (lowers to tracks or region)

### Reference: Anchor System

**Anchor table** by geometry class:
- Box (rectangle, round_rectangle, text_frame, image): top, bottom, left, right,
  corners, center
- Center+radius (arc, ellipse, pie): center, start_point, end_point
- Segment (line, track): start, end, midpoint
- Vertex-list (polyline, polygon, bezier): vertex[N], centroid
- Point (pin, label, via, pad): location

**Edge vs corner vs center**: which can be used with `on:`, which are coordinate-only.

**Auto-orientation inference table**:
- left -> 0, right -> 180, top -> 270, bottom -> 90

### Reference: Expression Language

**Literals**: String, Template, Integer, Float, Dim, Color, Bool, Null — syntax and
examples for each.

**Operators**: precedence table (. [] at 90, unary - at 70, * / at 60, + - at 50).

**Type rules for arithmetic**: dim+dim=dim, dim*number=dim, number+number=number, etc.

**Path expressions**: `$ident`, `ident`, `.field`, `[index]`, `["key"]` — resolution
order (keywords -> bindings -> enum registry).

**Coord construction**: `(x, y)` tuple syntax. `(expr)` is grouping, not 1-tuple.

**Arrays**: `[expr, ...]`. Homogeneous types.

**Objects and spread**: `{ key: val, ...spread }`. Last-wins rule. No array spread.

**Template strings**: `` `text {expr} text` ``. Escaping: `{{`, `}}`, `` \` ``.

### Reference: Type System

**Scalar types table**: String, Integer, Float, Dim, Bool, Null, Color, Ident.

**Unit suffixes**: mil (10,000 per), mm (393,701 per), in (10,000,000 per),
dxp (100,000 per), raw (1 per). Bare number default = mils.

**Enum resolution**: case-insensitive, underscore-insensitive. Full list of all enums
and their variants:
- PinElectricalType: input, io, output, open_collector, open_emitter, passive, hi_z, power
- PadShape: round, rectangular, octagonal
- ComponentKind: standard, mechanical, graphical, ...
- (all others)

**Type coercion table**: what converts to what at field boundaries.

### Reference: Identity Keys and Reconciliation

**Identity key table**: which field is the identity key for each entity kind.

**Equality/normalization rules**: dimensions (internal units), coordinates (±1 tolerance),
colors (COLORREF), strings (case rules per field), enums (case/underscore insensitive),
booleans.

**Unique ID scheme**: seed format table, MD5 hash algorithm, collision resolution.

**Additive semantics**: add-if-missing, update-if-different, never-delete. Implications.

### Reference: ECO Format

- Text format structure (header box, summary, change tree, symbols)
- JSON format structure
- EntityChange types (Add, Update, Unchanged)
- PropChange structure
- Summary computation

### Reference: CLI Commands

- `altium plan <spec>` — plan ECO without mutating
- `altium apply <spec>` — apply ECO to document
- `altium dump <library>` — reverse-generate spec from binary
- Flags: `--output`, `--target`, `--json`, `--report-json`
- Exit codes

### Reference: Formal Grammar (EBNF)

Complete EBNF grammar from spec-lang.md section 16 (verbatim or lightly reformatted
for readability).

### Reference: Lexical Rules

- Token table with patterns and examples
- Disambiguation rules (DIM vs INTEGER+IDENT, COLOR, DOLLAR_IDENT)
- Separator rules (comma, newline, newline suppression inside brackets)
- Noise tokens (let, semicolon)

### Reference: Error Code Index

Full table of all error codes (E1001-E1008, E2001-E2008, etc.) with:
- Code
- Category (parse, compile, reconcile, import)
- Message template
- Common causes
- Fix suggestions


---

## Cross-Cutting Concerns

Things to weave through ALL three tiers, not put in one place:

### Coordinate System Diagrams
- Schematic: origin at component center, +X right, +Y up, units in mils
- PCB: origin at footprint center, +X right, +Y up, units in mm (typically)
- Pin orientation diagram (the four orientations and what they mean)
- Anchor edge diagram (which edge is which, what start/center/end mean on each)

### Mental Model: "Spec as Desired State"
Reinforce constantly:
- Not a script. Not imperative. Not "add pin 1, then add pin 2."
- "The document should contain a component R with these pins."
- Running twice = no-op. This is a FEATURE.
- The spec is a SUBSET. Absence from spec != deletion.

### Naming Conventions
Document the conventions used in examples:
- Components: uppercase with underscores (R_0603, LM358)
- Pins: numbers for physical pins, names like VCC/GND for power
- Pads: numbers matching pin numbers, EP for exposed pad
- Bindings: lowercase descriptive (body, p2, outline, pin1_mark)
- Templates: lowercase_with_underscores (passive_pin, smd_pad)

### Units Cheat Sheet
Persistent reference card:
- `100mil` = 100 thousandths of an inch = 2.54mm = 1,000,000 internal units
- `1mm` = ~39.37mil = 393,701 internal units
- `1in` = 1000mil = 25.4mm = 10,000,000 internal units
- Bare `25` in a dim field = 25mil
- When to use mil (schematic symbols) vs mm (PCB footprints from datasheets)


---

## Appendices

### Appendix A: Complete Working Examples

Full, copy-pasteable spec files for common components/packages:
1. Passive library (R, C, L) — schlib-spec
2. Basic connector (2x5 header) — schlib-spec
3. SOT-23 footprint — pcblib-spec
4. SOIC-8 footprint — pcblib-spec
5. QFP-48 footprint — pcblib-spec
6. BGA-256 footprint — pcblib-spec
7. DIP-8 footprint — pcblib-spec
8. Multi-part IC (LM358) with imported footprint — schlib-spec + pcblib-spec
9. Multi-file project (passives + ICs + footprints + main entry point)

### Appendix B: Altium Concepts Glossary

For readers unfamiliar with Altium terminology:
- SchLib / PcbLib: library files containing reusable components/footprints
- CFB: Compound File Binary (OLE container format Altium uses)
- ECO: Engineering Change Order
- Designator: component reference (R1, U3, C12)
- OWNERINDEX: Altium's parent-child linking mechanism
- Sidecar stream: supplementary data in separate CFB streams
- UniqueID: 8-character alphabetic identifier for traceability

### Appendix C: Property Quick-Reference Cards

One-page-per-entity-type summary cards with ALL properties, types, defaults.
Designed for printing or keeping open in a second monitor. Tables only, no prose.
