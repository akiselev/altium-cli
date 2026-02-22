# Coordinate System

## Internal units

All coordinates in Altium are stored in internal units where 10,000 internal units = 1 mil
(0.001 inch).

| Quantity | Internal units | Mils | mm |
|----------|---------------|------|----|
| 1 internal unit | 1 | 0.0001 | 0.0000254 |
| 1 mil | 10,000 | 1 | 0.0254 |
| 1 inch | 10,000,000 | 1,000 | 25.4 |
| 1 mm | ~393,701 | ~39.37 | 1 |

## DXP Fractional Encoding (parameter text records)

Parameter text records split coordinates into integer and fractional parts using paired
keys:

```
LOCATION.X=100
LOCATION.X_FRAC=5000
```

Reconstruction: `raw_value = integer_part * 100,000 + fractional_part`

Source: `Rt_Schematic.Consts.cBaseUnit = 100000`. Each "DXP unit" (the integer part)
represents 10 mils (100,000 internal units).

So `LOCATION.X=100, LOCATION.X_FRAC=5000` gives:
`100 * 100,000 + 5,000 = 10,005,000` internal units = 1000.5 mils.

The `_FRAC` key is omitted from the parameter string when its value is zero.
The fractional part range is 0..99,999.

### Coordinate pairs in record types

| Record | Integer key | Fractional key | Description |
|--------|-------------|---------------|-------------|
| All | `LOCATION.X` | `LOCATION.X_FRAC` | X position |
| All | `LOCATION.Y` | `LOCATION.Y_FRAC` | Y position |
| Rectangle, Line, etc. | `CORNER.X` | `CORNER.X_FRAC` | Second corner X |
| Rectangle, Line, etc. | `CORNER.Y` | `CORNER.Y_FRAC` | Second corner Y |
| Arc, Ellipse | `RADIUS` | `RADIUS_FRAC` | Primary radius |
| Ellipse, EllipticalArc | `SECONDARYRADIUS` | `SECONDARYRADIUS_FRAC` | Secondary radius |
| Polyline, Polygon, Bezier | `X{N}` | `X{N}_FRAC` | Vertex N X (1-based) |
| Polyline, Polygon, Bezier | `Y{N}` | `Y{N}_FRAC` | Vertex N Y (1-based) |

## Binary pin coordinates

Binary pins (flags=0x01 blocks) use a truncated format. Coordinates are stored as `i16`
values where each unit represents 100,000 internal coordinate units (1 DXP unit):

```
// Write (from C# reference):
WriteShort(Convert.ToInt16(value_in_internal_units / 100000), fieldName);

// Read:
ReadShort(out var raw_i16, fieldName);
value_in_internal_units = (i32)raw_i16 * 100000;
```

Examples:

| Internal units | Mils | i16 stored |
|---------------|------|------------|
| 1,500,000 | 150 | 15 |
| -500,000 | -50 | -5 |
| 3,000,000 | 300 | 30 |
| 0 | 0 | 0 |

### Sub-unit precision via PinFrac

The `PinFrac` sidecar stream provides the remainder (in internal units) to reconstruct
full-precision coordinates:

```
final_location_x = (binary_i16_x * 100000) + pinfrac.location_x_frac
final_location_y = (binary_i16_y * 100000) + pinfrac.location_y_frac
final_pin_length  = (binary_i16_length * 100000) + pinfrac.length_frac
```

The `PinFrac` stream is only written when the fractional parts are non-zero (i.e., when
the coordinate doesn't fall exactly on a DXP unit boundary of 100,000 internal units).

## Color encoding

Colors use the Win32 COLORREF format: `0x00BBGGRR` stored as a little-endian `i32`.
Note the byte order: R is the least significant byte, B is the most significant byte
of the lower 3 bytes.

| Decimal | Hex | R | G | B | Color name |
|---------|-----|---|---|---|------------|
| 0 | 0x00000000 | 0 | 0 | 0 | Black |
| 128 | 0x00000080 | 128 | 0 | 0 | Dark red |
| 255 | 0x000000FF | 255 | 0 | 0 | Red |
| 65280 | 0x0000FF00 | 0 | 255 | 0 | Green |
| 16711680 | 0x00FF0000 | 0 | 0 | 255 | Blue |
| 16777215 | 0x00FFFFFF | 255 | 255 | 255 | White |
| 16317695 | 0x00F8F0FF | 255 | 240 | 248 | Default sheet background |

## Angle encoding

Angles (used in `SchArc`, `SchPie`, `SchEllipticalArc`) are stored as floating-point
degrees in the parameter text. The `STARTANGLE` and `ENDANGLE` keys hold `f64` values.

A full circle is `STARTANGLE=0.0`, `ENDANGLE=360.0`.

## Indexed vertex coordinates

Polylines, polygons, and bezier curves use 1-based indexed coordinate pairs:

```
LOCATIONCOUNT=3
X1=100|X1_FRAC=0|Y1=200|Y1_FRAC=0
X2=300|X2_FRAC=0|Y2=400|Y2_FRAC=0
X3=500|X3_FRAC=0|Y3=600|Y3_FRAC=0
```

The `LOCATION.X` / `LOCATION.Y` fields inherited from `SchGraphicalBase` are NOT used
for polyline vertices; the indexed `X{N}` / `Y{N}` keys are the vertex positions.
`LOCATIONCOUNT` gives the exact number of vertices; iterate N from 1 to `LOCATIONCOUNT`.
