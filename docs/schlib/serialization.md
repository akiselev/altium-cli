# SchLib Serialization (Round-Trip Write)

How to serialize an in-memory `SchLib` back to a byte-identical CFB file.

Sources: `SchDataExporterLibraryV5.cs`, `SchDataExporterBaseV5.cs`, `FileFormatV5.cs`,
`SchDataSerializerParam.cs`, `SchDataSerializer.cs`, `SchDataObjectComparator.cs`,
`FileFormatConsts.cs`, `BinaryFileCode.cs` (all in AD26-dotnet/Altium.Sch.DataModel/).

---

## 1. Architecture: The Write Pipeline

The write pipeline is the exact inverse of the read pipeline:

```
SchLib (in-memory)
  → serialize FileHeader stream (library header + font table + component index)
  → serialize Storage stream (embedded images, zlib-compressed)
  → serialize SectionKeys stream (if any component name > 31 chars)
  → for each component:
      → serialize /{key}/Data stream (SchComponent + child records as blocks)
      → serialize /{key}/PinFrac..PinFunctionData (9 sidecar streams, sparse)
      → serialize /{key}/Additional stream (if present)
  → for each alias:
      → serialize /{alias}/Redirection stream
  → write CFB container to disk
```

Corresponding C# entry point: `SchDataExporterLibraryV5.Run()` which calls the
base class `SchDataExporterBaseV5.Run()` pipeline:

```
1. InitializeForSaving()              — fix up font IDs
2. FillBaseAndAdditionalWarehouses()  — collect records into flat list
3. FillExtendedWarehouse()            — collect embedded images
4. FixBaseWarehouse()                 — post-processing
5. WriteBaseWarehouse()               — → FileHeader
6. WriteExtendedWarehouse()           — → Storage stream
7. WriteAdditionalWarehouse()         — → Additional stream
8. PrepareAndWritePinsExtendedData()  — → 9 pin sidecar streams
9. FinalizeForSaving()
```

---

## 2. Sparse Saving — The Two-Tier Export System

**Critical finding**: Altium does NOT always write all parameters. The serializer
(`SchDataSerializerParam`) has two tiers of export methods:

### Tier 1: Default-Skipping Methods

These skip the parameter entirely when the value equals the type's zero/default:

| Method | Skips when | Effect |
|--------|-----------|--------|
| `Export_Boolean(value)` | `value == false` | Parameter omitted |
| `Export_Byte(value)` | `value == 0` | Parameter omitted |
| `Export_LongInt(value)` | `value == 0` | Parameter omitted |
| `Export_UInt(value)` | `value == 0` | Parameter omitted |
| `Export_Color(value)` | `value == 0` | Parameter omitted (via WriteUInt) |
| `Export_Float(value)` | `value == 0.0` | Parameter omitted |
| `Export_Double(value)` | `≈ 0.0` | Parameter omitted (uses RealNumEqual) |
| `Export_String(value)` | `IsNullOrEmpty` | Parameter omitted |
| `Export_DynamicString(value)` | `IsNullOrEmpty` | Parameter omitted |
| `Export_Coord(value)` | Integer part always written; `_Frac` skipped when `frac == 0` | Frac may be omitted |

### Tier 2: Always-Write Methods

These always emit the parameter regardless of value:

| Method | Always writes |
|--------|--------------|
| `Export_Boolean_WithDefault(value)` | `T` or `F` |
| `Export_Byte_WithDefault(value)` | Numeric string |
| `Export_LongInt_WithDefault(value)` | Numeric string (incl. `0`) |

### Per-Field Export Method Selection

The choice of Tier 1 vs Tier 2 is **hardcoded per field** in `FileFormatV5.cs`.
Each field in each record type uses a specific export call. The tables in Section 4
document which tier each field uses.

### Call-Site Conditional Exports

In addition to the two tiers, some fields have explicit `if` guards at the call site:

```csharp
// Only written if true (never writes "F" for this field):
if (obj.GetIgnoreOnLoad()) Export_Boolean(true, "IgnoreOnLoad");

// Only written if non-empty:
if (uniqueIdInReuseBlock.Length != 0) Export_String(value, "UniqueIDInReuseBlock");

// Only written for container objects:
if (obj is ISchDataContainer) Export_DynamicString(value, "WiringDiagramOriginUniqueId");

// Only written if non-zero:
if (fontID != 0) Export_FontID(fontID, "FontID");
```

### Implementation Strategy

For byte-perfect output, each field serialization must exactly match the C# tier:

```rust
// Tier 1 (default-skipping):
if value != T::default() { params.insert(key, value.to_param_value()); }

// Tier 2 (always-write):
params.insert(key, value.to_param_value());

// Call-site conditional:
if condition { params.insert(key, value.to_param_value()); }
```

The `#[derive(ToParams)]` macro should support attributes like:
- `#[param(key = "...", tier1)]` — skip on default
- `#[param(key = "...", tier2)]` or just `#[param(key = "...")]` — always write
- `#[param(key = "...", conditional = "expr")]` — custom guard

---

## 3. Parameter Encoding Rules

### String Format

Parameters are serialized as pipe-delimited key=value pairs, NUL-terminated:

```
|RECORD=1|KEY1=VALUE1|KEY2=VALUE2|\0
```

### Text Encoding

- Default: Windows-1252 (`encoding_rs::WINDOWS_1252.encode()`)
- If a key requires UTF-8: prefix with `%UTF8%` (e.g., `%UTF8%DESCRIPTION=...`)
- Value escaping: `|` → `[]`, `=` → `{}`

### Boolean Encoding

- `true` → `T`
- `false` → `F`
- Legacy `TRUE`/`FALSE` accepted on read but never written

### Integer/Coord/Color Encoding

- Integers: decimal string (e.g., `42`, `-1`, `0`)
- Colors: Win32 COLORREF as decimal i32 (e.g., `128` = dark red, `16711680` = blue)
- Coords (DXP fractional encoding):
  ```
  integer_part = internal_value / 100_000
  fractional_part = internal_value % 100_000  (always 0..99_999)

  Always write: KEY=<integer_part>
  Write only if frac != 0: KEY_FRAC=<fractional_part>
  ```

### Float/Double Encoding

- Stored as string: `StartAngle=45.0`, `EndAngle=360.0`
- Uses Delphi-compatible formatting (no trailing zeros for whole numbers)

### Extended Record Types

Records with type >= 256 use two parameters:
```
|RECORD=254|RECORDEX=<actual_value>|...
```

---

## 4. Parameter Order Per Record Type

Parameter order is **explicitly hardcoded** in `FileFormatV5.cs`. It is NOT
alphabetical, NOT field-declaration order, NOT RTTI-driven.

### Common Prefix: ExportDataObject

All objects start with this base export (8 parameters):

```
# Field                              Key                              Tier  Notes
1 owner_index                        OwnerIndex                       T1    Skipped if 0
2 is_not_accessible                  IsNotAccesible                   T1    Inverted; skipped if false
3 owner_index_additional_list        OwnerIndexAdditionalList          T1    Skipped if false
4 index_in_sheet                     IndexInSheet                     T1    Skipped if 0
5 ignore_on_load                     IgnoreOnLoad                     COND  Only if true
6 wiring_diagram_origin_unique_id    WiringDiagramOriginUniqueId      COND  Only for containers, if non-empty
7 is_schematic_block_object          IsSchematicBlockObject            T1    Skipped if false
8 unique_id_in_reuse_block           UniqueIDInReuseBlock             COND  Only if non-empty
```

### Common Prefix: ExportGraphicalObject (extends ExportDataObject)

Appends 5 more parameters after ExportDataObject:

```
# Field                       Key                        Tier  Notes
9  owner_part_id               OwnerPartId                T1    (i16 export)
10 owner_part_display_mode     OwnerPartDisplayMode        T1    (byte export)
11 selection_memory             SelectionMemory             T1    (byte export)
12 union_index                  UnionIndex                  T1    Skipped if 0
13 graphically_locked           GraphicallyLocked           T1    Skipped if false
```

### RECORD=1: SchComponent

```
# Field                        Key                              Tier  Notes
1  lib_reference                LibReference                     STR   Default "*"
2  component_description        ComponentDescription             STR
3  part_count                   PartCount                        T1    (i16)
4  display_mode_count           DisplayModeCount                 T1    (byte)
-- [ExportGraphicalObject]      (13 params from base hierarchy)
5  location.x                   Location.X                       COORD
6  location.y                   Location.Y                       COORD
7  display_mode                 DisplayMode                      T1
8  is_mirrored                  IsMirrored                       T1
9  orientation                  Orientation                      T1
10 current_part_id              CurrentPartId                    T1    (i16)
11 show_hidden_fields           ShowHiddenFields                 T1
12 show_hidden_pins             ShowHiddenPins                   T1
13 library_path                 LibraryPath                      STR
14 source_library_name          SourceLibraryName                STR
15 database_table_name          DatabaseTableName                STR
16 sheet_part_file_name         SheetPartFileName                STR
17 target_file_name             TargetFileName                   STR
18 unique_id                    UniqueID                         STR
19 area_color                   AreaColor                        COLOR (T1)
20 color                        Color                            COLOR (T1)
21 pin_color                    PinColor                         COLOR (T1)
22 overide_colors               OverideColors                    T1    Note: typo is canonical
23 display_field_names           DisplayFieldNames                T1
24 designator_locked             DesignatorLocked                 T1
25 part_id_locked                PartIDLocked                     T2    Export_Boolean_WithDefault
26 pins_moveable                PinsMoveable                     T1
27 alias_list                   AliasList                        STR   Comma-separated
28 not_use_library_name         NotUseLibraryName                T1    Inverted from UseLibraryName
29 not_use_db_table_name        NotUseDBTableName                T1    Inverted from UseDBTableName
30 design_item_id               DesignItemId                     STR
31 vault_guid                   VaultGUID                        STR
32 item_guid                    ItemGUID                         STR
33 revision_guid                RevisionGUID                     STR
34 symbol_vault_guid            SymbolVaultGUID                  STR
35 symbol_item_guid             SymbolItemGUID                   STR
36 symbol_revision_guid         SymbolRevisionGUID               STR
37 generic_template_guid        GenericComponentTemplateGUID      STR
38 has_only_current_part_info   HasOnlyCurrentPartInfo            T1
39 all_pin_count                AllPinCount                      T1    (i16)
40 key_component_unique_id      KeyComponentUniqueId              STR
41 component_kind               ComponentKind                    BYTE  Special encoding (see below)
42 custom_display_mode_names    CustomDisplayModeName{0..N}       STR   N = display_mode_count
```

**ComponentKind special encoding:**
```
if kind == Jumper:
    Export_Byte(0, "ComponentKind")
    Export_Byte(0, "ComponentKindVersion2")
    Export_Byte(6, "ComponentKindVersion3")
elif kind == Standard_NoBOM:
    Export_Byte(0, "ComponentKind")
    Export_Byte(value, "ComponentKindVersion2")
else:
    Export_Byte(value, "ComponentKind")
```

### RECORD=2: SchPin (Text Parameter Format)

Pins are normally binary (Section 5), but when serialized as parameters:

```
# Field                    Key                              Tier  Notes
1  owner_index              OwnerIndex                       T1
2  owner_part_id            OwnerPartId                      T1    (i16)
3  owner_part_display_mode  OwnerPartDisplayMode              T1    (byte)
4  symbol_inner_edge        SymBol_InnerEdge                  T1    (byte, note: capital B)
5  symbol_outer_edge        SymBol_OuterEdge                  T1    (byte)
6  symbol_inside            SymBol_Inner                      T1    (byte)
7  symbol_outside           SymBol_Outer                      T1    (byte)
8  description              Description                      STR
9  formal_type              FormalType                        T1    (byte)
10 electrical               Electrical                        T1    (PinElectricalType)
11 conglomerate             PinConglomerate                   T1    Bitmask (see below)
12 pin_length               PinLength                        COORD
13 location.x               Location.X                       COORD
14 location.y               Location.Y                       COORD
15 color                    Color                            COLOR
16 name                     Name                             STR
17 designator               Designator                       STR
18 swap_id_pin              SwapIdPin                        STR
19 swap_id_part             SwapIDPart                       STR
20 default_value            DefaultValue                     STR
21 swap_id_pair             SwapIdPair                       STR   (ASCII-only)
-- [conditional position data for name, then designator]
22 symbol_line_width        SymBol_LineWidth                  T1    (ASCII-only byte)
23 pin_package_length       PinPackageLength                  T1    (ASCII-only coord)
24 propagation_delay        PinPropagationDelay               T1    (ASCII-only double)
25 unique_id                UniqueID                         STR   SchLib/SchDoc only
26 hide_pin_name_as_func    HidePinNameAsFunction             T1    (ASCII-only bool)
27 selected_functions       PinSelectedFunctionsCount + PinSelectedFunction{1..N}  COND
28 defined_functions        PinDefinedFunctionsCount + PinDefinedFunction{1..N}    COND
29 pin_symbolic_name        PinSymbolicName                   STR   (ASCII-only)
30 show_symbolic_as_func    ShowPinSymbolicNameAsFunction      T1    (ASCII-only bool)
```

**PinConglomerate byte encoding:**
```
bit 0-1: orientation (RotationBy90: 0=0°, 1=90°, 2=180°, 3=270°)
bit 2:   is_hidden
bit 3:   show_name
bit 4:   show_designator
bit 5:   is_not_accessible (inverted)
bit 6:   graphically_locked
bit 7:   owner_index_additional_list
```

**Conditional position data** (for name and designator each):
```
PinName_PositionConglomerate (byte):
    bit 0: has_custom_position
    bit 1: rotation_anchor (0=pin, 1=component)
    bit 2-3: rotation_relative (RotationBy90)
    bit 4: has_custom_font

if has_custom_position:
    Name_CustomPosition_Margin (coord)
if has_custom_font:
    Name_CustomFontID (font_id)
    Name_CustomColor (color)

PinDesignator_PositionConglomerate (byte):
    [same encoding]
if has_custom_position:
    Designator_CustomPosition_Margin (coord)
if has_custom_font:
    Designator_CustomFontID (font_id)
    Designator_CustomColor (color)
```

### RECORD=12: SchArc

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  radius         Radius         COORD  (+ RADIUS_FRAC)
4  line_width     LineWidth      T1     (TSize enum)
5  start_angle    StartAngle     DBL
6  end_angle      EndAngle       DBL
7  color          Color          COLOR
8  unique_id      UniqueID       STR
```

### RECORD=13: SchLine

```
# Field          Key            Tier  Notes
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  corner.x       Corner.X       COORD
4  corner.y       Corner.Y       COORD
5  line_width     LineWidth      T1    (TSize enum)
6  line_style     LineStyle      T1    Clamped to <= 4 for backward compat
7  color          Color          COLOR
8  line_style_ext LineStyleExt   T1    (ASCII-only byte, stores full enum)
9  unique_id      UniqueID       STR
```

### RECORD=14: SchRectangle

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  corner.x       Corner.X       COORD
4  corner.y       Corner.Y       COORD
5  line_style_ext LineStyleExt   T1    (TLineStyle enum)
6  line_width     LineWidth      T1    (TSize enum)
7  color          Color          COLOR
8  area_color     AreaColor      COLOR
9  is_solid       IsSolid        T1
10 transparent    Transparent    T1
11 unique_id      UniqueID       STR
```

### RECORD=4: SchLabel

```
# Field          Key            Tier
-- [ExportGraphicalObject]
1  location.x     Location.X     COORD
2  location.y     Location.Y     COORD
3  color          Color          COLOR
4  area_color     AreaColor      COLOR
5  text           Text           STR
6  orientation    Orientation    T1
7  justification  Justification  T1
8  font_id        FontID         T1
9  is_mirrored    IsMirrored     T1
10 is_hidden      IsHidden       T1
11 unique_id      UniqueID       STR
```

### RECORD=6: SchPolyline

```
# Field          Key                    Tier
-- [ExportGraphicalObject]
1  line_width     LineWidth              T1
2  line_style     LineStyle              T1    Clamped to <= DashDotted
3  start_shape    StartLineShape          T1
4  end_shape      EndLineShape            T1
5  line_shape_size LineShapeSize          T1
6  color          Color                  COLOR
7  vertices       LOCATIONCOUNT + X{1..N} + Y{1..N}  INDEXED  (1-based, with _FRAC)
8  line_style_ext LineStyleExt           T1    (ASCII-only byte)
9  unique_id      UniqueID               STR
```

### RECORD=7: SchPolygon

```
# Field          Key                    Tier
-- [ExportGraphicalObject]
1  line_width     LineWidth              T1
2  color          Color                  COLOR
3  area_color     AreaColor              COLOR
4  is_solid       IsSolid                T1
5  transparent    Transparent            T1
6  vertices       LOCATIONCOUNT + X{1..N} + Y{1..N}  INDEXED  (1-based)
7  unique_id      UniqueID               STR
```

### RECORD=34: SchDesignator

```
# Field                      Key                            Tier  Notes
-- [ExportGraphicalObject]
1  location.x                 Location.X                     COORD
2  location.y                 Location.Y                     COORD
3  orientation                Orientation                    T1
4  justification              Justification                  T1
5  color                      Color                          COLOR
6  font_id                    FontID                         T1
7  is_hidden                  IsHidden                       T1
8  text                       Text                           STR
9  param_type                 ParamType                      T1    (ParameterKind - always "Designator")
10 name                       Name                           STR
11 show_name                  ShowName                       T1
12 read_only_state            ReadOnlyState                  T1
13 unique_id                  UniqueID                       STR
14 description                Description                    STR
15 not_allow_lib_sync         NotAllowLibrarySynchronize      T1    Inverted
16 not_allow_db_sync          NotAllowDatabaseSynchronize     T1    Inverted
17 not_auto_position          NotAutoPosition                 T1    Inverted from AutoPosition
18 is_mirrored                IsMirrored                     T1
19 text_horz_anchor           TextHorzAnchor                  T1
20 text_vert_anchor           TextVertAnchor                  T1
21 is_image_parameter         IsImageParameter                T1
22 override_not_auto_pos      OverrideNotAutoPosition          COND  Only for SchDesignator
```

### RECORD=41: SchParameter

Same order as RECORD=34 (both call `Export_Parameter` internally). The difference
is that SchDesignator may set `OverrideNotAutoPosition` conditionally.

### RECORD=44: SchImplementationList

```
-- [ExportGraphicalObject]   (only base fields, no additional)
```

### RECORD=45: SchImplementation

```
# Field                      Key                            Tier  Notes
-- [ExportDataObject]        (NOT GraphicalObject)
1  model_name                 ModelName                      STR
2  model_type                 ModelType                      STR
3  datafile_count             DataFileCount                  T1
4  model_datafile_entity{0..N} ModelDataFileEntity{0..N}      STR
5  model_datafile_kind{0..N}  ModelDataFileKind{0..N}         STR
6  is_current                 IsCurrent                      T1
7  datalinks_locked           DataLinksLocked                 T1
8  database_datalinks_locked  DatabaseDataLinksLocked          T1
9  integrated_model           IntegratedModel                 T1
10 database_model             DatabaseModel                   T1
```

### RECORD=46: SchImplementationMap

```
-- [ExportDataObject]        (only base fields)
```

### RECORD=47: SchMapDefiner

```
# Field      Key          Tier
-- [ExportDataObject]
1  pin_name   PinName      STR
2  pad_name   PadName      STR
```

### RECORD=48: SchImplementationParameters (SchParameterList)

```
-- [ExportDataObject]        (only base fields)
```

---

## 5. Binary Pin Serialization

Pins in SchLib `Data` streams are written as **binary blocks** (flags=0x01).

### On-Disk Layout

```
Offset   Size   Type     Field                    Encoding
------   ----   -----    -----                    --------
0x00     1      u8       binary_code              Always 0x02
0x01     4      i32 LE   owner_index              Absolute within component
0x05     2      i16 LE   owner_part_id
0x07     1      u8       owner_part_display_mode
0x08     1      u8       symbol_inner_edge        IeeeSymbol as u8
0x09     1      u8       symbol_outer_edge
0x0A     1      u8       symbol_inside
0x0B     1      u8       symbol_outside
0x0C     1      u8       description_length       N (0-254)
0x0D     N      bytes    description              Windows-1252 (pascal string)
0x0D+N   1      u8       formal_type
0x0E+N   1      u8       electrical               PinElectricalType as u8
0x0F+N   1      u8       pin_conglomerate         Bitmask (see Section 4)
0x10+N   2      i16 LE   pin_length               internal_value / 100_000
0x12+N   2      i16 LE   location_x               internal_value / 100_000
0x14+N   2      i16 LE   location_y               internal_value / 100_000
0x16+N   4      i32 LE   color                    Win32 COLORREF
0x1A+N   1+M    pascal   name                     u8 length + Windows-1252
+M       1+P    pascal   designator               u8 length + Windows-1252
+P       1+Q    pascal   swap_id_pin              u8 length + ASCII
+Q       1+R    pascal   swap_id_part             u8 length + ASCII
+R       1+S    pascal   default_value            u8 length + ASCII
```

Total size = 23 + N + M + P + Q + R + S bytes (variable due to pascal strings).

### Coordinate Truncation for Binary Pins

Binary pin coordinates are stored as `i16` values, each representing one DXP unit
(100,000 internal units). The sub-unit remainder goes to the `PinFrac` sidecar:

```rust
// Write:
let dxp_unit = coord.to_internal() / C_BASE_UNIT;  // integer division → i16
let frac = coord.to_internal() % C_BASE_UNIT;       // remainder → PinFrac

// Read:
let coord = Coord::from_internal(dxp_unit as i32 * C_BASE_UNIT + frac);
```

### PinConglomerate Byte Encoding

```rust
fn encode_pin_conglomerate(pin: &SchPin) -> u8 {
    (pin.orientation as u8 & 0x03)
        | if pin.is_hidden { 0x04 } else { 0 }
        | if pin.show_name { 0x08 } else { 0 }
        | if pin.show_designator { 0x10 } else { 0 }
        | if pin.is_not_accessible { 0x20 } else { 0 }
        | if pin.graphically_locked { 0x40 } else { 0 }
        | if pin.owner_index_additional_list { 0x80 } else { 0 }
}
```

---

## 6. Block Stream Writing

### Block Header Format (4 bytes, little-endian i32)

```
bits [23:0]  = payload size in bytes  (mask 0x00FF_FFFF)
bits [31:24] = flags byte             (0x00 = text, 0x01 = binary)
```

### Writing a Text Block (flags=0x00)

```rust
fn write_text_block(stream: &mut Vec<u8>, params: &OrderedParams) {
    let payload = params.to_pipe_delimited_nul_terminated();
    // payload = b"|RECORD=1|KEY=VALUE|...\0" (Windows-1252 encoded, NUL terminated)
    let header: i32 = (0x00 << 24) | (payload.len() as i32 & 0x00FF_FFFF);
    stream.extend_from_slice(&header.to_le_bytes());
    stream.extend_from_slice(&payload);
}
```

### Writing a Binary Block (flags=0x01)

```rust
fn write_binary_block(stream: &mut Vec<u8>, data: &[u8]) {
    let header: i32 = (0x01 << 24) | (data.len() as i32 & 0x00FF_FFFF);
    stream.extend_from_slice(&header.to_le_bytes());
    stream.extend_from_slice(data);
}
```

---

## 7. Component Data Stream

### Block Sequence

```
Block 0:    SchComponent (flags=0x00, RECORD=1) — always first
Block 1..N: Child records sorted by SchDataObjectComparator:
              Pins → flags=0x01 (binary)
              Others → flags=0x00 (parameters)
```

### Child Record Ordering (SchDataObjectComparator)

```csharp
int codeX = GetBinaryCodeForObject(x);
int codeY = GetBinaryCodeForObject(y);
if (codeX > 225 || codeY > 225) {
    return codeX - codeY;  // Extended records: sort by RECORD ascending
}
return x.OwnerIndexForSave - y.OwnerIndexForSave;  // Standard: preserve insertion order
```

Rules:
- RECORD <= 225 (standard records): **preserve insertion order** (stable sort by index)
- RECORD > 225 (extended records like Hyperlink=226, RTFLink=241): **sort ascending by
  record type**
- If one child is standard and one is extended, the extended record generally sorts to
  the end (its RECORD code is > 225 which exceeds typical insertion indices)

### No Explicit End Marker

SchLib component `Data` streams do NOT have an explicit RECORD=0 end marker. Reading
terminates at stream EOF. (SchDoc `FileHeader` streams do use RECORD=0 as end marker,
but SchLib components just end.)

### OwnerIndex: Relative Within Component

During save, owner indices must be **relative to the component's Data stream**:
- SchComponent at relative index 0 gets `OwnerIndex=-1` (no parent)
- All children reference parents by relative position within the same component

---

## 8. FileHeader Stream

Single parameter text block (flags=0x00) containing all library metadata.

### Parameter Order

```
1.  HEADER                  = "Protel for Windows - Schematic Library Editor Binary File Version 5.0"
2.  Weight                  = sum(primitives + aliases) across all components
3.  MinorVersion            = 9 (current) or 2 (legacy)
4.  UniqueID                = library-level unique identifier

-- Font Table (1-based indexing) --
5.  FontIdCount             = N
6.  Size1                   = points (i32)
7.  Rotation1               = 0
8.  Underline1              = T/F
9.  Italic1                 = T/F
10. Bold1                   = T/F
11. StrikeOut1              = T/F
12. FontName1               = "Times New Roman"
    ... repeat Size{i} through FontName{i} for i = 2..N ...

-- Display Settings --
    UseMBCS                 = T (always)
    IsBOC                   = T (always)
    SheetStyle              = 9 (custom)
    BorderOn                = T/F
    SheetNumberSpaceSize    = 0
    AreaColor               = COLORREF
    SnapGridOn              = T/F
    SnapGridSize            = value
    VisibleGridOn           = T/F
    VisibleGridSize         = value
    CustomX                 = width
    CustomY                 = height
    UseCustomSheet          = T (always)
    ReferenceZonesOn        = T/F
    Display_Unit            = 0

-- Component Index (0-based) --
    CompCount               = M
    LibRef0                 = component_name
    CompDescr0              = description
    PartCount0              = parts
    AliasCount0             = aliases
    Comp0Alias0             = alias_name  (if AliasCount0 > 0)
    ... repeat for each component 0..M-1 ...
```

### Weight Calculation

```
Weight = sum over all components of:
    (number of records in Data stream, EXCLUDING SchComponent and end marker)
    + (number of aliases for that component)
```

---

## 9. Sidecar Stream Writing

### When to Write Each Stream

Each sidecar stream is written **only if at least one pin in the component has
non-default data** for that stream. The per-pin conditions:

| Stream | Write condition per pin |
|--------|----------------------|
| `PinFrac` | `location_x_frac != 0 OR location_y_frac != 0 OR length_frac != 0` |
| `PinDesc` | `description.len() > 254` |
| `PinMiscData` | `swap_id_pair` is non-empty |
| `PinTextData` | Any of name/designator position or font mode is Custom |
| `PinWideText` | Any of desc, name, desig, swap_id, swap_id_part, default_value is non-empty |
| `PinSymbolLineWidth` | `symbol_line_width != TSize::eZeroSize` |
| `PinPackageLength` | `pin_package_length != 0` |
| `PinPropagationDelay` | `propagation_delay != 0.0` |
| `PinFunctionData` | `selected_functions.count > 0 OR defined_functions.count > 0` |

### Embedded Object Envelope Format

All sidecar streams use the same envelope structure:

```
[header block, flags=0x00]
    |RECORD=0|HEADER=<StreamName>|Weight=<count>|\0

[entry blocks, flags=0x01, one per pin with data]
    0xD0                    (1 byte)  embedded object tag
    id_length               (1 byte)  length of pin index string
    id                      (N bytes) pin index as ASCII decimal ("0", "1", "15")
    inner_header            (4 bytes) bits[23:0]=data_len, bits[31:24]=0x00
    inner_data              (M bytes) stream-specific payload
```

Pin indices are **0-based** and reference the pin's position in the ordered list of
pins within the component (the order they appear in the `Data` stream).

### Stream-Specific Inner Data Formats

**PinFrac** (12 bytes):
```
i32 LE  location_x_frac    (internal_value % 100_000)
i32 LE  location_y_frac
i32 LE  length_frac
```

**PinDesc** (length-prefixed ASCII):
```
u32 LE  text_length
bytes   ASCII text (description[254..], the overflow portion)
```

**PinMiscData** (length-prefixed UTF-16LE params):
```
u32 LE  byte_length
bytes   UTF-16LE: "PairSwapID=<value>"
```

**PinTextData** (2-22 bytes binary):
```
[name text data struct]
[designator text data struct]

Each struct:
  byte   flags (bit0=custom_position, bit1=rotation_anchor, bit2-3=rotation, bit4=custom_font)
  if custom_position: i32 LE custom_margin
  if custom_font:     i16 LE font_id, u32 LE color
```

**PinWideText** (length-prefixed UTF-16LE params):
```
u32 LE  byte_length
bytes   UTF-16LE: "|Desc=...|Name=...|Desig=...|SwapId=...|SwapIDPart=...|DefValue=...|"
```

Only include keys for fields that are non-empty.

**PinSymbolLineWidth** (length-prefixed UTF-16LE params):
```
u32 LE  byte_length
bytes   UTF-16LE: "SymBol_LineWidth=<value>"
```

**PinPackageLength** (length-prefixed UTF-16LE params):
```
u32 LE  byte_length
bytes   UTF-16LE: "PinPackageLength=<value>"
```

**PinPropagationDelay** (length-prefixed UTF-16LE params):
```
u32 LE  byte_length
bytes   UTF-16LE: "PinPropagationDelay=<value>"
```

**PinFunctionData** (length-prefixed UTF-16LE params):
```
u32 LE  byte_length
bytes   UTF-16LE: "PinSelectedFunctionsCount=N|PinSelectedFunction1=...|...|
                   PinDefinedFunctionsCount=M|PinDefinedFunction1=...|..."
```

Function indices are **1-based**.

---

## 10. Storage Stream (Embedded Images)

The `/Storage` stream holds embedded binary objects (images) using the same envelope:

```
[header block, flags=0x00]
    |RECORD=0|HEADER=Icon storage|Weight=<count>|\0

[entry blocks, flags=0x01, one per image]
    0xD0                    (1 byte)  embedded object tag
    id_length               (1 byte)  length of image filename
    id                      (N bytes) image filename (ASCII)
    inner_header            (4 bytes) bits[23:0]=compressed_size, bits[31:24]=0x00
    inner_data              (M bytes) zlib-compressed image data
```

Zlib compression uses standard deflate (header bytes typically `0x78 0x9C`).

---

## 11. SectionKeys Stream

Written only if any component name exceeds 31 characters after character sanitization.

Format: single parameter text block:
```
|KEYCOUNT=<N>|LIBREF0=<full_name>|SECTIONKEY0=<short_key>|...|
```

### Component Key Generation

1. Replace invalid CFB characters (`` /\:*?"<>|! ``) with `_`
2. If result length > 31, truncate to 31 characters
3. If truncation creates a collision, append numeric suffix within 31-char limit
4. Store full-name-to-key mapping in SectionKeys

---

## 12. Alias Redirection Streams

Each alias gets its own CFB sub-storage `/<AliasKey>/` containing only a
`Redirection` stream:

```
[single text block, flags=0x00]
    |RECORD=0|SectionName=<canonical_component_name>|\0
```

The `SectionName` value is the canonical component's **full name** (not its CFB key).

---

## 13. CFB Container Writing

### Required Capabilities

The `cfb` crate (Rust) supports both reading AND writing CFB V3 files. Key operations:

1. `CompoundFile::create()` — create new CFB file
2. `cf.create_storage(path)` — create sub-storage
3. `cf.create_stream(path)` — create stream
4. `cf.write_stream(path, data)` — write stream data
5. `cf.flush()` — persist to disk

### Stream Write Order

Match Altium's export order for byte-identical output:

```
1. /FileHeader                        (library header + component index)
2. /Storage                           (embedded images)
3. /SectionKeys                       (optional, if long names)
4. For each component (in index order):
   a. /<key>/Data                     (component + child records)
   b. /<key>/PinFrac                  (optional)
   c. /<key>/PinDesc                  (optional)
   d. /<key>/PinMiscData              (optional)
   e. /<key>/PinTextData              (optional)
   f. /<key>/PinWideText              (optional)
   g. /<key>/PinSymbolLineWidth       (optional)
   h. /<key>/PinPackageLength         (optional)
   i. /<key>/PinPropagationDelay      (optional)
   j. /<key>/PinFunctionData          (optional)
   k. /<key>/Additional               (optional)
5. /LibAdditional                     (optional)
6. For each alias (in index order):
   a. /<alias>/Redirection
```

### CFB Metadata

- CFB Version: V3 (sector size 512 bytes)
- No custom OLE properties required
- Stream names and storage names are case-sensitive in CFB

---

## 14. Byte-Perfect Validation Strategy

### Round-Trip Test

```
1. Read original file → SchLib
2. Serialize SchLib → new CFB file
3. Compare original and new file byte-by-byte
4. On mismatch: report offset, expected byte, actual byte
```

### Incremental Comparison (for debugging mismatches)

Compare at each layer individually:

1. **CFB structure**: Compare stream names, storage hierarchy
2. **Stream contents**: Compare raw bytes of each stream
3. **Block headers**: Compare block count, sizes, flags per stream
4. **Parameter strings**: Compare decoded parameter key-value pairs per block
5. **Binary records**: Compare binary pin records byte-by-byte
6. **Sidecar streams**: Compare each sidecar entry's id and inner data

### Known Sources of Non-Determinism

- **CFB sector allocation**: The `cfb` crate may allocate sectors differently than
  Altium's OLE implementation. Byte-identical comparison at the CFB container level
  may not be achievable. Compare at the **stream content** level instead.
- **Parameter key casing**: Altium's keys use canonical casing from `FileFormatV5.cs`.
  Our parser is case-insensitive on read but must use exact canonical casing on write.
- **Floating-point formatting**: `StartAngle=0` vs `StartAngle=0.0` — must match
  Altium's exact Delphi-compatible formatting.

---

## 15. Implementation Checklist

### Layer 1: Low-Level Writers (no domain knowledge needed)

- [ ] `write_text_block(params) → Vec<u8>` — encode params to pipe-delimited NUL-terminated Windows-1252, write 4-byte header + payload
- [ ] `write_binary_block(data) → Vec<u8>` — write 4-byte header (flags=0x01) + raw bytes
- [ ] `write_embedded_object(id, inner_data) → Vec<u8>` — encode 0xD0 envelope
- [ ] `write_embedded_object_stream(header, entries) → Vec<u8>` — header block + entry blocks
- [ ] `ParameterCollection::to_bytes() → Vec<u8>` — pipe-delimited, escaped, Windows-1252, NUL-terminated
- [ ] `zlib_compress(data) → Vec<u8>` — standard zlib compression (for Storage stream images)

### Layer 2: Record Serializers (one per record type)

- [ ] `SchComponent::to_params() → ParameterCollection` — hardcoded order from Section 4
- [ ] `SchPin::to_binary() → Vec<u8>` — binary pin format from Section 5
- [ ] `SchPin::to_params() → ParameterCollection` — text format (needed for Additional stream)
- [ ] `SchArc::to_params()`, `SchLine::to_params()`, `SchRectangle::to_params()`, etc.
- [ ] `SchDesignator::to_params()`, `SchParameter::to_params()`
- [ ] `SchImplementation::to_params()`, `SchMapDefiner::to_params()`, etc.
- [ ] `#[derive(ToParams)]` proc macro mirroring `#[derive(FromParams)]`

### Layer 3: Pin Sidecar Writers

- [ ] `write_pin_frac(pins) → Option<Vec<u8>>` — sparse, only if fractional parts exist
- [ ] `write_pin_desc(pins) → Option<Vec<u8>>` — sparse, only if description > 254 chars
- [ ] `write_pin_misc_data(pins) → Option<Vec<u8>>` — sparse
- [ ] `write_pin_text_data(pins) → Option<Vec<u8>>` — sparse
- [ ] `write_pin_wide_text(pins) → Option<Vec<u8>>` — sparse
- [ ] `write_pin_symbol_line_width(pins) → Option<Vec<u8>>` — sparse
- [ ] `write_pin_package_length(pins) → Option<Vec<u8>>` — sparse
- [ ] `write_pin_propagation_delay(pins) → Option<Vec<u8>>` — sparse
- [ ] `write_pin_function_data(pins) → Option<Vec<u8>>` — sparse

### Layer 4: Document-Level Serialization

- [ ] `SchLib::write_file_header() → Vec<u8>` — FileHeader stream bytes
- [ ] `SchLib::write_storage() → Vec<u8>` — Storage stream (embedded images)
- [ ] `SchLib::write_section_keys() → Option<Vec<u8>>` — SectionKeys (if needed)
- [ ] `SchLibComponent::write_data() → Vec<u8>` — component Data stream
- [ ] `SchLib::write_alias_redirection(alias) → Vec<u8>` — Redirection stream

### Layer 5: CFB Assembly

- [ ] `SchLib::save_to_file(path) → Result<()>` — create CFB, write all streams
- [ ] `SchLib::save_as(path) → Result<()>` — public API

### Layer 6: Validation

- [ ] `validate_round_trip(original_path, output_path) → Result<Vec<Mismatch>>` — stream-level comparison
- [ ] CLI command: `altium-cli schlib validate --original file.SchLib --output copy.SchLib`

---

## 16. Source References

### C# Decompiled Sources

| File | Purpose |
|------|---------|
| `FileFormatV5.cs` (5575 lines) | Per-record parameter order, all Export_* call sites |
| `SchDataExporterBaseV5.cs` | Save pipeline orchestration |
| `SchDataExporterLibraryV5.cs` | SchLib-specific save (sidecar streams, component iteration) |
| `SchDataSerializerParam.cs` | Tier 1/2 sparse-save method implementations |
| `SchDataSerializer.cs` | Base serializer (Export_Coord, Export_Boolean, etc.) |
| `SchDataObjectComparator.cs` | Child record sort order |
| `FileFormatConsts.cs` | Stream name constants, header strings |
| `BinaryFileCode.cs` | Binary instruction codes (0x02=Pin, 0xD0=Embedded, 0xFE=Extended) |

### Existing Codebase (Read Path — Reference for Inverse)

| File | Purpose |
|------|---------|
| `crates/altium-format/src/schlib.rs` | SchLib loading pipeline |
| `crates/altium-format/src/sch_records.rs` | All record type definitions + parse_binary_pin |
| `crates/altium-format/src/param_collection.rs` | ParameterCollection (read-only today) |
| `crates/altium-format/src/block_stream.rs` | Block parsing (read-only today) |
| `crates/altium-format/src/embedded_object.rs` | Embedded object parsing (read-only today) |
| `crates/altium-format/src/binary_io.rs` | BinaryReader + BinaryWriter (writer exists but unused) |
| `crates/altium-format/src/param_value.rs` | ToParamValue trait implementations |
| `crates/altium-format-derive/src/lib.rs` | FromParams derive macro (needs ToParams counterpart) |
