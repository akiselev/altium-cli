# Auto-Sizing & Default Geometry for Spec Language

## Problem Statement

The spec language already handles the *logical* layout of a schematic component:
- Which side each pin goes on (`on: $body.left`)
- Pin ordering and spacing (`after: $p1, gap: 5mil`)
- Relative positioning (`at: center`, `at: start`, `at: end`)
- Pin orientation (auto-computed from edge)

But the spec author (human or LLM) must currently compute concrete geometry:
- The body rectangle dimensions (`from: (-200mil, -300mil), to: (200mil, 300mil)`)
- Pin lengths (`length: 200mil`)
- The exact `from:`/`to:` coordinates that make the body big enough for all the pins

This is tedious and error-prone. An LLM generating a spec for a 40-pin IC has to
manually count pins per side, multiply by pitch, account for text widths, and produce
exact coordinates. The result is often wrong — tiny bodies, absurd pin lengths, pins
that don't fit.

**What we want**: The spec says *what goes where* (sides, ordering, grouping). The
layout engine figures out *how big* and *where exactly*, filling in sensible defaults.

## Scope: What This Does and Doesn't Do

### In Scope

1. **Auto-size body rectangle** — given pins assigned to each side with ordering/gaps,
   compute `from:`/`to:` dimensions that fit everything with proper margins
2. **Default pin length** — when spec doesn't specify `length:`, apply a sensible
   default (200mil for ICs, 100mil for passives)
3. **Default pin pitch** — when `gap:` isn't specified in `after:` chains, use 100mil
4. **Text-aware width** — make the body wide enough that pin names on left and right
   sides don't overlap inside the body
5. **Grid snapping** — ensure all computed coordinates land on 10mil grid
6. **Default body margins** — space between edge of body and first/last pin on each side

### Out of Scope

- **Pin-to-side assignment** — the spec says `on: $body.left`; we don't guess
- **Pin ordering** — the spec says `after: $p1`; we don't reorder
- **Pin grouping/classification** — no name heuristics, no PinRole inference
- **Custom graphics** — resistor zig-zags, op-amp triangles, logic gate shapes are
  manual `draw` specs. The layout engine only handles rectangular body sizing.
- **SchDoc sheet layout** — placing components on sheets, wire routing. Different problem entirely.
- **PcbDoc placement** — different domain

## How It Integrates with the Spec Language

### Current Flow

```
spec source → compile → SpecModel (absolute coordinates) → merge into SchLib
```

The spec author must provide exact coordinates for the body rectangle. The compiler
resolves all anchors relative to that rectangle.

### Proposed Flow

```
spec source → compile (body = auto) → resolve pins relative to auto-sized body → SpecModel → merge
```

When the body dimensions are omitted or marked `auto`, the compiler:
1. Collects all pins assigned to each edge (from `on:` directives)
2. Counts pins per side, accounting for `gap:` values in `after:` chains
3. Computes the body rectangle that fits everything
4. Then resolves anchor positions as normal

**Key insight**: This happens *inside* the compiler, not as a post-processing pass.
The auto-sizing must happen before anchor resolution because pin positions depend on
the body edges.

### Spec Syntax

```spec
# Current: manual dimensions required
component IC {
    body = rectangle { from: (-200mil, -300mil), to: (200mil, 300mil) }
    p1 = pin 1 { on: $body.left, at: "start", side: "outside", length: 200mil }
    ...
}

# Proposed: auto-sized body
component IC {
    body = rectangle { auto: true }
    p1 = pin 1 { on: $body.left, at: "start", side: "outside" }
    ...
}

# Proposed: partial override (fixed width, auto height)
component IC {
    body = rectangle { width: 400mil, auto_height: true }
    ...
}
```

When `auto: true`, the compiler must compute dimensions before resolving anchors.

## The Auto-Sizing Algorithm

### Inputs

For each edge of the body, the compiler knows:
- How many pins are on that edge (from `on:` directives)
- The spacing between pins (from `gap:` values, default 100mil)
- Pin lengths (from `length:` values, default 200mil)
- Pin names (for text width estimation)

### Algorithm

```
1. COLLECT: Group pins by edge (left, right, top, bottom)

2. COMPUTE EDGE SPANS: For each edge, calculate total span needed:
   left_span  = (left_pin_count - 1) * left_gap + 2 * margin
   right_span = (right_pin_count - 1) * right_gap + 2 * margin
   top_span   = (top_pin_count - 1) * top_gap + 2 * margin
   bot_span   = (bot_pin_count - 1) * bot_gap + 2 * margin

   Where "gap" may vary per pin pair (from after:/before: gap values).
   For simplicity: sum all inter-pin gaps on that edge + 2 * margin.

3. SIZE BODY:
   body_height = max(left_span, right_span, min_body_height)
   body_width  = max(top_span, bot_span, min_body_width)

   Also consider text widths:
   text_width = max_left_name_chars * char_width + max_right_name_chars * char_width + center_gap
   body_width = max(body_width, text_width)

4. SNAP TO GRID:
   body_height = round_up(body_height, grid)
   body_width  = round_up(body_width, grid)

5. CENTER AT ORIGIN:
   from = (-body_width/2, -body_height/2)
   to   = (body_width/2, body_height/2)

6. RESOLVE ANCHORS: Normal anchor resolution proceeds with the computed rectangle
```

### Handling `after:` Chains with Varying Gaps

When pins have explicit gaps:
```spec
p1 = pin 1 { on: $body.left, at: "start", side: "outside" }
p2 = pin 2 { on: $body.left, after: $p1, gap: 100mil, side: "outside" }
p3 = pin 3 { on: $body.left, after: $p2, gap: 200mil, side: "outside" }  # bigger gap (group break)
p4 = pin 4 { on: $body.left, after: $p3, gap: 100mil, side: "outside" }
```

Total left span = 100 + 200 + 100 + 2 * margin = 400 + 2 * 50 = 500mil.

The algorithm must walk the `after:`/`before:` chain to sum actual gaps, not just
multiply count by default pitch. This requires resolving the dependency graph
(which the compiler already does for anchor resolution) to extract gap values.

### Handling `at:` Positions

Pins placed with `at: center` or `at: start`/`at: end` (without `after:`) are
trickier — they're positioned relative to the edge endpoints, which depend on the
body size we're computing. This creates a circular dependency:

- Body size depends on how many pins are on each edge
- Pin positions depend on body size (for `at: center`)

**Resolution**: The auto-sizer computes body dimensions from pin *count* and *gaps*.
The `at:` positions are then resolved normally against the computed body. Pins using
`at: center` end up centered on the (now correctly-sized) edge. Pins using
`at: start` start at the edge with margin. This is correct — `at:` is about
*where on the edge*, not about *how big the edge is*.

## Default Values

| Parameter | Default | Source | Notes |
|-----------|---------|--------|-------|
| Pin length | 200mil | IPC-2612 convention | Override per-pin with `length:` |
| Pin pitch (gap) | 100mil | Altium standard grid | Override per-pin with `gap:` |
| Body margin | 50mil | Convention | Space from edge to first/last pin |
| Grid snap | 10mil | Altium base grid | All computed coordinates |
| Min body width | 200mil | Convention | Even with 0 top/bottom pins |
| Min body height | 200mil | Convention | Even with 0 left/right pins |
| Char width (approx) | 50mil | Altium default font | For text width estimation |

### Pin Length Heuristics

When the spec doesn't specify `length:`, the default depends on context:

| Context | Default Length | Rationale |
|---------|--------------|-----------|
| Any pin without override | 200mil | Standard IC pin length |

We keep it simple: one default. The spec author can override per-pin. We don't try
to infer "passive vs IC" — that's the spec author's job.

If we want to support a component-level default:
```spec
component R {
    defaults { pin_length: 100mil }
    body = rectangle { auto: true }
    ...
}
```

## Text Width Estimation

The body must be wide enough that pin names don't overlap. With pin names displayed
inside the body (Altium's default for `show_name: inside`):

```
          ┌──────────────────────┐
  CLK ────┤ CLK            DOUT  ├──── DOUT
  DIN ────┤ DIN             CS   ├──── CS
          └──────────────────────┘
```

Required width ≥ `max_left_name_length * char_width + max_right_name_length * char_width + center_gap`

Where `center_gap` is a small spacing so names don't touch (e.g., 50mil).

**Approximation**: We don't have real font metrics. Altium's default font (Font ID 1,
"Times New Roman" 10pt or similar) renders at roughly 50mil per character at standard
size. This is a rough estimate. We can refine later when we have access to the font
table from the document.

**When pin names are hidden**: Don't count them toward width. Check the pin's
`show_name` property.

## Implementation Approach

### Where in the Compiler

The auto-sizing logic lives in the **compiler**, not as a separate crate. It runs
during the compilation phase, before anchor resolution:

```
parse → AST
         ↓
       collect pin-to-edge assignments (from on: directives)
         ↓
       if body is auto-sized:
         compute dimensions from pin counts, gaps, text widths
         set body from:/to: coordinates
         ↓
       resolve anchors (existing logic, unchanged)
         ↓
       SpecModel with absolute coordinates
```

This keeps the auto-sizing tightly coupled with the compiler's existing understanding
of edges, anchors, and pin chains. No new crate needed.

### Compiler Changes

1. **New body syntax**: `rectangle { auto: true }` or `rectangle { auto_height: true, width: 400mil }`

2. **Pre-resolution pass**: After parsing and binding graphics, but before resolving
   anchors, walk all pin directives to:
   - Count pins per edge
   - Sum gap values from `after:`/`before:` chains
   - Estimate text widths from pin name strings
   - Compute body dimensions

3. **Apply computed dimensions**: Set the `BoxGeometry` from/to on the auto-sized body

4. **Existing anchor resolution**: Proceeds unchanged with the now-known body dimensions

### Minimal Types

```rust
/// Configuration for auto-sizing, with defaults
pub struct AutoSizeConfig {
    pub grid: Coord,              // 10mil
    pub default_pin_length: Coord,// 200mil
    pub default_pin_gap: Coord,   // 100mil
    pub body_margin: Coord,       // 50mil
    pub min_body_width: Coord,    // 200mil
    pub min_body_height: Coord,   // 200mil
    pub char_width: Coord,        // 50mil
    pub center_gap: Coord,        // 50mil (gap between left/right names)
}

impl Default for AutoSizeConfig {
    fn default() -> Self {
        AutoSizeConfig {
            grid: Coord::from_mil(10),
            default_pin_length: Coord::from_mil(200),
            default_pin_gap: Coord::from_mil(100),
            body_margin: Coord::from_mil(50),
            min_body_width: Coord::from_mil(200),
            min_body_height: Coord::from_mil(200),
            char_width: Coord::from_mil(50),
            center_gap: Coord::from_mil(50),
        }
    }
}
```

No `PinRole`, no `PinGroup`, no `Side` enum — the spec language already expresses
all of that through `on:` and `after:` directives.

## Pitfalls

### 1. Circular Dependency: Body Size ↔ Pin Position

Auto-sizing computes body dimensions from pin count/gaps. But `at: center` positions
depend on body dimensions. Solution: this isn't circular. Auto-sizing determines the
body size (a function of pin count and gaps). Then `at: center` is computed against
that size. The center of a correctly-sized body is exactly where you want centered pins.

### 2. Mixed `at:` and `after:` on the Same Edge

```spec
p1 = pin 1 { on: $body.left, at: "start" }
p2 = pin 2 { on: $body.left, after: $p1, gap: 100mil }
p3 = pin 3 { on: $body.left, at: "end" }  # also on left, but at the end
```

Here p1 and p2 form a chain from the start; p3 is independently at the end. The
auto-sizer must account for all pins — the span must be large enough for both
the chain (p1 + gap + p2 = 100mil from start) and the independent p3 at the end,
with margin.

The simplest approach: total span = chain span + gaps between chains + margins.
Pins at `at: center` are free (they don't expand the body; they land at the midpoint).
Pins at `at: start`/`at: end` with no chain following them contribute just the margin.

### 3. Grid Snapping Can Shift Pin Positions

When the body is snapped to grid, the edge positions change slightly. This shifts
all pin positions that are relative to those edges. Since pins should also be on
grid, and pin pitch is a multiple of the grid (100mil pitch on 10mil grid), this
should be fine. But verify: body_height must be a multiple of the grid AND the pin
pitch must place pins on grid points.

### 4. Multi-Part Components

Each part can have a different body rectangle. With `auto: true`, each part's body
is sized independently based on its own pins. Shared pins (`owner_part_id == 0`)
appear in all parts — they must be counted for each part's body sizing.

### 5. No Body at All

Some components are just pins and graphics (e.g., power symbols). `auto: true`
doesn't apply — the spec author draws manual graphics. The layout engine should
only activate when a body rectangle exists and is marked auto.

### 6. What About Non-Rectangular Bodies?

Round rectangles, polygons, custom shapes — `auto: true` only applies to
`rectangle` graphics. Everything else is manually specified. This keeps the scope
clean and focused.

## Example: Before and After

### Before (manual sizing)

```spec
component SPI_Flash {
    body = rectangle { from: (-200mil, -250mil), to: (200mil, 250mil) }

    p1 = pin 1 "CS"   { on: $body.left, at: "start", side: "outside", length: 200mil, electrical: "input" }
    p2 = pin 2 "MISO" { on: $body.left, after: $p1, gap: 100mil, side: "outside", length: 200mil, electrical: "output" }
    p3 = pin 3 "WP"   { on: $body.left, after: $p2, gap: 100mil, side: "outside", length: 200mil, electrical: "input" }
    p4 = pin 4 "GND"  { on: $body.left, after: $p3, gap: 100mil, side: "outside", length: 200mil, electrical: "passive" }

    p5 = pin 5 "VCC"  { on: $body.right, at: "start", side: "outside", length: 200mil, electrical: "passive" }
    p6 = pin 6 "HOLD" { on: $body.right, after: $p5, gap: 100mil, side: "outside", length: 200mil, electrical: "input" }
    p7 = pin 7 "SCK"  { on: $body.right, after: $p6, gap: 100mil, side: "outside", length: 200mil, electrical: "input" }
    p8 = pin 8 "MOSI" { on: $body.right, after: $p7, gap: 100mil, side: "outside", length: 200mil, electrical: "input" }
}
```

The author had to manually calculate: 4 pins × 100mil pitch + margins = ~500mil
height, and pick a width that fits the pin names.

### After (auto-sized)

```spec
component SPI_Flash {
    body = rectangle { auto: true }

    p1 = pin 1 "CS"   { on: $body.left, at: "start", side: "outside", electrical: "input" }
    p2 = pin 2 "MISO" { on: $body.left, after: $p1, side: "outside", electrical: "output" }
    p3 = pin 3 "WP"   { on: $body.left, after: $p2, side: "outside", electrical: "input" }
    p4 = pin 4 "GND"  { on: $body.left, after: $p3, side: "outside", electrical: "passive" }

    p5 = pin 5 "VCC"  { on: $body.right, at: "start", side: "outside", electrical: "passive" }
    p6 = pin 6 "HOLD" { on: $body.right, after: $p5, side: "outside", electrical: "input" }
    p7 = pin 7 "SCK"  { on: $body.right, after: $p6, side: "outside", electrical: "input" }
    p8 = pin 8 "MOSI" { on: $body.right, after: $p7, side: "outside", electrical: "input" }
}
```

- No `from:`/`to:` — body is auto-sized
- No `length:` — default 200mil applied
- No `gap:` — default 100mil applied
- Compiler computes: 4 pins/side × 100mil pitch + 2 × 50mil margin = 500mil height.
  Text width: max("MISO","MOSI") = 4 chars × 50mil + max("HOLD") = 4 chars × 50mil + 50mil center = 450mil width.
  Body: from (-225mil, -250mil) to (225mil, 250mil), snapped to grid.

## Crate Ecosystem Summary

**Conclusion: no external crates needed.**

The auto-sizing algorithm is ~200 lines of straightforward arithmetic on `Coord`
values. It runs inside the existing spec compiler. No constraint solvers, no geometry
crates, no graph libraries. We already have `Coord`, `CoordPoint`, `BoxGeometry`,
edge computation, and the anchor resolution pipeline.

| Crate | Verdict | Why |
|-------|---------|-----|
| kasuari (Cassowary) | Not needed | No soft constraints; body sizing is deterministic from pin counts |
| taffy (CSS layout) | Not needed | Wrong model (box flow vs pin radiation) |
| petgraph | Not needed | No graph problems in component sizing |
| euclid/geo | Not needed | We have our own Coord/CoordPoint types |
| z3 / good_lp | Not needed | No optimization; sizing is a formula |
| rstar | Not needed | No spatial queries for single-component sizing |

## Open Questions

1. **Syntax for auto-size**: `auto: true` on the rectangle? Omit `from:`/`to:` entirely
   and have the compiler infer? A component-level `defaults { ... }` block?

2. **Component-level defaults for pin properties**: Should `defaults { pin_length: 100mil }`
   be supported? Or just let the spec author set `length:` on each pin (or not, accepting
   the 200mil default)?

3. **Text width**: Use fixed char_width estimate, or try to read the font table from an
   open document for accurate metrics? Fixed is simpler and good enough for initial impl.

4. **What about the `at: "center"` single-pin case?** A component with one pin on the
   left at center — the body height is just `min_body_height`. The pin ends up centered.
   Correct behavior, but the body might look too tall for a single pin. Should the minimum
   be smaller?

5. **Override escape hatch**: If auto-sizing gets it wrong, the author just switches to
   explicit `from:`/`to:`. No need for fine-grained override flags. Keep it simple.
