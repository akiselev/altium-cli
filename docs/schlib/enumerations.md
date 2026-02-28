> **Authoritative reference**: See [../../dxp/sch-files.md](../../dxp/sch-files.md)
> for the canonical format specification. This document covers SchLib-specific details.
> Shared enumerations are also documented in [../schdoc/enumerations.md](../schdoc/enumerations.md).

# Enumerations

All enumerations used by SchLib record types.

## PinElectricalType

Used by the `ELECTRICAL` field in binary pin records (1 byte, values 0-7).

| Value | Name | Description |
|-------|------|-------------|
| 0 | Input | Signal input |
| 1 | InputOutput | Bidirectional |
| 2 | Output | Signal output |
| 3 | OpenCollector | Open-collector output |
| 4 | Passive | Passive (default) - resistors, capacitors |
| 5 | HiZ | High-impedance |
| 6 | OpenEmitter | Open-emitter output |
| 7 | Power | Power supply pin |

## PinSymbol

Used by `SYMBOL_INNEREDGE`, `SYMBOL_OUTEREDGE`, `SYMBOL_INSIDE`, `SYMBOL_OUTSIDE` fields
in binary pin records. Each is a 1-byte value.

| Value | Name | Visual meaning |
|-------|------|---------------|
| 0 | None | No symbol (default) |
| 1 | Dot | Inversion dot |
| 2 | RightLeftSignalFlow | Signal flow right to left |
| 3 | Clock | Clock edge indicator |
| 4 | ActiveLowInput | Active-low input bar |
| 5 | AnalogSignalIn | Analog signal input |
| 6 | NotLogicConnection | Not a logic connection |
| 8 | PostponedOutput | Postponed output |
| 9 | OpenCollector | Open-collector symbol |
| 10 | HiZ | High-impedance symbol |
| 11 | HighCurrent | High current |
| 12 | Pulse | Pulse |
| 13 | Schmitt | Schmitt trigger symbol |
| 17 | OpenCollectorPullUp | Open-collector with pull-up |
| 22 | OpenEmitter | Open-emitter |
| 23 | OpenEmitterPullUp | Open-emitter with pull-up |
| 25 | ShiftLeft | Shift left |
| 30 | OpenOutput | Open output |
| 33 | LeftRightSignalFlow | Signal flow left to right |
| 34 | BiDirectionalSignalFlow | Bidirectional signal flow |

## PinConglomerateFlags

The `PINCONGLOMERATE` byte in binary pin records is a bitmask.

| Bit | Name | Description |
|-----|------|-------------|
| 0 | HIDE | Pin is hidden |
| 1 | DISPLAY_NAME_VISIBLE | Show pin name text |
| 2 | DESIGNATOR_VISIBLE | Show pin number/designator text |
| 3-4 | ROTATION | Pin orientation (0=0deg, 1=90deg, 2=180deg, 3=270deg) |
| 5 | FLIPPED | Pin orientation is flipped |

The ROTATION field occupies bits 3 and 4 (i.e., `(byte >> 3) & 0x3` gives the rotation
index).

## LineWidth

Used by `LINEWIDTH` parameter in line-based records.

| Value | Name |
|-------|------|
| 0 | Smallest |
| 1 | Small (default) |
| 2 | Medium |
| 3 | Large |

## LineStyle

Used by `LINESTYLE` parameter in `SchLine` and `SchPolyline`.

| Value | Name |
|-------|------|
| 0 | Solid (default) |
| 1 | Dashed |
| 2 | Dotted |
| 3 | DashDotted |

## TextOrientation (ORIENTATION bitmask)

Used by `ORIENTATION` parameter in `SchLabel` and `SchComponent`.

| Bit | Name | Description |
|-----|------|-------------|
| 0 | ROTATED | Rotate text 90 degrees |
| 1 | FLIPPED | Flip text |

## TextJustification

Used by `JUSTIFICATION` parameter in `SchLabel`.

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

## ComponentKind

Used by `COMPONENTKIND` parameter in `SchComponent`.

| Value | Name |
|-------|------|
| 0 | Standard |
| 1 | MechanicalLink |
| 2 | Graphical |
| 3 | GraphicalLink |
| 4 | NetlistObject |
| 5 | JumperWire |
| 6 | Standard_NoERC |

## PowerObjectStyle

Used by `STYLE` parameter in RECORD=17 (power port objects, not commonly found in
SchLib but defined for completeness).

| Value | Name |
|-------|------|
| 0 | Circle |
| 1 | Arrow |
| 2 | Bar |
| 3 | Wave |
| 4 | PowerGround |
| 5 | SignalGround |
| 6 | Earth |
| 7 | GndPower |

## TRotationBy90

Used in `PinTextData` sidecar stream to encode text rotation. Stored in bits 2-3 of the
flags byte.

| Value | Name |
|-------|------|
| 0 | eRotate0 |
| 1 | eRotate90 |
| 2 | eRotate180 |
| 3 | eRotate270 |

## TPinTextRotationAnchor

Used in `PinTextData` sidecar stream to encode the rotation anchor. Stored in bit 1 of
the flags byte (only present when `PositionMode == Custom`).

| Value | Name | Description |
|-------|------|-------------|
| 0 | raPin | Anchor rotation at the pin endpoint |
| 1 | raComponent | Anchor rotation at the component body |
