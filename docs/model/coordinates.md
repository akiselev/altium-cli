# Coordinate System

Altium uses a fixed-point integer coordinate system throughout both schematic
and PCB domains.

## Internal Representation

```rust
/// Internal coordinate value, stored as a fixed-point integer.
/// Defined in crates/altium-format/src/types/coord.rs
pub struct Coord(i32);

/// 2D point with X and Y coordinates.
pub struct CoordPoint {
    pub x: Coord,
    pub y: Coord,
}

/// Axis-aligned bounding rectangle.
pub struct CoordRect {
    pub min: CoordPoint,
    pub max: CoordPoint,
}
```

**Resolution**: 10,000 internal units = 1 mil (0.001 inch).

| Value | Internal units | Mils | Inches | Millimeters |
|-------|---------------|------|--------|-------------|
| 1 internal unit | 1 | 0.0001 | 0.0000001 | 0.0000254 |
| 1 mil | 10,000 | 1 | 0.001 | 0.0254 |
| 1 inch | 10,000,000 | 1,000 | 1 | 25.4 |
| 1 mm | ~393,701 | ~39.37 | ~0.03937 | 1 |

The `Coord` type provides conversion methods: `from_mils()`, `from_mms()`,
`to_mils()`, etc.

## DXP Fractional Encoding (Schematic Parameters)

In schematic parameter strings, coordinates are split into **two separate
parameters**: an integer part and a fractional part.

```
LOCATION.X=100
LOCATION.X_FRAC=5000
```

The raw coordinate value is reconstructed as:

```
raw = integer_part * 10,000 + fractional_part
```

So `LOCATION.X=100, LOCATION.X_FRAC=5000` → `raw = 1,005,000` → `100.5 mils`.

### Encoding rules

- `integer_part` = `raw / 10,000` (integer division, can be negative)
- `fractional_part` = `raw % 10,000` (remainder, always 0..9999 for
  canonical form)
- When `fractional_part` is 0, the `_FRAC` parameter is typically omitted.
- **Non-canonical values** are accepted on read (e.g., `RADIUS=14,
  RADIUS_FRAC=85746` decodes to `raw = 225,746`, same as canonical
  `RADIUS=22, RADIUS_FRAC=5746`). The library normalizes to canonical form
  on write.

### Field attribute

In the derive macro, fractional coordinates use the `frac` attribute:

```rust
#[altium(param = "LOCATION.X", frac = "LOCATION.X_FRAC")]
pub location_x: i32,
```

This generates code that reads both parameters and combines them into a single
`i32` raw value, and splits them back on write.

## Indexed Vertex Coordinates

Polylines, polygons, and beziers store variable-length vertex arrays using
indexed parameters:

```
LOCATIONCOUNT=3
X1=100
X1_FRAC=0
Y1=200
Y1_FRAC=0
X2=300
X2_FRAC=0
Y2=400
Y2_FRAC=0
X3=500
X3_FRAC=0
Y3=600
Y3_FRAC=0
```

The derive macro handles this with the `indexed_coords` attribute:

```rust
#[altium(indexed_coords, prefix_x = "X", prefix_y = "Y", count = "LOCATIONCOUNT")]
pub vertices: Vec<(i32, i32)>,
```

Indices are 1-based (X1, X2, X3, ...).

## PCB Binary Coordinates

In PCB binary records, coordinates are stored directly as **i32 little-endian**
values with the same 10,000 units/mil resolution. No fractional split is needed
since the full i32 is stored directly.

```
[i32 x][i32 y]   ← CoordPoint: two consecutive little-endian i32 values
```

## Color Encoding

Colors use the Win32 `COLORREF` format: a 32-bit integer with bytes in
`0x00BBGGRR` order (blue in high byte, red in low byte).

```
COLOR=128      → 0x00000080 → R=128, G=0, B=0 (dark red)
COLOR=16711680 → 0x00FF0000 → R=0, G=0, B=255 (blue)
```

Stored as `i32` in parameter format. The `Color` type provides conversion
methods.
