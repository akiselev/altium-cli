# Binary Pin Format

Pins in the `Data` stream are written as binary blocks (flags=0x01). The first byte of
the payload is always `0x02`, which is the binary code for `SchDataPin`.

## On-disk layout

The record is variable-length because `description`, `name`, and `designator` are
length-prefixed strings embedded directly in the binary data. The 33-byte size observed
in some test files reflects the minimal case: empty description, short name, short
designator.

```
Offset          Size  Type      Field                    Parameter equivalent
--------        ----  -----     -----                    --------------------
0x00            1     u8        binary_code              Always 0x02
0x01            4     i32 LE    owner_index              OWNERINDEX
0x05            2     i16 LE    owner_part_id            OWNERPARTID
0x07            1     u8        owner_part_display_mode  OWNERPARTDISPLAYMODE
0x08            1     u8        symbol_inner_edge        SYMBOL_INNEREDGE
0x09            1     u8        symbol_outer_edge        SYMBOL_OUTEREDGE
0x0A            1     u8        symbol_inside            SYMBOL_INSIDE
0x0B            1     u8        symbol_outside           SYMBOL_OUTSIDE
0x0C            1     u8        description_length       Length of DESCRIPTION (0 if empty)
0x0D            N     bytes     description              DESCRIPTION (ASCII, 0-254 bytes)
0x0D+N          1     u8        formal_type              FORMALTYPE
0x0E+N          1     u8        electrical               ELECTRICAL (0-7, see enumerations.md)
0x0F+N          1     u8        pin_conglomerate         PINCONGLOMERATE (bitmask, see enumerations.md)
0x10+N          2     i16 LE    pin_length               PINLENGTH
0x12+N          2     i16 LE    location_x               LOCATION.X
0x14+N          2     i16 LE    location_y               LOCATION.Y
0x16+N          4     i32 LE    color                    COLOR (COLORREF)
0x1A+N          1     u8        name_length              Length of NAME
0x1B+N          M     bytes     name                     NAME (ASCII)
0x1B+N+M        1     u8        designator_length        Length of DESIGNATOR
0x1C+N+M        P     bytes     designator               DESIGNATOR (ASCII)
0x1C+N+M+P      1     u8        swap_id_pin_length       Length of SWAPIDPIN
+Q              1     u8        swap_id_part_length      Length of SWAPIDPART (binary)
+R              1     u8        default_value_length     Length of DEFAULTVALUE
+S              —     —         (end)
```

Where N = `description_length`, M = `name_length`, P = `designator_length`,
Q = `swap_id_pin_length`, R = `swap_id_part_length`, S = `default_value_length`.

Total record size = 25 + N + M + P + Q + R + S bytes.

## Binary code 0x02

In binary-mode blocks (flags=0x01), the first byte is a "binary code" that identifies the
record type:
- `0x02` = Pin (`SchDataPin`) - the only binary code appearing in `Data` streams
- `0xD0` (208) = Embedded binary object - used in `/Storage` and all pin sidecar streams

## Coordinate encoding

Binary pins store coordinates as `i16` values in DXP units, where 1 DXP unit = 100,000
internal coordinate units.

```
// Write (from C# reference implementation):
WriteShort(Convert.ToInt16(argN / 100000), argName);

// Read:
ReadShort(out var value, argName);
argN = value * 100000;
```

Examples:
- A pin at 150 mils (1,500,000 internal units) → i16 value 15
- A pin at -50 mils (-500,000 internal units) → i16 value -5
- Pin length of 300 mils (3,000,000 internal units) → i16 value 30

The `PinFrac` sidecar stream provides the sub-unit remainder to reconstruct full
precision:

```
pin.location.x = (binary_location_x * 100000) + pinfrac.location_x_frac
pin.location.y = (binary_location_y * 100000) + pinfrac.location_y_frac
pin.pin_length  = (binary_pin_length  * 100000) + pinfrac.length_frac
```

See [pin-sidecar-streams.md](pin-sidecar-streams.md) for the `PinFrac` format and
[coordinate-system.md](coordinate-system.md) for the full coordinate system description.

## PinConglomerate bitmask

The `pin_conglomerate` byte encodes orientation and visibility flags. See
[enumerations.md](enumerations.md) for the `PinConglomerateFlags` bit definitions.

## Description length limit

The binary record holds up to 254 bytes of description (`description_length` is a `u8`
with value range 0-254). Descriptions exceeding 254 bytes overflow into the `PinDesc`
sidecar stream, which appends the remainder. See [pin-sidecar-streams.md](pin-sidecar-streams.md).

## PinWideText authority

When the `PinWideText` sidecar stream is present, it fully replaces the `name`,
`designator`, and description fields parsed from the binary record. `PinWideText` is the
authoritative source for text data when present. See [pin-sidecar-streams.md](pin-sidecar-streams.md).
