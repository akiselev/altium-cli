# Coordinate System

PCB coordinates (both PcbLib and PcbDoc) use a unified internal unit system that differs
significantly from the schematic coordinate system.

## Internal units

- **1 mil = 10,000 internal units** (i32)
- **1 mm = 393,701 internal units** (approximately)
- Coordinates are stored as `i32` little-endian values
- The origin (0,0) is at the board origin in PcbDoc, or the footprint origin in PcbLib

## Type mapping

| Altium concept | Internal representation | Rust type |
|----------------|----------------------|-----------|
| Position X/Y | i32 internal units | `Coord` (newtype over i32) |
| Width/Height | i32 internal units | `Coord` |
| Point (X,Y) | (i32, i32) | `CoordPoint` |
| Angle | f64 degrees | `f64` |
| Boolean | u8 (0=false, nonzero=true) | `bool` |
| Layer | u8 | `Layer` (newtype) |
| Flags | u16 little-endian | `PcbFlags` (bitflags) |

## Comparison with schematic coordinates

| Aspect | Schematic (SchLib/SchDoc) | PCB (PcbLib/PcbDoc) |
|--------|--------------------------|---------------------|
| Unit | DXP unit = 100,000 internal | 1 mil = 10,000 internal |
| Storage | i16 DXP + i32 fractional remainder | i32 internal units directly |
| Range | ±3,276.7 DXP units ≈ ±32,767 mils | ±214,748 mils ≈ ±5,456 mm |
| Fractional | PinFrac sidecar stream | Built into i32 precision |
| Resolution | 1/100,000 DXP unit | 1/10,000 mil = 0.1 µm |

## Colors

PCB files use Win32 COLORREF format: `0x00BBGGRR` (BGR, not RGB).

- Stored as `u32` (or `i32` cast to unsigned)
- Red channel in lowest byte
- Blue channel in highest non-zero byte
- Alpha is not used (always 0x00 in high byte)

Example: `0x00FF0000` = pure blue (not red!)

## Coordinate display

Altium displays coordinates in user-selected units (mils or mm). For parsing purposes,
all values are stored in internal units and unit conversion is a display concern.

Conversion formulas:
- Internal → mils: `value / 10_000`
- Internal → mm: `value / 393_701.0`
- Mils → internal: `value * 10_000`
- mm → internal: `value * 393_701.0` (round to nearest i32)

## String-encoded coordinates

The PcbLib Parameters stream and Library/Data stream use string-encoded coordinates with
unit suffixes:

| Format | Example | Internal value |
|--------|---------|---------------|
| `Nmil` | `21.6535mil` | 216,535 |
| `Nmm` | `1.0mm` | 393,701 |
| `N` (bare number) | `0` | 0 |

These string values need a dedicated parser that handles the unit suffix.

## Rotation

Rotation angles are stored as `f64` (IEEE 754 double-precision) in degrees:
- 0.0 = no rotation
- 90.0 = 90° counter-clockwise
- Range: 0.0 to 360.0 (may also use negative values)

## Footprint origin

In PcbLib, each footprint has its own local coordinate system:
- Origin (0,0) is the footprint reference point
- Pads, tracks, text, etc. are positioned relative to this origin
- When placed on a board (PcbDoc), the footprint origin maps to the component's placement
  location, and all primitive coordinates are transformed accordingly
