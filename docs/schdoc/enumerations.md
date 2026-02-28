> **Authoritative reference**: See [../../dxp/sch-files.md](../../dxp/sch-files.md)
> for the canonical format specification. This document covers SchDoc-specific details.
> Shared enumerations are also documented in [../schlib/enumerations.md](../schlib/enumerations.md).

# Enumerations

All enumerations used by SchDoc record types. Most are shared with SchLib -- those are
marked as such. SchDoc-specific enumerations are at the end.

## Shared with SchLib

### PinElectricalType

Used by the `ELECTRICAL` field in pin records (RECORD=2).

| Value | Name | Description |
|-------|------|-------------|
| 0 | Input | Signal input |
| 1 | InputOutput | Bidirectional |
| 2 | Output | Signal output |
| 3 | OpenCollector | Open-collector output |
| 4 | Passive | Passive (default) -- resistors, capacitors |
| 5 | HiZ | High-impedance |
| 6 | OpenEmitter | Open-emitter output |
| 7 | Power | Power supply pin |

### PinSymbol

Used by `SYMBOL_INNEREDGE`, `SYMBOL_OUTEREDGE`, `SYMBOL_INSIDE`, `SYMBOL_OUTSIDE` fields
in pin records. Each is a 1-byte value.

| Value | Name |
|-------|------|
| 0 | None (default) |
| 1 | Dot (inversion) |
| 2 | RightLeftSignalFlow |
| 3 | Clock |
| 4 | ActiveLowInput |
| 5 | AnalogSignalIn |
| 6 | NotLogicConnection |
| 8 | PostponedOutput |
| 9 | OpenCollector |
| 10 | HiZ |
| 11 | HighCurrent |
| 12 | Pulse |
| 13 | Schmitt |
| 17 | OpenCollectorPullUp |
| 22 | OpenEmitter |
| 23 | OpenEmitterPullUp |
| 25 | ShiftLeft |
| 30 | OpenOutput |
| 33 | LeftRightSignalFlow |
| 34 | BiDirectionalSignalFlow |

### PinConglomerateFlags

The `PINCONGLOMERATE` field is a bitmask encoding orientation and visibility.

| Bits | Mask | Name | Description |
|------|------|------|-------------|
| 0-1 | 0x03 | ORIENTATION | Pin rotation: TRotationBy90 (0=0°, 1=90°, 2=180°, 3=270°) |
| 2 | 0x04 | IS_HIDDEN | Pin is hidden |
| 3 | 0x08 | SHOW_NAME | Show pin name text |
| 4 | 0x10 | SHOW_DESIGNATOR | Show pin designator text |
| 5 | 0x20 | NOT_ACCESSIBLE | Inverse accessibility (set = NOT accessible) |
| 6 | 0x40 | GRAPHICALLY_LOCKED | Written on export but reset to 0 on import |
| 7 | 0x80 | OWNER_INDEX_ADDITIONAL_LIST | OwnerIndex refers to Additional stream |

ORIENTATION values: `orientation = byte & 0x03`. 0=0° (right), 1=90° (up), 2=180° (left), 3=270° (down).

### LineWidth

Used by `LINEWIDTH` parameter in line-based records.

| Value | Name |
|-------|------|
| 0 | Smallest |
| 1 | Small (default) |
| 2 | Medium |
| 3 | Large |

### LineStyle

Used by `LINESTYLE` parameter in SchLine, SchPolyline, and SchDashedRectangle (RECORD=225).

| Value | Name |
|-------|------|
| 0 | Solid (default) |
| 1 | Dashed |
| 2 | Dotted |
| 3 | DashDotted |

### TextJustification

Used by `JUSTIFICATION` parameter in SchLabel.

| Value | Name |
|-------|------|
| 0 | BottomLeft |
| 1 | BottomCenter |
| 2 | BottomRight |
| 3 | MiddleLeft |
| 4 | MiddleCenter |
| 5 | MiddleRight |
| 6 | TopLeft |
| 7 | TopCenter |
| 8 | TopRight |

### TextOrientation (ORIENTATION bitmask)

Used by `ORIENTATION` parameter in SchLabel.

| Bit | Name | Description |
|-----|------|-------------|
| 0 | ROTATED | Rotate text 90 degrees |
| 1 | FLIPPED | Flip text |

### ComponentKind

Used by `COMPONENTKIND` parameter in SchComponent.

| Value | Name |
|-------|------|
| 0 | Standard |
| 1 | MechanicalLink |
| 2 | Graphical |
| 3 | GraphicalLink |
| 4 | NetlistObject |
| 5 | JumperWire |
| 6 | Standard_NoERC |

## SchDoc-specific enumerations

### PowerObjectStyle

Used by `STYLE` parameter in RECORD=17 (SchPowerObject).

| Value | Name | Visual / C# string |
|-------|------|--------|
| 0 | Circle | "Circle" |
| 1 | Arrow | "Arrow" |
| 2 | Bar | "Bar" (VCC style) |
| 3 | Wave | "Wave" |
| 4 | GndPower | "Power Ground" |
| 5 | GndSignal | "Signal Ground" |
| 6 | GndEarth | "Earth" |
| 7 | GostArrow | "GOST Arrow" |
| 8 | GostGndPower | "GOST Power Ground" |
| 9 | GostGndEarth | "GOST Earth" |
| 10 | GostBar | "GOST Bar" |

Observed in real files: 2 (Bar/VCC) and 4 (GndPower/GND).

### PortIoType

Used by `IOTYPE` parameter in RECORD=18 (SchPort) and RECORD=16 (SchSheetEntry).

| Value | Name |
|-------|------|
| 0 | Unspecified |
| 1 | Output |
| 2 | Input |
| 3 | Bidirectional |

### PortArrowStyle

Used by `STYLE` parameter in RECORD=18 (SchPort).

| Value | Name | C# string |
|-------|------|-----------|
| 0 | None | "None (Horizontal)" |
| 1 | Left | "Left" |
| 2 | Right | "Right" |
| 3 | LeftRight | "Left & Right" |
| 4 | NoneVertical | "None (Vertical)" |
| 5 | Top | "Top" |
| 6 | Bottom | "Bottom" |
| 7 | TopBottom | "Top & Bottom" |

### SheetStyle

Used by `SHEETSTYLE` parameter in RECORD=31 (SchSheet).

| Value | Name | Size |
|-------|------|------|
| 0 | A4 | 297 x 210 mm |
| 1 | A3 | 420 x 297 mm |
| 2 | A2 | 594 x 420 mm |
| 3 | A1 | 841 x 594 mm |
| 4 | A0 | 1189 x 841 mm |
| 5 | A | 11 x 8.5 in |
| 6 | B | 17 x 11 in |
| 7 | C | 22 x 17 in |
| 8 | D | 34 x 22 in |
| 9 | E | 44 x 34 in |
| 10 | Letter | 11 x 8.5 in |
| 11 | Legal | 14 x 8.5 in |
| 12 | Tabloid | 17 x 11 in |
| 13 | OrCAD A | |
| 14 | OrCAD B | |
| 15 | OrCAD C | |
| 16 | OrCAD D | |
| 17 | OrCAD E | |

When `SheetStyle` is absent, `CustomX` and `CustomY` define the sheet dimensions.

### ComponentOrientation

Used by `ORIENTATION` parameter in RECORD=1 (SchComponent).

| Value | Degrees |
|-------|---------|
| 0 | 0 (no rotation) |
| 1 | 90 |
| 2 | 180 |
| 3 | 270 |

### NoConnectSymbol

Used by `SYMBOL` parameter in RECORD=22 (SchNoConnect).

| Internal value | String in file | Description |
|----------------|----------------|-------------|
| 0 | `Thin Cross` | Thin X mark (default) |
| 1 | `Thick Cross` | Thick X mark |
| 2 | `Small Cross` | Small X mark |
| 3 | `Checkbox` | Checkbox mark |
| 4 | `Triangle` | Triangle mark |

Note: The `SYMBOL` parameter stores a string value in the file (e.g., `SYMBOL=Thin Cross`). The internal enum is serialized/deserialized via a string lookup dictionary.
