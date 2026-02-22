# Coordinate System

The coordinate system for SchDoc is identical to SchLib. This document is kept for
completeness but references the shared coordinate system.

See also: [../schlib/coordinate-system.md](../schlib/coordinate-system.md)

## Internal units

All coordinates in Altium are stored in internal units where 10,000 internal units = 1 mil
(0.001 inch).

| Quantity | Internal units | Mils | mm |
|----------|---------------|------|----|
| 1 internal unit | 1 | 0.0001 | 0.0000254 |
| 1 mil | 10,000 | 1 | 0.0254 |
| 1 inch | 10,000,000 | 1,000 | 25.4 |
| 1 mm | ~393,701 | ~39.37 | 1 |

## DXP Fractional Encoding

Parameter text records split coordinates into integer and fractional parts using paired
keys:

```
LOCATION.X=100
LOCATION.X_FRAC=5000
```

Reconstruction: `raw_value = integer_part * 100,000 + fractional_part`

Source constant: `Rt_Schematic.Consts.cBaseUnit = 100000`. Each "DXP unit" (the integer
part) represents 10 mils (100,000 internal units).

So `LOCATION.X=100, LOCATION.X_FRAC=5000` gives:
`100 * 100,000 + 5,000 = 10,005,000` internal units = 1000.5 mils.

The `_FRAC` key is omitted from the parameter string when its value is zero.
The fractional part range is 0..99,999.

## SchDoc vs SchLib coordinate encoding

| Feature | SchDoc | SchLib |
|---------|--------|--------|
| Pin coordinates | DXP fractional (full precision) | Binary i16 (truncated to DXP units) |
| Pin fractional recovery | Not needed (already full precision) | PinFrac sidecar stream |
| All other records | DXP fractional | DXP fractional (identical) |
| Grid sizes | DXP fractional | DXP fractional |

In SchDoc, pins use the same DXP fractional encoding as all other records. There is no
truncation and no PinFrac sidecar stream. This is one of the key simplifications of SchDoc
over SchLib.

## Coordinate pairs in record types

| Record | Integer key | Fractional key | Description |
|--------|-------------|---------------|-------------|
| All | `LOCATION.X` | `LOCATION.X_FRAC` | X position |
| All | `LOCATION.Y` | `LOCATION.Y_FRAC` | Y position |
| Rectangle, Line, TextFrame, Image, etc. | `CORNER.X` | `CORNER.X_FRAC` | Second corner X |
| Rectangle, Line, TextFrame, Image, etc. | `CORNER.Y` | `CORNER.Y_FRAC` | Second corner Y |
| Arc, Ellipse | `RADIUS` | `RADIUS_FRAC` | Primary radius |
| Ellipse, EllipticalArc | `SECONDARYRADIUS` | `SECONDARYRADIUS_FRAC` | Secondary radius |
| Pin | `PINLENGTH` | `PINLENGTH_FRAC` | Pin length |
| Polyline, Polygon, Bezier, Wire, Bus | `X{N}` | `X{N}_FRAC` | Vertex N X (1-based) |
| Polyline, Polygon, Bezier, Wire, Bus | `Y{N}` | `Y{N}_FRAC` | Vertex N Y (1-based) |
| Grid sizes | `SnapGridSize` | `SnapGridSize_Frac` | Snap grid |
| Grid sizes | `VisibleGridSize` | `VisibleGridSize_Frac` | Visible grid |
| Grid sizes | `HotSpotGridSize` | `HotSpotGridSize_Frac` | Hotspot grid |

## Color encoding

Colors use the Win32 COLORREF format: `0x00BBGGRR` stored as a little-endian `i32`.

| Decimal | Hex | Color |
|---------|-----|-------|
| 0 | 0x00000000 | Black |
| 128 | 0x00000080 | Dark red (common for wires, text) |
| 255 | 0x000000FF | Red |
| 65280 | 0x0000FF00 | Green |
| 8388608 | 0x00800000 | Dark blue (common for wires) |
| 16711680 | 0x00FF0000 | Blue |
| 11599871 | 0x00B0FFFF | Light yellow (component fill) |
| 16317695 | 0x00F8F0FF | Lavender (default sheet background) |
| 16777215 | 0x00FFFFFF | White |

Commonly observed colors in SchDoc files:
- `128` (0x80) -- dark red: labels, designators, parameter text
- `8388608` (0x800000) -- dark blue: wires
- `255` (0xFF) -- red: compile masks, warnings, no-connect markers
- `11599871` -- light yellow: component body fill
- `16317695` -- lavender: sheet background (AreaColor in RECORD=31)
- `16777215` -- white: text frame backgrounds, dashed rectangle area color

## Angle encoding

Angles (used in SchArc, SchPie, SchEllipticalArc) are stored as floating-point degrees
in the parameter text. The `STARTANGLE` and `ENDANGLE` keys hold `f64` values.

A full circle is `STARTANGLE=0.0`, `ENDANGLE=360.0`.

Observed fractional angles: `5.595`, `174.207` -- indicating decimal degree precision.
