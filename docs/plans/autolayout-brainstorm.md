# Auto-Sizing & Default Geometry for Spec Language

## Problem Statement

The spec language already handles the *logical* layout of a schematic component:
- Which side each pin goes on (`on: $body.left`)
- Pin ordering and spacing (`after: $p1, gap: 5mil`)
- Relative positioning (`at: center`, `at: start`, `at: end`)
- Pin orientation (auto-computed from edge)

But the spec author (human or LLM) must currently hand-compute concrete geometry:
- The body rectangle dimensions (`from: (-200mil, -300mil), to: (200mil, 300mil)`)
- Pin lengths (`length: 200mil`)
- Pin-to-pin gaps (`gap: 100mil`)
- The exact `from:`/`to:` coordinates that make the body big enough for all the pins

This is tedious and error-prone. An LLM generating a spec for a 40-pin IC has to
manually count pins per side, multiply by pitch, account for text widths, and produce
exact coordinates. The result is often wrong — tiny bodies, absurd pin lengths, pins
that don't fit.

**What we want**: The spec says *what goes where* (sides, ordering, grouping). A
unified layout pass figures out body size, pin lengths, pin gaps, and pin positions
all at once, because they all depend on each other.

## Scope: What This Does and Doesn't Do

### In Scope

1. **Auto-size body rectangle** — compute `from:`/`to:` to fit all pins with margins
2. **Default pin length** — 200mil when spec doesn't specify `length:`
3. **Default pin pitch** — 100mil when `gap:` isn't specified in `after:` chains
4. **Pin placement** — compute absolute pin positions as part of the same pass
5. **Text-aware width** — body wide enough that pin names don't overlap inside
6. **Grid snapping** — all computed coordinates on 10mil grid

### Out of Scope

- **Pin-to-side assignment** — the spec says `on: $body.left`; we don't guess
- **Pin ordering** — the spec says `after: $p1`; we don't reorder
- **Pin grouping/classification** — no name heuristics, no PinRole inference
- **Custom graphics** — resistor zig-zags, op-amp triangles, logic gate shapes are
  manual `draw` specs. Layout only handles rectangular body sizing.
- **SchDoc sheet layout** — placing components on sheets, wire routing
- **PcbDoc placement** — different domain

## Why a Unified Pass (Not Pre-Compute Body, Then Resolve Anchors)

The previous version of this document proposed computing body dimensions first, then
running the existing anchor resolver unchanged. That doesn't work well because:

1. **Body size, pin length, pin gap, and pin position are interdependent.** The body
   height depends on how many pins are on each vertical edge × their gaps. The body
   width depends on pin name text widths. Pin connection point depends on body edge
   position + pin length. All of these feed into each other.

2. **The existing anchor resolver uses the body as a given.** It looks up `BoxGeometry`
   from the binding map, then computes pin positions relative to those edges. If we
   compute the body first and then hand it to the existing resolver, we've just split
   one coherent computation into two halves that need to agree on all the same defaults.

3. **Pin defaults must be resolved before body sizing.** We need to know the effective
   pin length (user-specified or 200mil default) and effective gap (user-specified or
   100mil default) *before* we can compute body dimensions. But we also need those
   resolved values *during* pin placement. Doing it in one pass avoids double-work.

4. **The current anchor resolver hardcodes `Coord::from_mils(25)` as pin length default
   and `0` as gap default.** These are wrong for auto-layout. We need to change these
   defaults anyway, so we should do it in one coherent place rather than patching the
   existing code path and adding a separate pre-computation path.

## How It Works: The Unified Auto-Layout Pass

### Current Compiler Flow

```
parse → AST
  ↓
build_graphic_binding_map()    ← extracts BoxGeometry from bound rectangles
  ↓
resolve_anchor_pins()          ← classify pins, topo sort, compile_one_pin()
  ↓
SpecModel with absolute coordinates
```

Key existing functions:
- `build_graphic_binding_map()` — scans `body = rectangle { from: ..., to: ... }` and
  stores `BoxGeometry { from, to }` in a `HashMap<String, BoxGeometry>`. Skips if
  `from:` or `to:` is missing.
- `resolve_anchor_pins()` — classifies each pin into `PinAnchorMode` (Absolute,
  AtPosition, After, Before), topo-sorts by dependencies, then calls
  `compile_one_pin()` for each in order.
- `compile_one_pin()` — looks up edge from `BoxGeometry`, calls
  `resolve_anchor_placement()` to get absolute coordinates.

### Proposed Flow

```
parse → AST
  ↓
build_graphic_binding_map()    ← same, BUT for auto bodies, stores placeholder
  ↓
if any auto body exists:
  auto_layout_pass()           ← NEW: unified body sizing + pin placement
else:
  resolve_anchor_pins()        ← existing logic, unchanged
  ↓
SpecModel with absolute coordinates
```

When a bound rectangle has `auto: true` (or simply omits `from:`/`to:`), the compiler
takes a different path that computes everything together.

### The auto_layout_pass() Algorithm

```
INPUT:
  - Pin declarations with: on: $body.<edge>, at:/after:/before:, gap:, length:, name:
  - Auto-sized body binding (e.g., "body")
  - AutoSizeConfig (defaults for length, gap, margin, grid, etc.)

STEP 1: CLASSIFY & COLLECT
  For each pin, extract:
  - Which edge (left/right/top/bottom) from `on:` directive
  - Anchor mode (AtPosition/After/Before) — same as current classify_pin
  - Effective pin length = spec `length:` value OR default (200mil)
  - Effective gap = spec `gap:` value OR default (100mil)
  - Pin name (for text width estimation)
  Group pins by edge. Build same dependency graph as current topo_sort_pins().

STEP 2: COMPUTE EDGE SPANS
  For each edge, walk the after:/before: chains in topo order to sum the total
  span needed:

  For a chain like: p1(at:start) → p2(after:p1, gap:100mil) → p3(after:p2, gap:100mil)
    chain_span = sum of gaps = 200mil

  For standalone pins using `at:start`/`at:end`: contribute margin but not span
  For `at:center`: free (doesn't expand body)

  edge_span = chain_span + 2 * margin

  Per-edge:
    left_span   = compute from left pin chains + margins
    right_span  = compute from right pin chains + margins
    top_span    = compute from top pin chains + margins
    bottom_span = compute from bottom pin chains + margins

STEP 3: SIZE BODY
  body_height = max(left_span, right_span, min_body_height)
  body_width  = max(top_span, bottom_span, min_body_width, text_width)

  Where text_width accounts for pin names displayed inside the body:
    text_width = max_left_name_width + max_right_name_width + center_gap
    (approximated as char_count × char_width)

  Snap to grid:
    body_height = round_up(body_height, grid)
    body_width  = round_up(body_width, grid)

  Center at origin:
    from = (-body_width/2, -body_height/2)
    to   = (body_width/2, body_height/2)

STEP 4: PLACE PINS
  Now that we know the body edges, place each pin in topo-sorted order:

  For AtPosition pins:
    Use resolve_anchor_placement() with computed edges (same as current logic)
    But with the resolved effective pin_length, not the 25mil default

  For After/Before pins:
    Same as current logic, but:
    - Use effective gap (100mil default) instead of 0
    - Use effective pin_length (200mil default) instead of 25mil

  Store positions in cache for dependency resolution (same as current).

STEP 5: UPDATE BINDING MAP
  Insert the computed BoxGeometry into the binding map so that:
  - The graphic spec gets the correct from:/to: coordinates
  - Any other references to the body (e.g., from other graphics) work

OUTPUT:
  - Vec<PinSpec> with absolute coordinates (same as current resolve_anchor_pins)
  - Updated BoxGeometry for the body rectangle (written back to binding map + graphic)
```

### Why This Is Better Than Two Separate Passes

- **One source of truth for defaults**: Pin length default (200mil) and gap default
  (100mil) are applied once, used for both body sizing and pin placement.
- **No coordination problem**: The body size is computed from the same effective values
  used for pin placement. They can't disagree.
- **Simpler mental model**: "auto-layout computes the body and pins together" vs
  "pre-compute body, then hope the anchor resolver uses the same defaults."
- **Existing non-auto path is unchanged**: Components with explicit `from:`/`to:` keep
  using the existing `resolve_anchor_pins()` code path.

## Current Defaults vs Proposed Defaults

| Value | Current Default | Proposed Default | Why |
|-------|----------------|-----------------|-----|
| Pin length | 25mil | 200mil | 25mil is Altium's internal minimum; 200mil is the IPC-2612 convention for IC pins |
| Pin gap | 0 | 100mil | 0 gap means pins stack on top of each other; 100mil is standard Altium grid pitch |
| Grid snap | none | 10mil | Altium's base grid |
| Body margin | n/a | 50mil | Space between edge of body and first/last pin attachment point |
| Min body size | n/a | 200×200mil | Reasonable minimum for any component body |
| Char width | n/a | ~50mil | Approximate for Altium's default font |

**Important**: The default changes only apply in auto-layout mode. The existing code
path (explicit `from:`/`to:`) keeps its current 25mil/0 defaults for backward
compatibility. We could consider changing those too, but that's a separate decision.

## Spec Syntax

### Minimal: Omit `from:`/`to:`

```spec
component SPI_Flash {
    body = rectangle { }  # or: body = rectangle { auto: true }

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

No `from:`/`to:`, no `length:`, no `gap:`. The compiler detects that the body
has no coordinates and enters auto-layout mode. Body sized to 4 pins × 100mil +
margins. Pin lengths default to 200mil.

### With Overrides

```spec
component SPI_Flash {
    body = rectangle { auto: true, is_solid: true, color: "#FF0000" }

    # Override pin length for specific pins
    p1 = pin 1 "CS" { on: $body.left, at: "start", side: "outside", length: 300mil }
    # Override gap for a group break
    p3 = pin 3 "WP" { on: $body.left, after: $p2, gap: 200mil, side: "outside" }
}
```

### Explicit Body (Existing Behavior, Unchanged)

```spec
component SPI_Flash {
    body = rectangle { from: (-200mil, -250mil), to: (200mil, 250mil) }
    p1 = pin 1 "CS" { on: $body.left, at: "start", side: "outside", length: 200mil }
    ...
}
```

Existing path. No auto-layout. Current defaults (25mil length, 0 gap).

### Detection Logic

The compiler enters auto-layout mode when a bound box graphic is referenced by pins
via `on:` but has **no `from:` or `to:` properties** (or has `auto: true`). This is
detected in `build_graphic_binding_map()` — instead of skipping unresolved boxes (as
it does now), it marks them as needing auto-layout.

## Implementation Plan

### 1. Detect Auto-Layout Bodies

In `build_graphic_binding_map()`, when a bound box-type graphic has no `from:`/`to:`:
- Don't skip it (current behavior: `continue`)
- Instead, mark it in a new `HashSet<String>` of auto-layout bindings
- Insert a placeholder `BoxGeometry` (will be overwritten)

### 2. New Function: `auto_layout_pins()`

A new function that replaces `resolve_anchor_pins()` when auto-layout bindings exist.
It does everything `resolve_anchor_pins()` does, but with the unified pass:

```rust
fn auto_layout_pins(
    pin_decls: &[(&PinDecl, i32)],
    binding_map: &mut GraphicBindingMap,  // mut: we write back computed body
    auto_bindings: &HashSet<String>,
    scope: &ScopeStack,
    config: &AutoSizeConfig,
) -> Result<Vec<PinSpec>, SpecError>
```

Steps:
1. Classify pins (same as current `resolve_anchor_pins`)
2. Topo sort (same as current)
3. **For each auto binding**: walk pin chains to compute edge spans
4. **Compute body dimensions** and insert into `binding_map`
5. **Place pins** using the computed body (same as current `compile_one_pin` but with
   new defaults)

### 3. Updated Defaults in Auto Mode

In `compile_one_pin` (or the auto-layout equivalent):
- `pin_length` default: `config.default_pin_length` (200mil) instead of 25mil
- `gap` default: `config.default_pin_gap` (100mil) instead of 0

### 4. Update Graphic Output

After auto-sizing, the computed `from:`/`to:` must be reflected in the `GraphicSpec`
output for the body rectangle, so it appears correctly in the saved SchLib.

### 5. Backward Compatibility

- Existing specs with explicit `from:`/`to:` use the current code path unchanged
- No behavior change for non-auto components
- The only default changes are in the new auto-layout path

## Edge Span Computation: Details

### Simple Chain

```
p1(at:start) → p2(after:p1) → p3(after:p2) → p4(after:p3)
```

With default 100mil gap:
- p1 at margin offset from edge start
- p2 at p1 + 100mil
- p3 at p2 + 100mil
- p4 at p3 + 100mil

Total span = 3 × 100mil + 2 × margin = 300 + 100 = 400mil

General: `(n-1) × gap + 2 × margin` for n pins with uniform gap.

### Chain with Varying Gaps

```
p1(at:start) → p2(after:p1, gap:100mil) → p3(after:p2, gap:200mil) → p4(after:p3, gap:100mil)
```

Total span = 100 + 200 + 100 + 2 × margin = 400 + 100 = 500mil

### Multiple Independent Chains on Same Edge

```
p1(at:start) → p2(after:p1)          # chain A: starts at start
p3(at:end) → p4(before:p3)           # chain B: starts at end
```

Chain A span = 1 × gap = 100mil (plus margin at start)
Chain B span = 1 × gap = 100mil (plus margin at end)

Total edge span = chain_A + chain_B + gap_between_chains
= 100 + 100 + margin_start + margin_end + inter_chain_gap

The inter-chain gap is the tricky part. If both chains are anchored to
start/end with no explicit middle pin, the body must be at least
`chain_A_span + chain_B_span + margin_between`. For simplicity, we can
require that the total span accommodates all chains without overlap.

### Pins at `at:center`

Pins at `at:center` don't contribute to edge span — they land at the midpoint
of whatever size the body ends up being. They're "free" from a sizing perspective.

But: if the only pin on an edge is `at:center`, the edge span is just `2 × margin`
(the minimum to have an edge at all).

## Text Width Estimation

Pin names are displayed inside the body by default (Altium's standard). The body
must be wide enough to accommodate the longest name on each side:

```
          ┌──────────────────────────┐
  CLK ────┤ CLK              DATAOUT ├──── DATAOUT
  DIN ────┤ DIN                  CS  ├──── CS
          └──────────────────────────┘
```

Width ≥ `max_left_name_len × char_width + max_right_name_len × char_width + center_gap`

For the SPI flash example:
- Left names: CS(2), MISO(4), WP(2), GND(3) → max = 4
- Right names: VCC(3), HOLD(4), SCK(3), MOSI(4) → max = 4
- Width ≥ 4×50 + 4×50 + 50 = 450mil

We only count horizontal edges (left/right) for width estimation, and vertical edges
(top/bottom) for height estimation. Pin names on top/bottom edges are rotated, but
for simplicity we use the same char_width approximation.

## Pitfalls

### 1. What if pins reference an auto-sized body AND a fixed-size body?

A component could have both a fixed rectangle and an auto-sized rectangle. Each
binding is independent — auto-layout only applies to bindings marked auto.

### 2. Grid Snapping and Pin Alignment

When the body is snapped to grid, edge positions change slightly. Pin gaps should
also be grid-aligned. Since default gap (100mil) and grid (10mil) are compatible
(100 is a multiple of 10), this should be fine. But custom gaps (e.g., 75mil) on
a 10mil grid should be rounded to 80mil.

### 3. Multi-Part Components

Each part can have its own body rectangle. Auto-layout sizes each independently.
Shared pins (part 0) appear in all parts — they must be counted for each part's
body, but their positions may differ between parts.

### 4. Parts with Different Pin Counts

Part 1 has 8 pins on left, Part 2 has 3 pins on left. Each part gets its own
body sized to its pins. The bodies will be different sizes. This is correct —
multi-part components often have different-sized parts.

### 5. Custom Graphics Alongside Auto Body

```spec
component MyChip {
    body = rectangle { auto: true }
    line { from: (-50mil, 0mil), to: (-30mil, 20mil) }  # clock wedge
    ...
}
```

Auto-layout sizes the body and places pins. Other graphics use absolute coordinates
and are not affected. The spec author is responsible for positioning custom graphics
relative to the body (they could use the body binding: `$body.center`, etc.).

### 6. `round_rectangle` Auto-Sizing

`round_rectangle` is also a box-type graphic. Auto-sizing should work the same way —
compute `from:`/`to:` and let the existing corner radius properties apply. The edges
are the same as a regular rectangle (the corners are just rounded visually).

## Types

```rust
/// Configuration for the auto-layout pass.
pub struct AutoSizeConfig {
    /// Snap grid for body dimensions (default: 10mil)
    pub grid: Coord,
    /// Default pin stub length when not specified (default: 200mil)
    pub default_pin_length: Coord,
    /// Default gap between pins in after:/before: chains (default: 100mil)
    pub default_pin_gap: Coord,
    /// Margin from body edge to first/last pin on each side (default: 50mil)
    pub body_margin: Coord,
    /// Minimum body width (default: 200mil)
    pub min_body_width: Coord,
    /// Minimum body height (default: 200mil)
    pub min_body_height: Coord,
    /// Approximate width per character for text width estimation (default: 50mil)
    pub char_width: Coord,
    /// Gap between left and right pin name columns inside body (default: 50mil)
    pub center_gap: Coord,
}
```

## Example Walkthrough

### Input Spec

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

### Step 1: Classify

| Pin | Edge | Mode | Gap | Length | Name |
|-----|------|------|-----|--------|------|
| p1  | left | AtPosition(start) | - | default 200mil | "CS" |
| p2  | left | After(p1) | default 100mil | default 200mil | "MISO" |
| p3  | left | After(p2) | default 100mil | default 200mil | "WP" |
| p4  | left | After(p3) | default 100mil | default 200mil | "GND" |
| p5  | right | AtPosition(start) | - | default 200mil | "VCC" |
| p6  | right | After(p5) | default 100mil | default 200mil | "HOLD" |
| p7  | right | After(p6) | default 100mil | default 200mil | "SCK" |
| p8  | right | After(p7) | default 100mil | default 200mil | "MOSI" |

### Step 2: Compute Edge Spans

Left: p1 → p2(100) → p3(100) → p4(100) = 300mil chain + 2 × 50mil margin = 400mil
Right: p5 → p6(100) → p7(100) → p8(100) = 300mil chain + 2 × 50mil margin = 400mil
Top: 0 pins = 0
Bottom: 0 pins = 0

### Step 3: Size Body

body_height = max(400, 400, 200) = 400mil
text_width = max("MISO") × 50 + max("MOSI") × 50 + 50 = 200 + 200 + 50 = 450mil
body_width = max(0, 0, 200, 450) = 450mil

Snap: 400mil (already on grid), 450mil (already on grid)

Center: from = (-225mil, -200mil), to = (225mil, 200mil)

### Step 4: Place Pins

Left edge: x = -225mil (from.x), y range = [-200mil, 200mil]
- Forward direction for left edge = -1 (top to bottom = decreasing Y)
- Start = max Y = 200mil

p1: at start → along = 200mil - 50mil(margin) = 150mil, x = -225mil - 200mil(length) = -425mil
  → location = (-4,250,000, 1,500,000), orientation = Rotate0

p2: after p1, gap 100mil → along = 150mil + (-1)(100mil) = 50mil
  → location = (-4,250,000, 500,000)

p3: after p2 → along = 50mil + (-1)(100mil) = -50mil
  → location = (-4,250,000, -500,000)

p4: after p3 → along = -50mil + (-1)(100mil) = -150mil
  → location = (-4,250,000, -1,500,000)

Right edge: x = 225mil (to.x), y range = [-200mil, 200mil]
- Forward direction for right edge = +1 (bottom to top = increasing Y)
- Start = min Y = -200mil

p5: at start → along = -200mil + 50mil(margin) = -150mil, x = 225mil + 200mil(length) = 425mil
  → location = (4,250,000, -1,500,000), orientation = Rotate180

(etc.)

### Result

An 8-pin DIP-style symbol with:
- 450mil × 400mil body centered at origin
- 4 pins on each side, 100mil apart, 200mil stub length
- Pin names fit inside the body
- Everything on 10mil grid

## Crate Ecosystem Summary

**No external crates needed.**

The unified auto-layout pass is ~300 lines of `Coord` arithmetic inside the existing
spec compiler. It reuses existing types (`BoxGeometry`, `Edge`, `PendingPin`,
`PinAnchorMode`) and the existing topo sort. The only new code is the edge span
computation and body sizing logic.

## Open Questions

1. **Detection mechanism**: Is "no `from:`/`to:`" sufficient to trigger auto-layout?
   Or should we require explicit `auto: true`? The former is more ergonomic; the
   latter is more explicit. Recommendation: accept both — missing coords OR
   `auto: true` triggers it.

2. **Component-level defaults**: Should we support `defaults { pin_length: 100mil }`
   at the component level? Useful for passives where 100mil is standard. Could be
   done later without architecture changes.

3. **How does `at: "start"` interact with margin?** Currently `at: start` means the
   literal start of the edge range (min or max Y depending on forward direction). In
   auto-layout, we want "start" to mean "start + margin" so pins aren't flush with
   the body corner. This requires adjusting the edge range by margin before passing
   to `resolve_anchor_placement()`.

4. **What about `at: "center"` on auto body?** If the only pin on an edge is
   `at: center`, the body is `min_body_height` and the pin is centered. If there are
   also `after:` chains, the body is sized to the chains and the center pin floats
   in the middle. Both seem correct. But what if `at: center` and `at: start` are
   both on the same edge with no chain connecting them? The body should be at least
   `2 × margin` to have room for both, but the center pin doesn't contribute to span.

5. **Should the non-auto path also get updated defaults?** Changing pin length from
   25mil to 200mil and gap from 0 to 100mil for the non-auto path would be a breaking
   change for existing specs. Probably best to leave it alone for now.
