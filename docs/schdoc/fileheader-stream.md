# FileHeader Stream

The `FileHeader` stream contains **all** schematic records in a single sequential stream.
This is the primary data stream for a SchDoc file.

## Stream layout

```
Block 0:    Document header (no RECORD key)
Block 1:    Sheet properties (RECORD=31) with font table
Block 2:    Template reference (RECORD=39)
Block 3..T: Template-owned primitives (labels, lines from the template)
Block T+1:  First schematic content record
  ...
Block N:    Last schematic content record
```

All blocks use flags=0x00 (parameter text). There are no binary blocks in the FileHeader
stream -- even pins are parameter text format (RECORD=2).

## Block 0: Document header

The first block is a lightweight document header with no `RECORD` key.

| Key | Type | Description |
|-----|------|-------------|
| `HEADER` | string | Must equal `Protel for Windows - Schematic Capture Binary File Version 5.0` |
| `Weight` | i32 | Total number of object records that follow (blocks 1..N) |
| `MinorVersion` | i32 | Format minor version (observed: `2` in LimeSDR test files; current Altium 26 writes `13` per sch-files.md Appendix C) |
| `UniqueID` | string | 8-character document-level unique identifier |

The `Weight` value equals the total block count minus 1 (the header block itself). Use
this to pre-allocate the record list.

## Block 1: Sheet properties (RECORD=31)

Always the first content record. Contains sheet dimensions, grid settings, and the font
table.

**Note:** `docs/dxp/sch-files.md` section 9 states that the header parameters and sheet
parameters may be serialized as "one combined parameter block" by the .NET serializer.
In the LimeSDR test files (Altium Designer 16 era), they appear as two separate blocks
(Block 0 = header, Block 1 = RECORD=31 with font table). Current Altium 26 may combine
them. When implementing, check whether `FontIdCount` appears in Block 0 or Block 1 and
handle both cases.

### Core sheet fields

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `RECORD` | i32 | 31 | Always 31 |
| `SheetStyle` | i32 | | Sheet size preset (0=A4, 1=A3, 2=A2, etc.; absent for custom) |
| `SystemFont` | i32 | 1 | System font ID |
| `BorderOn` | bool | T | Show sheet border |
| `SheetNumberSpaceSize` | i32 | 12 | Sheet number space size |
| `AreaColor` | i32 | 16317695 | Background color (COLORREF) |
| `SnapGridOn` | bool | T | Snap grid enabled |
| `SnapGridSize` + `SnapGridSize_Frac` | i32 | | Snap grid size (DXP fractional) |
| `VisibleGridOn` | bool | T | Visible grid enabled |
| `VisibleGridSize` + `VisibleGridSize_Frac` | i32 | | Visible grid size (DXP fractional) |
| `HotSpotGridOn` | bool | T | Hotspot grid enabled |
| `HotSpotGridSize` + `HotSpotGridSize_Frac` | i32 | | Hotspot grid size (DXP fractional) |
| `CustomX` | i32 | 1000 | Custom sheet width (in DXP units) |
| `CustomY` | i32 | 800 | Custom sheet height (in DXP units) |
| `ShowTemplateGraphics` | bool | T | Show template graphics |
| `TemplateFileName` | string | | Path to the template file (.SchDot) |
| `Display_Unit` | i32 | 1 | Display unit (0=mils, 1=mm) |
| `UseMBCS` | bool | T | Use multi-byte character set |
| `IsBOC` | bool | T | (always T) |

### Font table

Fonts use 1-based indexing. `FontIdCount` gives the total number of fonts.

| Key | Type | Description |
|-----|------|-------------|
| `FontIdCount` | i32 | Number of fonts (observed: 5-14) |
| `Size{N}` | i32 | Font size in points |
| `FontName{N}` | string | Font face name (e.g., "Times New Roman", "Arial") |
| `Rotation{N}` | i32 | Font rotation in degrees (optional, 0 if absent) |
| `Bold{N}` | bool | Bold flag (optional, F if absent) |
| `Italic{N}` | bool | Italic flag (optional, F if absent) |
| `Underline{N}` | bool | Underline flag (optional, F if absent) |
| `StrikeOut{N}` | bool | Strikethrough flag (optional, F if absent) |

Where `{N}` is the 1-based font index (1 through FontIdCount).

Example from a real file (14 fonts):
```
FontIdCount=14
Size1=10|FontName1=Times New Roman
Size2=10|Rotation2=90|FontName2=Times New Roman
Size3=10|Underline3=T|FontName3=Times New Roman
Size4=12|FontName4=Times New Roman
Size5=10|Italic5=T|FontName5=Times New Roman
Size6=10|FontName6=Arial
Size7=15|Italic7=T|Bold7=T|FontName7=Times New Roman
Size8=14|Bold8=T|FontName8=Times New Roman
Size9=22|Bold9=T|FontName9=Times New Roman
Size10=14|FontName10=Times New Roman
Size11=10|Bold11=T|FontName11=Times New Roman
Size12=14|Rotation12=90|Bold12=T|FontName12=Times New Roman
Size13=10|Rotation13=90|FontName13=Arial
Size14=18|Rotation14=90|FontName14=Times New Roman
```

## Block 2: Template reference (RECORD=39)

Always the second content record. References the sheet template file.

| Key | Type | Description |
|-----|------|-------------|
| `RECORD` | i32 | 39 |
| `IsNotAccesible` | bool | Always T |
| `OwnerPartId` | i32 | Always -1 |
| `FileName` | string | Full path to the .SchDot template file |

The template primitives (labels, lines, rectangles from the template) follow as the next
blocks, owned by this template record.

## Blocks 3+: Content records

All remaining blocks are schematic content records. The `RECORD` key identifies the type.
Records are ordered in a depth-first traversal of the ownership tree.

### Ordering rules

1. The sheet (RECORD=31) is always at relative index 0 in the warehouse
2. Template (RECORD=39) follows immediately after the sheet
3. Template-owned primitives follow the template
4. Components (RECORD=1) appear with all their children immediately after
5. Children are ordered by OWNERINDEX pointing to their parent's absolute index
6. Standalone sheet-level objects (wires, netlabels, etc.) reference the sheet via OWNERINDEX

### OwnerIndex semantics

`OWNERINDEX` is a 0-based absolute index into the flat record list (not including the
header block). Block 1 (Sheet, RECORD=31) is at index 0.

- Sheet-level objects (wires, netlabels, etc.): `OWNERINDEX` not present or implicitly 0
- Component children (pins, designators, parameters): `OWNERINDEX` = parent component index
- Implementation hierarchy: `OWNERINDEX` chains through 44 -> 45 -> 46/48

### Common fields across all records

These fields from `SchPrimitiveBase` appear on most records:

| Key | Type | Description |
|-----|------|-------------|
| `OWNERINDEX` | i32 | Parent record index (0-based into flat list) |
| `OWNERPARTID` | i32 | Multi-part symbol part (-1 for sheet-level objects) |
| `INDEXINSHEET` | i32 | Sequential index within sheet (-1 for auto) |
| `ISNOTACCESIBLE` | bool | Inverse accessibility flag (Altium typo: single 's') |
| `GRAPHICALLYLOCKED` | bool | Whether the primitive is locked |

The `IndexInSheet` field is notable: it's present in SchDoc but typically absent in SchLib.
It provides a sequential numbering for objects within the sheet.
