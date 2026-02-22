# Component Data Stream

Each component's `Data` stream contains sequential blocks describing the component and all
its child primitives.

## Block structure

Each block starts with a 4-byte little-endian header:

```
bits [23:0]  = payload size in bytes
bits [31:24] = flags byte
```

Flags:
- `0x00` - parameter text block: NUL-terminated pipe-delimited `key=value` pairs in
  Windows-1252. The `RECORD` key gives the record type number.
- `0x01` - binary data block: raw binary. In `Data` streams, these are always pin records.
  The first byte of the payload is always `0x02` (the binary code for SchDataPin).

## Block sequence in a component's Data stream

```
Block 0:   SchComponent (flags=0x00, RECORD=1)       <- always first, always parameter block
Block 1..N: pins and other primitives
             flags=0x01 for pins (binary pin record)
             flags=0x00 for all other primitives (parameter block, RECORD != 1)
Block N+1: end marker (flags=0x00, RECORD=0 or RECORD=44 depending on version)
```

Reading terminates when RECORD=0 is encountered (or when the stream ends).

## Record types in SchLib Data streams

Observed across 3 real SchLib files (LimeMicro 200 components, Synthiam 174 components,
BlankSchLib 1 component):

| RECORD | Rust type name | Description | Frequency |
|--------|----------------|-------------|-----------|
| 1 | `SchComponent` | Component container; always first block | Always (1 per component) |
| 2 | `SchPin` | Pin; always a binary block (flags=0x01) | Very common |
| 4 | `SchLabel` | Text annotation | Common |
| 5 | `SchBezier` | Bezier curve (4 control points) | Rare |
| 6 | `SchPolyline` | Multi-segment line | Common |
| 7 | `SchPolygon` | Closed filled polygon | Common |
| 8 | `SchEllipse` | Ellipse | Rare |
| 9 | `SchPie` | Pie/wedge shape | Rare |
| 11 | `SchEllipticalArc` | Elliptical arc | Rare |
| 12 | `SchArc` | Circular arc | Common |
| 13 | `SchLine` | Single line segment | Common |
| 14 | `SchRectangle` | Rectangle | Very common |
| 28 | `SchTextFrame` | Rich text box | Rare |
| 30 | `SchImage` | Embedded image | Rare |
| 34 | `SchDesignator` | Reference designator text | Always (1 per component) |
| 41 | `SchParameter` | Named parameter (Comment, Value, etc.) | Always (1+ per component) |
| 44 | `SchImplementationList` | Container for footprint assignments | Common |
| 45 | `SchImplementation` | Single footprint assignment | Common |
| 46 | `SchImplementationMap` | Container for pin mappings | Common |
| 47 | `SchMapDefiner` | Pin-to-pad mapping entry | Occasional |
| 48 | `SchImplementationParameters` | Footprint parameters | Common |

For complete field definitions of each record type, see [record-types.md](record-types.md).

## OwnerIndex in SchLib

`OWNERINDEX` values in a component's `Data` stream are **relative within that component's
section**, not absolute positions in any global list. The `SchComponent` record is at
relative index 0. All child records reference their parent by this local relative index.

Example ordering for a component with 2 pins, a rectangle, a designator, and one
footprint with mappings:

```
Relative Index 0:  SchComponent (RECORD=1)               OwnerIndex=-1 (root, no parent)
Relative Index 1:  SchPin (binary, flags=0x01)           OwnerIndex=0
Relative Index 2:  SchPin (binary, flags=0x01)           OwnerIndex=0
Relative Index 3:  SchRectangle (RECORD=14)              OwnerIndex=0
Relative Index 4:  SchDesignator (RECORD=34)             OwnerIndex=0
Relative Index 5:  SchParameter (RECORD=41)              OwnerIndex=0
Relative Index 6:  SchImplementationList (RECORD=44)     OwnerIndex=0
Relative Index 7:  SchImplementation (RECORD=45)         OwnerIndex=6
Relative Index 8:  SchImplementationMap (RECORD=46)      OwnerIndex=7
Relative Index 9:  SchMapDefiner (RECORD=47)             OwnerIndex=7
Relative Index 10: SchImplementationParameters (RECORD=48) OwnerIndex=7
```

During loading, these relative indices must be adjusted to absolute positions in the global
warehouse by adding the component's base offset:

```
absolute_owner_index = relative_owner_index + component_base_offset
```

This adjustment applies to every child record. The `SchComponent` itself (relative index
0) gets `OwnerIndex=-1` which means it has no parent.
