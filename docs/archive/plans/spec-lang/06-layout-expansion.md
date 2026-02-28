# 06 - Layout Expansion (Row / Column / Grid)

## Location

`crates/altium-format-ops/src/spec/compiler.rs` (layout expansion section)

## Purpose

Expand `row`, `column`, and `grid` layout declarations into individual `pad`
declarations with computed coordinates. This is syntactic sugar — after expansion,
the SpecModel contains only flat pad entries.

## Row Expansion

### Input

```
row {
    on: $body.left, at: center
    pitch: 0.8mm
    count: 8
    start: 1
    side: outside
    direction: forward
    pad: { shape: rectangular, x_size: 1.5mm, y_size: 0.3mm }
    skip: [3, 6]
}
```

### Algorithm

1. **Resolve anchor**: Same anchor resolution as pin placement (§05). Get the
   edge and starting position along the edge.

2. **Compute pad positions**: Generate `count` positions spaced by `pitch`
   along the edge, centered on the anchor position.

   For `at: center` with `count=8` and `pitch=0.8mm`:
   - Total span = (count - 1) * pitch = 7 * 0.8mm = 5.6mm
   - First pad offset from center = -span/2 = -2.8mm
   - Pad positions: center + [-2.8mm, -2.0mm, -1.2mm, -0.4mm, +0.4mm, +1.2mm, +2.0mm, +2.8mm]

   For `at: start`:
   - First pad at the start of the edge
   - Subsequent pads at start + i * pitch

   For `at: end`:
   - First pad at the end of the edge
   - Layout proceeds in reverse (or `direction: reverse` is implied)

3. **Apply direction**: `forward` follows the edge's natural direction
   (spec-lang.md §5.2). `reverse` inverts the pad naming order (not positions).

   | Edge | Forward |
   |------|---------|
   | left | Top-to-bottom (decreasing Y) |
   | right | Bottom-to-top (increasing Y) |
   | top | Left-to-right (increasing X) |
   | bottom | Right-to-left (decreasing X) |

4. **Name pads**: Starting from `start`, auto-increment: `start`, `start+1`,
   `start+2`, ... Skip any names in the `skip` list.

5. **Apply template**: Each generated pad gets the properties from the `pad:`
   template, plus the computed `at:` position.

6. **Apply skip**: Skip entries reference pad NAMES (not positional indices).
   Names in skip that don't match any generated name: warning.

### Absolute Row (no `on:`)

When `on:` is absent and `at:` is a coordinate:

```
row {
    at: (-150mil, 150mil)
    pitch: 100mil, count: 4, start: 1
    direction: down
    pad: { ... }
}
```

- First pad at the given coordinate
- `direction: up|down|left|right` determines axis:
  - `down`: each successive pad at y - pitch
  - `up`: each successive pad at y + pitch
  - `left`: each successive pad at x - pitch
  - `right`: each successive pad at x + pitch

Note: `up`/`down`/`left`/`right` are only valid for absolute rows. Using them
with anchor-based `on:` is an error.

## Column Expansion

`column` is syntactically and semantically identical to `row`. The keyword
exists for readability (users can use whichever feels natural for their
layout). The same expansion logic handles both.

## Grid Expansion

### Input

```
grid {
    origin: (0, 0)
    rows: 16, cols: 16
    pitch: 1mm
    naming: alphanumeric
    pad: { shape: round, x_size: 0.4mm, y_size: 0.4mm }
    skip: [H8, H9, J8, J9]
    perimeter_only: false
}
```

### Algorithm

1. **Compute positions**: For each (row, col) in the grid:
   - x = origin.x + (col - (cols-1)/2) * pitch_x
   - y = origin.y + (row - (rows-1)/2) * pitch_y

   If `pitch_x`/`pitch_y` are not specified, use `pitch` for both.

2. **Name pads**: Based on `naming`:
   - `numeric`: 1, 2, 3, ... (row-major, left-to-right, top-to-bottom)
   - `alphanumeric`: A1, A2, ..., A16, B1, ..., P16
     - Row letters: A, B, C, D, E, F, G, H, J, K, L, M, N, P, R, T, ...
       (BGA convention: skip I, O, Q, S, X, Z)
     - For grids > 20 rows: AA, AB, ... (double letters)

3. **Apply skip**: Same as row — skip by name.

4. **Apply perimeter_only**: If true, only generate pads on the outer ring
   (row 0, row N-1, col 0, col N-1).

5. **Apply template**: Each pad gets template properties + computed position.

## Override Semantics

When a `row`/`column`/`grid` generates a pad with name N, and an explicit
`pad N { ... }` also exists in the same footprint:

- Explicit fields override the template
- Template fields are inherited for fields NOT in the explicit declaration
- Position from the layout is overridable via explicit `at:`
- This is NOT a duplicate error

```rust
/// Merge layout-generated pad with explicit override.
fn merge_pad_override(
    generated: &mut PadProperties,
    explicit: &PadProperties,
) {
    // For each field in explicit, override the generated value
    if let Some(shape) = &explicit.shape { generated.shape = Some(shape.clone()); }
    if let Some(at) = &explicit.at { generated.at = Some(at.clone()); }
    // ... etc for all pad fields
}
```

## Output

The expansion phase produces a flat `Vec<PadSpec>` (or analogous type in the
SpecModel) with absolute coordinates. The row/column/grid structures do not
appear in the SpecModel — they are fully desugared.

## Test Strategy

- Row on each edge (left, right, top, bottom) with center alignment
- Row with start/end alignment
- Absolute row with up/down/left/right directions
- Grid with numeric and alphanumeric naming
- Grid with skip (verify correct pads omitted)
- Grid with perimeter_only
- Pad override: explicit pad modifies template-generated pad
- Skip with no matching name: warning
- Anchor-based row with absolute direction (up/down/left/right): error
- Row pitch validation (must be positive)
- Grid with asymmetric pitch (pitch_x != pitch_y)
