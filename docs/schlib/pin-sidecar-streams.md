# Pin Sidecar Streams

SchLib files can have up to 9 optional per-component sidecar streams that carry additional
pin data not stored in the main binary pin records. These streams only exist in SchLib,
not in SchDoc.

All sidecar streams live inside the same CFB sub-storage as the component's `Data` stream:
`/<ComponentKey>/PinFrac`, `/<ComponentKey>/PinDesc`, etc.

## Embedded object envelope

Every sidecar stream uses the same outer structure:

```
[header block, flags=0x00]
    NUL-terminated params: |RECORD=0|HEADER=<stream_name>|Weight=<count>|
    Where <count> is the number of entry blocks that follow.

[entry blocks, flags=0x01]
    One block per pin that has data in this stream.
```

Each entry block payload has this layout:

```
[1 byte]  0xD0 tag (embedded object marker)
[1 byte]  id_length
[N bytes] id (pin index as a decimal ASCII string, e.g. "0", "1", "15")
[4 bytes] inner header: bits[23:0]=inner_data_length, bits[31:24]=inner_flags (always 0x00)
[M bytes] inner data (stream-specific, described below for each stream)
```

Pin indices in the `id` field are 0-based and reference the pin's position in the ordered
list of pins within this component (i.e., the order the pins appear in the `Data` stream).

## Import order

Sidecar streams MUST be applied in this exact order. The order matters because `PinDesc`
appends while `PinWideText` replaces, and `PinWideText` must win.

| Step | Stream | Effect |
|------|--------|--------|
| 1 | `PinFrac` | Adjusts pin coordinates (additive) |
| 2 | `PinDesc` | Appends to description (additive) |
| 3 | `PinMiscData` | Sets `PairSwapID` |
| 4 | `PinTextData` | Sets custom text display settings |
| 5 | `PinWideText` | **Replaces** text fields (authoritative) |
| 6 | `PinSymbolLineWidth` | Sets symbol line width |
| 7 | `PinPackageLength` | Sets package length |
| 8 | `PinPropagationDelay` | Sets propagation delay |
| 9 | `PinFunctionData` | Sets pin functions |

Streams that do not exist for a given component are silently skipped.

**Critical:** `PinWideText` replaces; `PinDesc` appends. `PinWideText` must be processed
after `PinDesc` so that `PinWideText` is the final authoritative value.

## Stream formats

### PinFrac (12 bytes)

Provides sub-unit coordinate precision. Applied additively to coordinates decoded from
the binary pin record.

```
[4 bytes] location_x_frac  (i32 LE)
[4 bytes] location_y_frac  (i32 LE)
[4 bytes] length_frac      (i32 LE)
```

Applied as:
```
pin.location.x = (binary_x * 100000) + location_x_frac
pin.location.y = (binary_y * 100000) + location_y_frac
pin.pin_length  = (binary_length * 100000) + length_frac
```

Written only when pin coordinates don't align to the DXP2004SP1 unit grid (i.e., when
the fractional parts are non-zero).

### PinDesc (length-prefixed ASCII)

Holds description text that overflows the 254-byte limit in the binary pin record.

```
[4 bytes] text_length (u32 LE)
[N bytes] ASCII text (the overflow portion only, not the full description)
```

Applied as:
```
pin.description = pin.description + value
```

Written only when `description.length > 254`.

### PinMiscData (length-prefixed UTF-16LE params)

```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE param string
```

Param string format: `PairSwapID=<value>`

Sets the pin's `PairSwapID` field.

### PinTextData (variable-length binary, 2-22 bytes)

Holds custom text positioning and font settings for the pin's name and designator text.

The inner data consists of two consecutive structs in order: (1) name text data, then
(2) designator text data. Each struct has this layout:

```
Byte 0: Flags byte
    Bit 0: PositionMode
           0 = Default positioning
           1 = Custom positioning (additional fields follow)
    Bit 1: RotationAnchor (only present when bit 0 = 1)
           0 = raPin (anchor at pin)
           1 = raComponent (anchor at component)
    Bits 2-3: RotationRelative (only present when bit 0 = 1)
           TRotationBy90: 0=0deg, 1=90deg, 2=180deg, 3=270deg
    Bit 4: FontMode
           0 = Default font
           1 = Custom font (additional fields follow)

If PositionMode == Custom (bit 0 set):
    [4 bytes] customMargin (i32 LE)

If FontMode == Custom (bit 4 set):
    [2 bytes] customFontID (i16 LE) - file-local font ID
    [4 bytes] customColor  (u32 LE) - COLORREF
```

Each struct is 1 byte minimum (all defaults) or up to 11 bytes (both custom).
Total inner data: 2 bytes minimum, 22 bytes maximum.

### PinWideText (length-prefixed UTF-16LE params)

The authoritative source for pin text fields when present. Fully replaces the
corresponding fields parsed from the binary pin record and `PinDesc`.

```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE param string
```

Recognized keys (all optional):

| Key | Replaces |
|-----|----------|
| `Desc` | pin description |
| `Name` | pin name |
| `Desig` | pin designator |
| `SwapId` | swap group ID |
| `SwapIDPart` | swap part ID |
| `DefValue` | default value |

Each present key fully replaces the corresponding pin field.

### PinSymbolLineWidth (length-prefixed UTF-16LE params)

```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE param string
```

Param string: `SymBol_LineWidth=<value>`

Note the exact key casing: `SymBol_LineWidth` (capital B, capital L, underscore).

### PinPackageLength (length-prefixed UTF-16LE params)

```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE param string
```

Param string: `PinPackageLength=<value>`

Value is in internal coordinate units.

### PinPropagationDelay (length-prefixed UTF-16LE params)

```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE param string
```

Param string: `PinPropagationDelay=<value>`

Value is in scientific notation (e.g. `1.5E-9`).

### PinFunctionData (length-prefixed UTF-16LE params)

```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE param string
```

Keys use 1-based indices:

| Key | Description |
|-----|-------------|
| `PinSelectedFunctionsCount` | Number of selected functions |
| `PinSelectedFunction1`, `PinSelectedFunction2`, ... | Selected function values |
| `PinDefinedFunctionsCount` | Number of defined functions |
| `PinDefinedFunction1`, `PinDefinedFunction2`, ... | Defined function values |

## Stream presence patterns from real files

### LimeMicro (200 components)

| Streams present | Component count |
|----------------|-----------------|
| `Data` + `PinFrac` + `PinPackageLength` + `PinSymbolLineWidth` | 151 |
| `Data` + `PinPackageLength` + `PinSymbolLineWidth` | 32 |
| `Data` + `PinFrac` + `PinPackageLength` + `PinSymbolLineWidth` + `PinTextData` | 16 |
| `Data` only | 1 |

### Synthiam (174 components)

All 174 components have `Data` only; no sidecar streams present.

### BlankSchLib (1 component)

`Data` only; no sidecar streams present.
