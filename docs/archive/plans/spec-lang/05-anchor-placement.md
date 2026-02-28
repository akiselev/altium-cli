# 05 - Anchor-Based Placement

## Location

`crates/altium-format-ops/src/spec/compiler.rs` (anchor resolution section)

## Purpose

Resolve anchor-based pin/pad placement (`on: $body.left, at: center`) into
absolute coordinates. This is the core geometric computation that makes the spec
language ergonomic.

## Anchor Reference Model

A bound graphic (e.g., `body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }`)
exposes named anchors:

### Box Anchors (rectangle, round_rectangle, text_frame, image)

```
        top
  TL ─────────── TR
  |               |
left    center   right
  |               |
  BL ─────────── BR
       bottom
```

| Anchor | Type | Value (for rectangle from `(x1,y1)` to `(x2,y2)`) |
|--------|------|------|
| `top` | Edge | y = max(y1, y2), x range [min(x1,x2), max(x1,x2)] |
| `bottom` | Edge | y = min(y1, y2), x range [min(x1,x2), max(x1,x2)] |
| `left` | Edge | x = min(x1, x2), y range [min(y1,y2), max(y1,y2)] |
| `right` | Edge | x = max(x1, x2), y range [min(y1,y2), max(y1,y2)] |
| `center` | Point | ((x1+x2)/2, (y1+y2)/2) |
| `top_left` | Point | (min(x1,x2), max(y1,y2)) |
| `top_right` | Point | (max(x1,x2), max(y1,y2)) |
| `bottom_left` | Point | (min(x1,x2), min(y1,y2)) |
| `bottom_right` | Point | (max(x1,x2), min(y1,y2)) |

### Data Structures

```rust
/// An edge of a bounding box.
pub struct Edge {
    pub axis: Axis,         // X (vertical edge) or Y (horizontal edge)
    pub position: Coord,    // fixed coordinate
    pub range: (Coord, Coord), // min..max along the other axis
    pub side: EdgeSide,     // which side: Left, Right, Top, Bottom
}

pub enum Axis { X, Y }
pub enum EdgeSide { Left, Right, Top, Bottom }

/// Result of resolving an anchor reference.
pub enum AnchorValue {
    Edge(Edge),
    Point(CoordPoint),
}
```

## Placement Algorithm

Given a pin/pad with anchor-based properties:

```
pin 1 { on: $body.left, at: center, side: outside, gap: 100mil, length: 25 }
```

### Step 1: Resolve the edge

`$body.left` -> `Edge { axis: X, position: x_min, range: (y_min, y_max), side: Left }`

### Step 2: Compute position along the edge

The `at:` property determines where along the edge:

| `at` value | Position |
|------------|----------|
| `start` | For left/right edges: y_max (top). For top/bottom edges: x_min (left) |
| `center` | Midpoint of the edge range |
| `end` | For left/right edges: y_min (bottom). For top/bottom edges: x_max (right) |

This gives a point ON the edge.

### Step 3: Compute `after:` / `before:` positioning

If `after: $p2` is specified instead of `at:`:
1. Find `$p2`'s position on the same edge
2. Place the current entity at `$p2.position + gap` (in the edge's direction)

The `forward` direction for each edge (spec-lang.md §5.2):

| Edge | Forward direction | Coordinate change |
|------|-------------------|-------------------|
| left | Top-to-bottom | Decreasing Y |
| right | Bottom-to-top | Increasing Y |
| top | Left-to-right | Increasing X |
| bottom | Right-to-left | Decreasing X |

`after: $p2, gap: 60mil` means: place 60mil after $p2 in the forward direction.

### Step 4: Offset from edge (side)

The `side:` property determines the offset direction:

| Edge | `outside` | `inside` | `center` |
|------|-----------|----------|----------|
| left | Extend to the left (-X) | Extend to the right (+X) | Centered on edge |
| right | Extend to the right (+X) | Extend to the left (-X) | Centered on edge |
| top | Extend upward (+Y) | Extend downward (-Y) | Centered on edge |
| bottom | Extend downward (-Y) | Extend upward (+Y) | Centered on edge |

For pins, `side: outside` means the pin connection point is away from the body,
and the pin extends OUTWARD from the edge by `length`. The pin's `location`
(its connection point, where wires attach) is at `edge_position - side_direction * length`.
The pin's visible end is at `edge_position`.

### Step 5: Compute orientation (auto)

When `orientation: auto` (the default with anchor placement):

| Edge | Orientation (degrees) | Meaning |
|------|----------------------|---------|
| left | 0 | Pin points right (connects from the left) |
| right | 180 | Pin points left (connects from the right) |
| top | 270 | Pin points down (connects from the top) |
| bottom | 90 | Pin points up (connects from the bottom) |

### Step 6: Apply offset

If `offset: (dx, dy)` is specified, add it to the computed position as a
post-placement translation.

## Validation

### Same-edge constraint

`after:` and `before:` references must be on the SAME edge as the current
entity's `on:`. If not:

```
error[E_CROSS_EDGE_REFERENCE]: pin '$p2' (on $body.right) is not on
  the same edge as pin '3' (on $body.left)
```

### No anchor + absolute

If `on:` is present and `at:` is a coordinate (not an enum), this is an error:
anchor mode and absolute mode are mutually exclusive.

### Corner anchors not for `on:`

Corner anchors (`$body.top_left`, etc.) are points, not edges. Using them with
`on:` is an error — they can only be used in coordinate expressions.

### Mutual exclusivity

`at:` (enum), `after:`, and `before:` are mutually exclusive in anchor mode.

## Implementation

```rust
/// Resolve anchor-based placement to absolute coordinates and orientation.
pub fn resolve_anchor_placement(
    on_edge: &Edge,
    at_position: AnchorPosition,     // Start, Center, End, AfterRef, BeforeRef
    side: PlacementSide,             // Inside, Outside, Center
    gap: Coord,
    pin_length: Coord,
    offset: Option<CoordPoint>,
) -> Result<(CoordPoint, RotationBy90), SpecError>
```

The compiler calls this for each pin/pad that uses anchor-based placement.
The result is stored in the SpecModel with absolute coordinates — downstream
code never sees anchor references.

## Gap Accumulation for after/before

When multiple pins are placed sequentially on the same edge using `after:`,
each pin's position depends on the previous one. The compiler processes pins
in declaration order within a scope, but since forward references are valid
(§9.1), it must handle arbitrary ordering.

**Strategy**: Build a dependency graph of `after:`/`before:` references within
each edge, topologically sort, then compute positions in order. Cycles are an
error.

## Test Strategy

- All four edges with `at: start`, `at: center`, `at: end`
- `after:` chaining (3+ pins on same edge)
- `before:` references
- `side: inside` vs `outside` vs `center`
- `offset:` post-placement translation
- Auto orientation inference
- Cross-edge reference error
- Corner anchor rejection for `on:`
- Absolute placement (no anchors) — passthrough
- Mixed: some pins anchor-based, some absolute in same component
