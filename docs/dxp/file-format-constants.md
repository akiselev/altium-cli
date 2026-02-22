# FileFormatConsts.cs Reference

Comprehensive analysis of all constants in `Altium.Sch.DataModel.FileFormats.FileFormatConsts`.

Source: `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs`

---

## Table of Contents

1. [File Format Strings (Runtime Tags)](#1-file-format-strings-runtime-tags)
2. [File Header Strings (On-Disk Identifiers)](#2-file-header-strings-on-disk-identifiers)
3. [OLE Stream Names](#3-ole-stream-names)
4. [Record Types (RECORD Values)](#4-record-types-record-values)
5. [Core Record Structure Parameters](#5-core-record-structure-parameters)
6. [Coordinate Parameters](#6-coordinate-parameters)
7. [Component Parameters](#7-component-parameters)
8. [Pin Parameters](#8-pin-parameters)
9. [Visual / Style Parameters](#9-visual--style-parameters)
10. [Sheet / Document Parameters](#10-sheet--document-parameters)
11. [Text / Label Parameters](#11-text--label-parameters)
12. [Electrical Parameters](#12-electrical-parameters)
13. [Drawing / Shape Parameters](#13-drawing--shape-parameters)
14. [Model / Footprint Parameters](#14-model--footprint-parameters)
15. [Locking / Visibility Parameters](#15-locking--visibility-parameters)
16. [Database / Vault Parameters](#16-database--vault-parameters)
17. [Harness Connectivity Parameters](#17-harness-connectivity-parameters)
18. [Harness Physical Parameters](#18-harness-physical-parameters)
19. [Harness Color Parameters](#19-harness-color-parameters)
20. [Reuse Block Parameters](#20-reuse-block-parameters)
21. [Cross-Reference / Document Parameters](#21-cross-reference--document-parameters)
22. [Special / Internal Parameters](#22-special--internal-parameters)
23. [Data Version Constants](#23-data-version-constants)
24. [Unit System](#24-unit-system)
25. [Wire Type Summary](#25-wire-type-summary)

---

## 1. File Format Strings (Runtime Tags)

These are **internal runtime dispatch identifiers** stored on the document object as a `DataFormat` property. They are **not** written into the file. They are set by `FileFormatUtils.GetDataFormatByParameters(TSerializerType, TFileFormatVersion)` at the start of every import/export operation.

| Constant | Value | Purpose |
|----------|-------|---------|
| `SchFormatStringAscii` | `"Advanced Schematic ascii(*.asc)"` | ASCII schematic sheet (V4 or V5) |
| `SchFormatStringBinaryV40` | `"Advanced Schematic binary v4.0 (*.sch)"` | V4 binary schematic sheet |
| `SchFormatStringBinaryV50` | `"Advanced Schematic binary v5.0 (*.sch)"` | V5 binary schematic sheet |
| `SchFormatStringJSON` | `"Advanced Schematic json(*.json)"` | JSON schematic sheet |
| `SchFormatStringLibraryAscii` | `"Advanced Schematic ascii library(*.asc)"` | ASCII schematic library |
| `SchFormatStringLibraryBinaryV40` | `"Advanced Schematic binary library v4.0 (*.lib)"` | V4 binary schematic library |
| `SchFormatStringLibraryBinaryV50` | `"Advanced Schematic binary library v5.0 (*.lib)"` | V5 binary schematic library |

The dispatch mapping from `GetDataFormatByParameters()`:

```
TFileFormatVersion::ffv4 + stParametric/stBinary       → BinaryV40
TFileFormatVersion::ffv4 + stParametricAscii/stAscii   → Ascii
TFileFormatVersion::ffv4 + stParametricJSON             → JSON
TFileFormatVersion::ffv5 + stParametric                 → BinaryV50
TFileFormatVersion::ffv5 + stParametricAscii            → Ascii
TFileFormatVersion::ffv5 + stParametricJSON             → JSON
```

---

## 2. File Header Strings (On-Disk Identifiers)

These strings are written into the `HEADER` parameter of the `FileHeader` OLE stream (or as the first line of ASCII files). They are the actual on-disk format identifiers.

### Schematic Sheet Headers

| Constant | Value | Container | Era |
|----------|-------|-----------|-----|
| `SchSheetBinaryHeaderV40` | `"Protel for Windows - Schematic Capture Binary File Version 1.2 - 2.0"` | Plain binary (NOT OLE2) | Legacy (Protel 98/99/DXP) |
| `SchSheetAsciiHeaderV50` | `"Protel for Windows - Schematic Capture Ascii File Version 5.0"` | Plain text | Current |
| `SchSheetBinaryHeaderV50` | `"Protel for Windows - Schematic Capture Binary File Version 5.0"` | OLE2 compound doc | Current |
| `SchSheetJSONHeaderV50` | `"Altium Designer - Schematic Capture Json File Version 5.0"` | OLE2 compound doc | Current |

### Schematic Library Headers

| Constant | Value | Container | Era |
|----------|-------|-----------|-----|
| `SchLibraryAsciiHeaderV40` | `"Protel for Windows - Schematic Library Editor Ascii File Version 1.2 - 2.0"` | Plain text | Legacy |
| `SchLibraryBinaryHeaderV40` | `"Protel for Windows - Schematic Library Editor Binary File Version 1.2 - 2.0"` | Plain binary | Legacy |
| `SchLibraryAsciiHeaderV50` | `"Protel for Windows - Schematic Library Editor Ascii File Version 5.0"` | Plain text | Current |
| `SchLibraryBinaryHeaderV50` | `"Protel for Windows - Schematic Library Editor Binary File Version 5.0"` | OLE2 compound doc | Current |
| `SchLibraryJSONHeaderV50` | `"Altium Designer - Schematic Library Editor Json File Version 5.0"` | OLE2 compound doc | Current |

### Harness Headers

| Constant | Value |
|----------|-------|
| `HarnessWiringDiagramBinaryHeaderV1` | `"Altium Designer - Harness Wiring Diagram Binary File Version 1.0"` |
| `HarnessWiringDiagramAsciiHeaderV1` | `"Altium Designer - Harness Wiring Diagram Ascii File Version 1.0"` |
| `HarnessWiringDiagramJSONHeaderV1` | `"Altium Designer - Harness Wiring Diagram JSON File Version 1.0"` |
| `HarnessLayoutDrawingBinaryHeaderV1` | `"Altium Designer - Harness Layout Drawing Binary File Version 1.0"` |
| `HarnessLayoutDrawingAsciiHeaderV1` | `"Altium Designer - Harness Layout Drawing Ascii File Version 1.0"` |
| `HarnessLayoutDrawingJSONHeaderV1` | `"Altium Designer - Harness Layout Drawing JSON File Version 1.0"` |
| `HarnessLibraryBinaryHeaderV1` | `"Altium Designer - Harness Library Binary File Version 1.0"` |
| `HarnessLibraryAsciiHeaderV1` | `"Altium Designer - Harness Library Ascii File Version 1.0"` |
| `HarnessLibraryJSONHeaderV1` | `"Altium Designer - Harness Library JSON File Version 1.0"` |

### Electronics System Design

| Constant | Value |
|----------|-------|
| `ElectronicsSystemDesignJSONHeaderV1` | `"Altium Designer - Electronics System Design JSON File Version 1.0"` |

### "Protel" vs "Altium Designer" Naming

The "Protel for Windows" prefix is a historical artifact from the Protel EDA era (pre-2002). The header strings were never changed for backward compatibility. "Altium Designer" appears only in newer file types (JSON format, harness files, ESD) that were never tied to the Protel era.

### V4 vs V5

**V4** ("Version 1.2 - 2.0") is the legacy format from the Protel era. V4 files are plain binary or plain ASCII -- **not** OLE2 compound documents. They have no `MinorVersion` or `UniqueID` fields.

**V5** ("Version 5.0") is the current working format. V5 binary files are OLE2 compound documents containing multiple named streams. The `FileHeader` stream contains `RECORD`, `HEADER`, `Weight`, `MinorVersion`, and `UniqueID`.

### Format Detection

Detection is performed by `FileFormatUtils.GetFileKindFromFileName()`:

1. **V50 ASCII**: reads first line, checks for keywords `{"Ascii", "Schematic", "5.0"}`
2. **V40 ASCII**: reads first line, checks for `{"Ascii", "Schematic"}` and `{"1.2", "2.0"}`
3. **V50 Binary**: checks OLE2 magic bytes `D0 CF 11 E0 A1 B1 1A E1`, opens `FileHeader` stream, exact case-insensitive match on `HEADER` string
4. **V40 Binary Library**: opens with `SchDataSerializerBinary`, reads `HEADER`, checks for `{"Binary", "Library"}` + `{"1.2", "2.0"}`
5. **V50 Binary Library**: same OLE2 probe as V50 binary sheet
6. **V40 Binary Sheet**: reads first 255 bytes, checks for `{"Binary", "Schematic"}` + `{"1.2", "2.0"}`
7. **Harness files**: same OLE2 probe, exact match on harness header strings
8. **ESD**: extension-based only (no header string matching)

---

## 3. OLE Stream Names

These are the named streams within OLE2 compound document files.

### Core Streams (All File Types)

| Stream | File Types | Format | Purpose |
|--------|-----------|--------|---------|
| `FileHeader` | All V5 types | Parametric binary | File header: `HEADER`, `Weight` (record count), `MinorVersion`, `UniqueID`; in SchLib also contains the full component index (`CompCount`, `LibRef{N}`, `CompDescr{N}`, `PartCount{N}`, `AliasCount{N}`, aliases) |
| `Storage` | All types with images | Parametric + raw binary blobs | Embedded image data for `ISchDataImage` objects with `EmbedImage=true`. Header: `HEADER="Icon storage"`, `Weight=count`. Each entry: BINARY instruction (byte 208) + named blob |

### SchDoc-Only Streams

| Stream | Format | Purpose |
|--------|--------|---------|
| `Additional` | Parametric binary | Overflow objects whose `OwnerIndexAdditionalList=true` (owner index refers to this stream instead of main FileHeader) |
| `ReuseBlocks` | Raw LE binary blob | V1 reuse block info: version(i32), count(i32), then per-block: id, vault GUIDs, snippet GUIDs, part UniqueIDs |
| `ReuseBlocksV2` | Raw LE binary blob | V2 extension: adds PCB snippet vault/item/revision GUIDs per reuse block |
| `ObjectDefinitions` | Parametric binary | Object definition records (RECORD=129) referenced by `ObjectDefinitionId` from ports, power symbols, etc. |
| `ReuseBlockInfos` | Parametric binary | Dissolved reuse block tracking records (RECORD=138) |

### SchLib-Only Streams

| Stream | Location | Format | Purpose |
|--------|----------|--------|---------|
| `SectionKeys` | Root | Parametric | Mapping of `LibRef{N}` names to OLE section keys (handles name truncation to 31-char OLE limit) |
| `Data` | Per-component section | Parametric binary | Component primitives (component record, pins, parameters, sub-objects), terminated by RECORD=0 |
| `Additional` | Per-component section | Parametric binary | Per-component overflow objects |
| `LibAdditional` | Root | Parametric binary | Top-level container wrapping per-component `Additional` sub-streams |
| `Redirection` | Per-alias section | Parametric | Alias redirect: contains `SectionName` pointing to canonical component name |

### SchLib Pin Sidecar Streams (Per-Component Section)

All pin sidecar streams share the same outer wrapper: RECORD 0 + HEADER + Weight + BINARY 208 entries per pin. Each entry is named by pin index (decimal string).

| Stream | Blob Format | Purpose |
|--------|-------------|---------|
| `PinFrac` | `i32 locationX_frac, i32 locationY_frac, i32 pinLength_frac` | Sub-unit fractional coordinate corrections for pins |
| `PinDesc` | `i32 byte_length, ASCII bytes` | Pin description overflow (characters beyond 254) |
| `PinMiscData` | `i32 byte_length, UTF-16LE parametric string` | Misc pin data (`PairSwapID=...`) |
| `PinWideText` | `i32 byte_length, UTF-16LE parametric string` | Unicode pin fields (`Desc=...\|Name=...\|Desig=...\|SwapId=...\|SwapIDPart=...\|DefValue=...`) |
| `PinTextData` | Compact binary structure | Custom pin text position/font mode for name and designator labels (see [Pin Parameters](#8-pin-parameters)) |
| `PinSymbolLineWidth` | `i32 byte_length, UTF-16LE parametric string` | Pin symbol line width override (`SymBol_LineWidth=N`) |
| `PinPackageLength` | `i32 byte_length, UTF-16LE parametric string` | Physical package pin length (`PinPackageLength=N`) |
| `PinPropagationDelay` | `i32 byte_length, UTF-16LE parametric string` | Signal propagation delay in scientific notation (`PinPropagationDelay=Xe-Y`) |
| `PinFunctionData` | `i32 byte_length, UTF-16LE parametric string` | Selected/defined pin functions (`PinSelectedFunctionsCount=N\|PinSelectedFunction1=...\|...`) |

### Harness-Only Streams

| Stream | File Type | Format | Purpose |
|--------|-----------|--------|---------|
| `HarnessConnectionPointConnector` | Harness Layout Drawing | Raw LE binary blob | Connector/pin assignments for connection points. Version(i32=1), count(i32), then per-point: UniqueId, connector count, per-connector: id + pin IDs |
| `HarnessComponentCrimps` | Harness (declared, unused in .NET) | Unknown | Reserved/legacy |
| `HarnessAssociatedParts` | Harness (declared, unused in .NET) | Unknown | Reserved/legacy |
| `Files` | Harness Layout Drawing | Binary (instruction byte 227) | Embedded compressed files (physical model images), keyed by GUID + hash |

### EmbeddedData Name Constants

These name the logical payload objects inside streams (set via `SetName()`). They happen to have the same string values as the stream names but serve a different structural role:

| Constant | Value | Notes |
|----------|-------|-------|
| `EmbeddedDataNameReuseBlocks` | `"ReuseBlocks"` | Used for both V1 and V2 embedded objects |
| `EmbeddedDataNameHarnessConnectionPointConnector` | `"HarnessConnectionPointConnector"` | |
| `EmbeddedDataNameHarnessComponentCrimps` | `"HarnessComponentCrimps"` | |
| `EmbeddedDataNameHarnessAssociatedParts` | `"HarnessAssociatedParts"` | |

---

## 4. Record Types (RECORD Values)

Every object in the Data stream is preceded by a `RECORD` byte identifying its type. When the value is 254, a full `RECORDEX` i32 follows.

| RECORD | Object Type | Category |
|--------|-------------|----------|
| 1 | Component | Core |
| 2 | Pin | Core |
| 3 | Symbol (IEEE symbol) | Core |
| 4 | Label | Core |
| 5 | Bezier | Drawing |
| 6 | Polyline | Drawing |
| 7 | Polygon | Drawing |
| 8 | Ellipse | Drawing |
| 9 | Pie | Drawing |
| 10 | RoundRectangle | Drawing |
| 11 | EllipticalArc | Drawing |
| 12 | Arc | Drawing |
| 13 | Line | Drawing |
| 14 | Rectangle | Drawing |
| 15 | SheetSymbol | Hierarchy |
| 16 | SheetEntry | Hierarchy |
| 17 | PowerObject / CrossSheetConnector | Connectivity |
| 18 | Port | Connectivity |
| 22 | NoERC | Annotation |
| 23 | ErrorMarker | Annotation |
| 25 | NetLabel | Connectivity |
| 26 | Bus | Connectivity |
| 27 | Wire | Connectivity |
| 28 | TextFrame | Annotation |
| 29 | Junction | Connectivity |
| 30 | Image | Drawing |
| 31 | Sheet | Document |
| 32 | SheetName | Document |
| 33 | SheetFileName | Document |
| 34 | Designator | Component child |
| 37 | BusEntry | Connectivity |
| 39 | Template | Document |
| 40 | TaskHolder | Annotation |
| 41 | Parameter / ImageParameter | Component child |
| 43 | ParameterSet | Component child |
| 44 | ImplementationsList | Component child |
| 45 | Implementation | Component child |
| 46 | ImplementationMap | Component child |
| 47 | MapDefiner | Component child |
| 48 | ParameterList | Component child |
| 104 | HarnessWiringDiagram | Harness |
| 105 | HarnessLayoutDrawing | Harness |
| 106 | HarnessComponent | Harness |
| 107 | HarnessWire | Harness |
| 108 | HarnessSplice | Harness |
| 109 | HarnessLayoutLabel | Harness |
| 110 | HarnessLayoutConnectionPoint | Harness |
| 111 | HarnessBundle | Harness |
| 112 | HarnessLogicalSignal | Harness |
| 113 | HarnessPin | Harness |
| 114 | HarnessWireLabel | Harness |
| 115 | HarnessWireData | Harness |
| 116 | HarnessSpliceData | Harness |
| 117 | HarnessShield | Harness |
| 118 | HarnessTwist | Harness |
| 119 | HarnessNoConnect | Harness |
| 120 | HarnessNoConnectData | Harness |
| 121 | HarnessShieldData | Harness |
| 122 | HarnessTwistData | Harness |
| 123 | HarnessCable | Harness |
| 124 | HarnessCableData | Harness |
| 125 | HarnessAssociatedParts | Harness |
| 126 | LineView | Harness |
| 127 | HarnessLibrary | Harness |
| 128 | HarnessCovering | Harness |
| 129 | ObjectDefinition | Document |
| 130 | HarnessWireBreak | Harness |
| 131 | AssociatedObjects | Harness |
| 132 | ElectronicsSystemDesignDocument | ESD |
| 133 | FunctionalBlock | ESD |
| 134 | FunctionalConnectionLine | ESD |
| 135 | FunctionalTextFrame | ESD |
| 136 | SchematicBlock | Reuse |
| 137 | ReuseSheetSymbol | Reuse |
| 138 | ReuseBlockImplementationInfo | Reuse |
| 200 | SchLib (library root) | Library |
| 209 | Note | Annotation |
| 210 | Probe | Annotation |
| 211 | CompileMask | Annotation |
| 215 | HarnessConnector | Harness |
| 216 | HarnessEntry | Harness |
| 217 | HarnessConnectorType | Harness |
| 218 | SignalHarness | Harness |
| 220 | HighLevelCodeSymbol | ESD |
| 221 | HighLevelCodeEntry | ESD |
| 222 | HighLevelCode SheetName | ESD |
| 223 | HighLevelCode SheetFileName | ESD |
| 225 | Blanket | Annotation |
| 226 | Hyperlink | Annotation |
| 240 | RichTextDocument | Annotation |
| 241 | RTFLink | Annotation |
| 254 | (escape sentinel for RECORDEX) | Internal |

---

## 5. Core Record Structure Parameters

Every object begins with these base fields.

### DataObject Base Fields

| Parameter | Type | Notes |
|-----------|------|-------|
| `OwnerIndex` | i32 | Index of parent object in the flat record array. Record 0 is always the sheet/library root. |
| `IsNotAccesible` | bool | Stored **inverted**: `true` in file = NOT accessible. Note the intentional typo (one 's'). |
| `OwnerIndexAdditionalList` | bool | When true, `OwnerIndex` refers to the `Additional` sidecar stream, not the main Data stream. |
| `IndexInSheet` | i32 | Sequential index of this object within the sheet. Default on import: `-1`. |
| `IgnoreOnLoad` | bool | When true, skip this object during load. Only written when true. |

### GraphicalObject Additional Fields

| Parameter | Type | Notes |
|-----------|------|-------|
| `OwnerPartId` | i16 | Which part (1..N) of a multi-part component. `-1` = all parts. |
| `OwnerPartDisplayMode` | u8 | Which display mode (0..N-1) this belongs to. |
| `SelectionMemory` | u8 | 8-bit bitmask for selection memory group membership. |
| `UnionIndex` | i32 | Union group index. |
| `GraphicallyLocked` | bool | Exported but **always reset to false on import** (deprecated). |

---

## 6. Coordinate Parameters

All coordinates are stored as integers in DXP units: **1 DXP unit = 1/100,000 mil = 0.0000254 mm**.

- 1 mil = 100,000 DXP units
- 10 mils (one schematic grid square) = 1,000,000 DXP units

### Coordinate Serialization

**Binary serializer**: stores as 16-bit signed integer (whole mils) plus optional 32-bit `ParamName_Frac` (fractional remainder). Fraction is omitted when zero.

```
whole = coord / 100000
fraction = coord - 100000 * whole
```

**ASCII serializer**: reads/writes only the integer mil part, multiplied by 100,000 on import.

### Coordinate Parameters

| Parameter | Object Types | Purpose |
|-----------|-------------|---------|
| `Location.X`, `Location.Y` | Most objects | Origin/anchor position |
| `Corner.X`, `Corner.Y` | Rectangle, Line, RoundRectangle, Blanket, SchematicBlock | Opposite corner |
| `X{N}`, `Y{N}` | Polygon vertices (indexed 1..50) | Vertex coordinates |
| `EX{N}`, `EY{N}` | Polyline/wire overflow vertices (indexed, when >50 points) | Extra vertex coords |
| `Radius` | Arc, EllipticalArc, Pie, Ellipse | Primary radius |
| `SecondaryRadius` | EllipticalArc, Ellipse | Y-axis radius for ellipses |
| `CornerXRadius` | RoundRectangle | Horizontal corner rounding (default 20 mils) |
| `CornerYRadius` | RoundRectangle | Vertical corner rounding (default 20 mils) |
| `CustomX`, `CustomY` | Sheet/Library | Custom sheet size (default 1500, 950 mils) |
| `SnapGridSize` | Sheet/Library | Snap grid spacing |
| `VisibleGridSize` | Sheet/Library | Visible grid spacing |
| `HotSpotGridSize` | Sheet/Library | Hot spot grid spacing (default 8 mils) |

### Vertex Overflow (EXTRALOCATIONCOUNT)

Polylines store up to 50 vertices using `X{N}`/`Y{N}` keys. When a polyline has more than 50 vertices, `EXTRALOCATIONCOUNT` stores the overflow count and additional points use `EX{N}`/`EY{N}` keys.

---

## 7. Component Parameters

From `ExportComponent` / `ImportComponent`:

| Parameter | Type | Notes |
|-----------|------|-------|
| `LibReference` | string | Component name in the originating library (primary identifier) |
| `ComponentDescription` | string | Human-readable description |
| `PartCount` | i16 | Number of parts in a multi-part component (gate count) |
| `DisplayModeCount` | u8 | Number of alternate symbol display modes |
| `DisplayMode` | u8 | Currently active display mode (0..N-1) |
| `Location.X`, `Location.Y` | coord | Component placement origin |
| `IsMirrored` | bool | Horizontal mirror |
| `Orientation` | u8 | TRotationBy90: 0=0deg, 1=90deg, 2=180deg, 3=270deg |
| `CurrentPartId` | i16 | Currently active part (1..PartCount) |
| `ShowHiddenFields` | bool | |
| `LibraryPath` | string | Path to source library |
| `DesignItemId` | string | Vault/managed item identifier |
| `AliasList` | string | Comma-separated list of component aliases |
| `NotUseLibraryName` | bool | Stored inverted from UseLibraryName |
| `DesignatorLocked` | bool | |
| `PartIDLocked` | bool | Defaults to DesignatorLocked value if absent |
| `PinsMoveable` | bool | |
| `AllPinCount` | i16 | Total pin count across all parts |
| `KeyComponentUniqueId` | string | Unique ID of key component for multi-channel |
| `ComponentKind` | u8 | TComponentKind value (v1) |
| `ComponentKindVersion2` | u8 | Extended kind (v2); wins if >= 5 |
| `ComponentKindVersion3` | u8 | Further extended kind (v3); if = 6, overrides v2 |
| `HasOnlyCurrentPartInfo` | bool | |
| `CustomDisplayModeName{N}` | string | Name for each display mode |

### TComponentKind Enum

| Value | Name | Description |
|-------|------|-------------|
| 0 | `eComponentKind_Standard` | Normal component |
| 1 | `eComponentKind_Mechanical` | Mechanical (no logical net connections) |
| 2 | `eComponentKind_Graphical` | Purely graphical (no BOM, no connections) |
| 3 | `eComponentKind_NetTie_BOM` | Net tie, appears in BOM |
| 4 | `eComponentKind_NetTie_NoBOM` | Net tie, not in BOM |
| 5 | `eComponentKind_Standard_NoBOM` | Standard but excluded from BOM |
| 6 | `eComponentKind_Jumper` | Jumper component |

ComponentKind versioning: if `V3 == 6` -> Jumper; else if `V2 >= 5` -> use V2; else use V1.

### LibRef vs LibReference

- `LibRef` is used in the **library file header** (`SectionKeys` stream) as indexed keys (`LibRef0`, `LibRef1`, ...) mapping component names to section keys.
- `LibReference` is the component name stored **within each component record itself**.

---

## 8. Pin Parameters

### PinConglomerate (Packed Byte Bitfield)

A single byte containing multiple flags and the orientation:

```
Bits [1:0]  Orientation          TRotationBy90 (0=0deg, 1=90deg, 2=180deg, 3=270deg)
Bit  2      IsHidden             0x04
Bit  3      ShowName             0x08
Bit  4      ShowDesignator       0x10
Bit  5      NotAccessible        0x20 (stored INVERTED: 1 = NOT accessible)
Bit  6      GraphicallyLocked    0x40 (written but NEVER read back on import)
Bit  7      OwnerIndexAdditionalList  0x80 (OwnerIndex refers to Additional stream)
```

### Pin Record Fields (After PinConglomerate)

| Parameter | Type | Notes |
|-----------|------|-------|
| `OwnerIndex` | i32 | Index of parent component |
| `OwnerPartId` | i16 | Which part (-1 = all) |
| `OwnerPartDisplayMode` | u8 | Which display mode |
| `SymBol_InnerEdge` | u8 | TIeeeSymbol at component body side |
| `SymBol_OuterEdge` | u8 | TIeeeSymbol at net connection side |
| `SymBol_Inner` | u8 | Inner body symbol |
| `SymBol_Outer` | u8 | Outer body symbol |
| `Description` | string | Human-readable pin description |
| `FormalType` | u8 | TStdLogicState (VHDL formal type) |
| `Electrical` | u8 | TPinElectrical (electrical type) |
| `PinConglomerate` | u8 | Packed bitfield (see above) |
| `PinLength` | coord | Length of pin line in DXP units |
| `Location.X`, `Location.Y` | coord | Pin endpoint (net connection end) |
| `Color` | u32 | BGR color |
| `Name` | string | Pin function/signal name |
| `Designator` | string | Pin number/designator |
| `SwapIdPin` | string | Pin swap group within part |
| `SwapIDPart` | string | Part swap group ID |
| `DefaultValue` | string | Default value |
| `SwapIdPair` | string | (ASCII-only) Pair swap ID |

### TPinElectrical Enum

| Value | Name |
|-------|------|
| 0 | `eElectricInput` |
| 1 | `eElectricIO` |
| 2 | `eElectricOutput` |
| 3 | `eElectricOpenCollector` |
| 4 | `eElectricPassive` |
| 5 | `eElectricHiZ` |
| 6 | `eElectricOpenEmitter` |
| 7 | `eElectricPower` |

### TIeeeSymbol Enum (For SymBol_ Fields)

| Value | Name | Description |
|-------|------|-------------|
| 0 | `eNoSymbol` | No symbol |
| 1 | `eDot` | Active-low bubble |
| 2 | `eRightLeftSignalFlow` | |
| 3 | `eClock` | Clock edge triangle |
| 4 | `eActiveLowInput` | |
| 5 | `eAnalogSignalIn` | |
| 6 | `eNotLogicConnection` | |
| 7 | `eShiftRight` | |
| 8 | `ePostPonedOutput` | |
| 9 | `eOpenCollector` | |
| 10 | `eHiz` | |
| 11 | `eHighCurrent` | |
| 12 | `ePulse` | |
| 13 | `eSchmitt` | |
| 14 | `eDelay` | |
| 15 | `eGroupLine` | |
| 16 | `eGroupBin` | |
| 17 | `eActiveLowOutput` | |
| 18 | `ePiSymbol` | |
| 19 | `eGreaterEqual` | |
| 20 | `eLessEqual` | |
| 21 | `eSigma` | |
| 22 | `eOpenCollectorPullUp` | |
| 23 | `eOpenEmitter` | |
| 24 | `eOpenEmitterPullUp` | |
| 25 | `eDigitalSignalIn` | |
| 26 | `eAnd` | |
| 27 | `eInvertor` | |
| 28 | `eOr` | |
| 29 | `eXor` | |
| 30 | `eShiftLeft` | |
| 31 | `eInputOutput` | |
| 32 | `eOpenCircuitOutput` | |
| 33 | `eLeftRightSignalFlow` | |
| 34 | `eBidirectionalSignalFlow` | |
| 35 | `eInternalPullUp` | |
| 36 | `eInternalPullDown` | |

### TStdLogicState Enum (FormalType)

| Value | Name |
|-------|------|
| 0 | `eStdLogic_Unitialized` |
| 1 | `eStdLogic_ForcingUnknown` |
| 2 | `eStdLogic_Forcing0` |
| 3 | `eStdLogic_Forcing1` |
| 4 | `eStdLogic_HiZ` |
| 5 | `eStdLogic_WeakUnknown` |
| 6 | `eStdLogic_Weak0` |
| 7 | `eStdLogic_Weak1` |
| 8 | `eStdLogic_DontCare` |

### PinName_PositionConglomerate (ASCII-Only, Packed Byte)

| Bit(s) | Flag | Notes |
|--------|------|-------|
| 0 | NamePositionMode custom | 1=custom, 0=default |
| 1 | NameRotationAnchor | 1=component, 0=pin |
| 3:2 | NameRotationRelative | TRotationBy90 |
| 4 | NameFontMode custom | 1=custom font, 0=default |

If custom position: `Name_CustomPosition_Margin` coord follows.
If custom font: `Name_CustomFontID` and `Name_CustomColor` follow.

### PinDesignator_PositionConglomerate (ASCII-Only, Packed Byte)

Identical structure for the designator text:

| Bit(s) | Flag |
|--------|------|
| 0 | DesignatorPositionMode custom |
| 1 | DesignatorRotationAnchor |
| 3:2 | DesignatorRotationRelative |
| 4 | DesignatorFontMode custom |

### PinTextData Stream Binary Format

For each pin, two text items (name + designator), each:

```
byte flags:
  bit 0: positionMode is Custom
  bit 1 (if custom): rotationAnchor (1=Component, 0=Pin)
  bits 2-3 (if custom): customRotationRelative (TRotationBy90)
  bit 4: fontMode is Custom
if positionMode==Custom:
  i32: customMargin
if fontMode==Custom:
  i16: customFontID (1-based index into font table)
  u32: customColor (BGR)
```

### ASCII-Only Pin Fields

| Parameter | Type | Notes |
|-----------|------|-------|
| `SymBol_LineWidth` | u8 | TSize enum -- width of IEEE symbol lines |
| `PinPackageLength` | coord | Physical package pin length |
| `PinPropagationDelay` | double | Signal propagation delay (scientific notation) |
| `HidePinNameAsFunction` | bool | Show name as function alias |
| `PinSelectedFunctionsCount` / `PinSelectedFunction{N}` | count + strings | Selected alternate functions |
| `PinDefinedFunctionsCount` / `PinDefinedFunction{N}` | count + strings | All defined functions |
| `PinSymbolicName` | string | Symbolic name for pin |
| `ShowPinSymbolicNameAsFunction` | bool | |

---

## 9. Visual / Style Parameters

### Color

Colors are **u32 BGR values** (Windows COLORREF format `0x00BBGGRR`), **not** ARGB.

### FontID / Font Table

FontIDs are 1-based indices into the per-document font table. The font table is written at the start of each document/library record:

| Parameter | Type | Notes |
|-----------|------|-------|
| `FontIdCount` | i16 | Number of font entries |
| `Size{N}` | i16 | Point size |
| `Rotation{N}` | i16 | Text rotation angle (typically 0 or 90) |
| `Underline{N}` | bool | |
| `Italic{N}` | bool | |
| `Bold{N}` | bool | |
| `StrikeOut{N}` | bool | |
| `FontName{N}` | string | e.g. "Times New Roman" (default if empty) |

On import, a `FontIdTranslator` maps file-local IDs to global runtime IDs.

### LineWidth / TSize Enum

| Value | Name |
|-------|------|
| 0 | `eZeroSize` (default/thinnest) |
| 1 | `eSmall` |
| 2 | `eMedium` |
| 3 | `eLarge` |

### LineStyle / LineStyleExt

| Value | Name |
|-------|------|
| 0 | `eLineStyleSolid` |
| 1 | `eLineStyleDashed` |
| 2 | `eLineStyleDotted` |
| 3 | `eLineStyleDashDotted` |

**Dual-field pattern**: `LineStyle` is the legacy field (clamped to 0..2, no DashDotted). `LineStyleExt` is the full value (ASCII-only byte). On import: take the larger of the two. Rectangles only use `LineStyleExt`.

### TTextJustification Enum

| Value | Name |
|-------|------|
| 0 | `eJustify_BottomLeft` |
| 1 | `eJustify_BottomCenter` |
| 2 | `eJustify_BottomRight` |
| 3 | `eJustify_CenterLeft` |
| 4 | `eJustify_Center` |
| 5 | `eJustify_CenterRight` |
| 6 | `eJustify_TopLeft` |
| 7 | `eJustify_TopCenter` |
| 8 | `eJustify_TopRight` |

### TTextHorzAnchor / TTextVertAnchor Enums

| Value | HorzAnchor | VertAnchor |
|-------|------------|------------|
| 0 | `None` | `None` |
| 1 | `Both` | `Both` |
| 2 | `Left` | `Top` |
| 3 | `Right` | `Bottom` |

---

## 10. Sheet / Document Parameters

| Parameter | Type | Notes |
|-----------|------|-------|
| `UseMBCS` | bool | Always `T` in V5. Controls MBCS string encoding. |
| `IsBOC` | bool | Deprecated, always written as `T` |
| `HotSpotGridOn` | bool | |
| `HotSpotGridSize` | coord | Default 8 mils |
| `SheetStyle` | u8 | TSheetStyle paper size preset |
| `SystemFont` | FontID | Default font for new objects |
| `DocumentBorderStyle` | u8 | TSheetDocumentBorderStyle |
| `WorkspaceOrientation` | enum | TSheetOrientation (Landscape/Portrait) |
| `BorderOn` | bool | Show border |
| `TitleBlockOn` | bool | Show title block |
| `SheetNumberSpaceSize` | i32 | |
| `Color` | u32 | Sheet background color |
| `AreaColor` | u32 | Sheet area (within border) color |
| `SnapGridOn` | bool | |
| `SnapGridSize` | coord | Default 10 mils |
| `VisibleGridOn` | bool | |
| `VisibleGridSize` | coord | Default 10 mils |
| `CustomX` | coord | Custom width (default 1500 mils) |
| `CustomY` | coord | Custom height (default 950 mils) |
| `UseCustomSheet` | bool | Use custom size vs TSheetStyle preset |
| `ShowHiddenPins` | bool | |
| `ReferenceZonesOn` | bool | **Stored inverted!** `T` in file = zones OFF |
| `CustomXZones` | i32 | Number of X reference zones (default 6) |
| `CustomYZones` | i32 | Number of Y reference zones (default 4) |
| `CustomMarginWidth` | coord | Default 20 mils |
| `ShowTemplateGraphics` | bool | |
| `TemplateFileName` | string | Path to sheet template (`.SchDot`) |
| `Display_Unit` | enum | TUnit display unit (affects runtime unit system) |

### TSheetStyle Enum

| Value | Name |
|-------|------|
| 0 | `eSheetA4` |
| 1 | `eSheetA3` |
| 2 | `eSheetA2` |
| 3 | `eSheetA1` |
| 4 | `eSheetA0` |
| 5 | `eSheetA` (ANSI A) |
| 6 | `eSheetB` (ANSI B) |
| 7 | `eSheetC` (ANSI C) |
| 8 | `eSheetD` (ANSI D) |
| 9 | `eSheetE` (ANSI E) |
| 10 | `eSheetLetter` |
| 11 | `eSheetLegal` |
| 12 | `eSheetTabloid` |
| 13 | `eSheetOrcadA` |
| 14 | `eSheetOrcadB` |
| 15 | `eSheetOrcadC` |
| 16 | `eSheetOrcadD` |
| 17 | `eSheetOrcadE` |

---

## 11. Text / Label Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `Text` | DynamicString or special | Label, NetLabel, Port, Parameter, TextFrame, Note | Simple objects: length-prefixed string. TextFrame/Note: 16-bit length prefix + ASCII+NUL |
| `TextColor` | u32 (BGR) | Port, TextFrame, Note, FunctionalBlock | |
| `TextFontID` | i16 | BasicEntry (SheetEntry, BusEntry) | Font table index (1-based) |
| `TextStyle` | DynamicString | BasicEntry | `"Full"` or `"Prefix"` (TBusTextStyle) |
| `TextMargin` | coord | TextFrame, Note | Margin between text and border. Default: TextFrame=5, Note=500000 |
| `TextHorzAnchor` | u8 | SheetFileName, SheetName, Parameter | TTextHorzAnchor |
| `TextVertAnchor` | u8 | SheetFileName, SheetName, Parameter | TTextVertAnchor |
| `WordWrap` | bool | TextFrame, Note | Default: true |
| `RTFStream` | binary blob | RichTextDocument | RTF formatted text as raw binary |
| `FileNameRTF` | DynamicString | RTFLink | External RTF file path |
| `UseMBCS` | bool | Document/Library header | Always `T` in V5 |
| `ShowOnlyFirstLine` | bool | HarnessLayoutLabel | |
| `AutoSize` | bool | Port | Auto-size to fit text content |
| `ShowName` | bool | Component child | Show component name |
| `ShowDesignator` | bool | Component child | Show component designator |

---

## 12. Electrical Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `Electrical` | u8 | Pin | TPinElectrical (see Pin Parameters section) |
| `IOType` | u8 | Port, SheetEntry | TPortIO |
| `NetTopology` | u8 | V4 binary only | TNetTopology |
| `Side` | u8 | SheetEntry, BusEntry | TLeftRightSide |
| `FormalType` | u8 | Pin | TStdLogicState (VHDL type) |
| `IsCrossSheetConnector` | bool | PowerObject | Power symbol acts as cross-sheet connector |
| `ShowNetName` | bool | PowerObject | Display net name on symbol |
| `PortNameIsHidden` | bool | Port | Inverted: `F` = show net name |
| `ShowBreakSymbol` | bool | HarnessBundle | Draw break/gap symbol |
| `IsLengthSetManually` | bool | HarnessBundle | |

### TPortIO Enum

| Value | Name |
|-------|------|
| 0 | `ePortUnspecified` |
| 1 | `ePortOutput` |
| 2 | `ePortInput` |
| 3 | `ePortBidirectional` |

### TLeftRightSide Enum

| Value | Name |
|-------|------|
| 0 | `eLeftSide` |
| 1 | `eRightSide` |
| 2 | `eTopSide` |
| 3 | `eBottomSide` |

---

## 13. Drawing / Shape Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `StartAngle` | 6-byte Pascal Real | Arc, EllipticalArc, Pie | Degrees (0.0..360.0). Uses Turbo Pascal 6-byte Real format, **not** IEEE-754. |
| `EndAngle` | 6-byte Pascal Real | Arc, EllipticalArc, Pie | |
| `StartLineShape` | u8 | Polyline | TLineShape |
| `EndLineShape` | u8 | Polyline | TLineShape |
| `LineShapeSize` | u8 | Polyline | TSize enum (endpoint shape size) |
| `ArrowKind` | DynamicString | SheetEntry | `"Block & Triangle"`, `"Triangle"`, `"Arrow"`, `"Arrow Tail"` |
| `IsSolid` | bool | Pie, Ellipse, TextFrame, Image | Filled interior |
| `Transparent` | bool | Ellipse | Interior is transparent |
| `KeepAspect` | bool | Image | Maintain aspect ratio |
| `ClipToRect` | bool | TextFrame, Note | Clip text to bounds. Default: true |
| `EmbedImage` | bool | Image | Image stored in `Storage` stream vs. linked by filename |
| `ScaleFactor` | coord | Symbol (IEEE) | Scale for IEEE symbol shapes |
| `Rotation` | u8 or i16 | Various | TRotationBy90 for harness objects; indexed `Rotation{N}` for display modes |
| `Orientation` | u8 | Most objects | TRotationBy90 (0=0deg, 1=90deg, 2=180deg, 3=270deg) |
| `Mirror` | bool | Symbol | |
| `IsMirrored` | bool | Label, NetLabel, Parameter, Component | Horizontal mirror |

### TLineShape Enum

| Value | Name |
|-------|------|
| 0 | `eLineShapeNone` |
| 1 | `eLineShapeArrow` (open arrowhead) |
| 2 | `eLineShapeSolidArrow` (filled arrowhead) |
| 3 | `eLineShapeTail` (open tail) |
| 4 | `eLineShapeSolidTail` (filled tail) |
| 5 | `eLineShapeCircle` |
| 6 | `eLineShapeSquare` |

---

## 14. Model / Footprint Parameters

These live on **Implementation** records (children of a Component). Each component has an ImplementationMap containing zero or more Implementations.

| Parameter | Type | Notes |
|-----------|------|-------|
| `ModelType` | string | `"PCBLIB"`, `"SIM"`, `"PCB3DLib"`, `"PCADLib"`, `"SI"`, `"VHD"`, `"SCHLIB"`, `"SCH"`, `"Datasheet"`, `"HarnessWiring"`, `"HarnessLayout"` |
| `ModelName` | string | Footprint/model name within the library |
| `DatafileCount` | i16 | Number of ModelDatafile triplets |
| `ModelDatafile{N}` | DynamicString | File path or library identifier for Nth datafile |
| `ModelDatafileEntity{N}` | DynamicString | Entity name within the datafile. Falls back to ModelName if empty. |
| `ModelDatafileKind{N}` | DynamicString | Kind/type of the datafile (e.g. `"PCBLib"`) |
| `ModelLocation` | DynamicString | Legacy alternative to DatafileCount pattern |
| `IntegratedModel` | bool | Model comes from an integrated library (.IntLib) |
| `DatabaseModel` | bool | Model comes from a database library |
| `ModelItemGUID` | DynamicString | Vault item GUID |
| `ModelRevisionGUID` | DynamicString | Vault revision GUID |
| `ModelVaultGUID` | DynamicString | Vault instance GUID |
| `Footprint` | string | V4 format only: `"Footprint0"`..`"Footprint3"` (4 slots). Migrated to Implementation on load. |

---

## 15. Locking / Visibility Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `Locked` | bool | Junction | Cannot be moved/deleted |
| `GraphicallyLocked` | bool | All GraphicalObjects | **Always reset to false on import** (deprecated) |
| `DesignatorLocked` | bool | Component, Harness objects | Designator text cannot be changed |
| `PartIDLocked` | bool | Component | Part ID locked. Defaults to DesignatorLocked if absent. |
| `DatabaseDatalinksLocked` | bool | Implementation | Collapsed into UseComponentLibrary on import |
| `DatalinksLocked` | bool | Implementation | Collapsed into UseComponentLibrary on import |
| `IsHidden` | bool | SheetFileName, SheetName, Parameter | Not rendered. For Pin, use bit 2 of PinConglomerate. |
| `IsNotAccesible` | bool | All DataObjects | Stored **inverted**: true = NOT accessible/selectable |
| `ReadOnlyState` | u8 | Parameter | TParameter_ReadOnlyState |
| `SelectionMemory` | u8 | All GraphicalObjects | 8-bit bitmask for selection memory sets |
| `IgnoreOnLoad` | bool | All DataObjects | Skip during load. Only written when true. |

### TParameter_ReadOnlyState Enum

| Value | Name |
|-------|------|
| 0 | `eReadOnly_None` (fully editable) |
| 1 | `eReadOnly_Name` |
| 2 | `eReadOnly_Value` |
| 3 | `eReadOnly_NameAndValue` |

---

## 16. Database / Vault Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `VaultGUID` | DynamicString | Component, Implementation | Vault server GUID |
| `ItemGUID` | DynamicString | Component, Implementation | Item in vault |
| `ItemRevisionGUID` | DynamicString | Component, Implementation | Specific revision |
| `RevisionGUID` | DynamicString | Various | Same as ItemRevisionGUID for some object types |
| `DesignItemId` | DynamicString | Component, SheetSymbol, ObjectDefinition | Human-readable vault/DB item identifier |
| `LifeCycleDefinitionGUID` | DynamicString | Library objects | Lifecycle definition GUID |
| `RevisionNamingSchemeGUID` | DynamicString | Library objects | Naming scheme GUID |
| `SourceLibraryName` | DynamicString | Component, SheetSymbol | Source SchLib name for library sync |
| `DatabaseModel` | bool | Implementation | Model from database library |
| `DatabaseTableName` | DynamicString | Component, ObjectDefinition | DB table name in DbLib |
| `NotAllowDatabaseSynchronize` | bool | Parameter | Inverted: excludes from DB sync |
| `NotAllowLibrarySynchronize` | bool | Parameter | Inverted: excludes from library sync |
| `NotUseDBTableName` | bool | Component, ObjectDefinition | Inverted: don't use stored DB table name |
| `NotUseLibraryName` | bool | Component, ObjectDefinition | Inverted: don't use stored library name |
| `UseComponentLibrary` | bool | Implementation | Linked to component's library. Also written as DatalinksLocked and DatabaseDatalinksLocked for backward compat. |

---

## 17. Harness Connectivity Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `HarnessConnectorSide` | u8 | HarnessConnector | TLeftRightSide |
| `HarnessType` | DynamicString | Port | Harness type name |
| `ConnectedObjectUniqueId` | DynamicString | HarnessSplice, HarnessWireLabel, HarnessWireBreak, HarnessPin | UniqueID of connected object |
| `EndVertex1ConnectedObjectUniqueID` | DynamicString | HarnessWire, HarnessWireData, HarnessBundle | Wire/bundle start endpoint |
| `EndVertex2ConnectedObjectUniqueID` | DynamicString | HarnessWire, HarnessWireData, HarnessBundle | Wire/bundle end endpoint |
| `PrimaryConnectionPosition` | coord | HarnessConnector | Wire attach point within connector |
| `WiringDiagramOriginUniqueId` | DynamicString | HarnessPin (layout) | Corresponding wiring-diagram pin UniqueID |

### Connected-List Parameters (Count + Per-Item UniqueIDs)

All use `ExportConnectedObjectsUniqueIds(ids, serializer, countKey, itemKey)`.

| Count Parameter | Item Parameter | Objects |
|-----------------|----------------|---------|
| `ConnectedWiresUniqueIdsCount` | `ConnectedWireUniqueId` | HarnessSplice, HarnessPin, HarnessShield, HarnessTwist, HarnessNoConnect, and their Data variants |
| `ConnectedPinWiresUniqueIdsCount` | `ConnectedPinWireUniqueId` | HarnessShield, HarnessShieldData |
| `ConnectedBundlesUniqueIdsCount` | `ConnectedBundleUniqueId` | HarnessLayoutConnectionPoint |
| `ConnectedInlineSplicesUniqueIdsCount` | `ConnectedInlineSpliceUniqueId` | HarnessWire, HarnessWireData |
| `ConnectedWireLabelsUniqueIdsCount` | `ConnectedWireLabelUniqueId` | HarnessWire |
| `ConnectedShieldsUniqueIdsCount` | `ConnectedShieldUniqueId` | HarnessWire, HarnessWireData |
| `ConnectedTwistsUniqueIdsCount` | `ConnectedTwistUniqueId` | HarnessWire, HarnessWireData |
| `ConnectedCablesUniqueIdsCount` | `ConnectedCableUniqueId` | HarnessWire, HarnessWireData |
| `BundlesToGoThroughUniqueIdsCount` | `BundleToGoThroughUniqueId` | (routing through bundles) |

---

## 18. Harness Physical Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `PhysicalStartDistance` | i64 | HarnessCovering | Physical length from bundle start to covering start (harness units) |
| `PhysicalEndDistance` | i64 | HarnessCovering | Physical length from bundle end to covering end |
| `PhysicalLength` | i64 | HarnessCovering | Actual physical length of covering |
| `Thickness` | u8 | HarnessCovering | Visual thickness (clamped by `CoveringThicknessClamper`) |
| `StartPointDistance` | i32 | HarnessCovering | Visual offset from start (DXP display coords) |
| `EndPointDistance` | i32 | HarnessCovering | Visual offset from end (DXP display coords) |
| `HarnessLayoutCoveringBrush` | u8 | HarnessCovering | THarnessBrush (visual fill pattern). File key: `"HarnessLayoutBraidBrush"` |
| `HarnessLengthUnit` | enum | HarnessDocument | THarnessLengthUnit. Default: `eMillimeter` |
| `LengthType` | u8 | HarnessBundleSubLineData | THarnessWireLengthType |
| `LengthLong` | i64 | HarnessBundle/SubLine | Physical length (harness units) |
| `LengthOffset` | i64 | HarnessBundleSubLineData | Offset added to calculated length |
| `DrawnLength` | i64 | HarnessBundle/SubLine | Graphically drawn length |
| `UserLength` | i64 | HarnessBundleSubLineData | User-specified length override |

### THarnessLengthUnit Enum

| Name | Suffix |
|------|--------|
| `eMillimeter` | `"mm"` |
| `eCentimeter` | `"cm"` |
| `eMeter` | `"m"` |
| `eInch` | `"in"` |
| `eFoot` | `"ft"` |

### THarnessWireLengthType Enum

| Value | Name |
|-------|------|
| 0 | `eCalculated` |
| 1 | `eUserDefined` |
| 2 | `eMCADCoDesigner` |

### Covering Covered Items

Serialized inline in the covering record:

```
CoveredItemsCount = N
for i in [0..N):
    CoveredItemType{i}     u8 (TObjectId: eHarnessBundle or eHarnessComponent)
    CoveredItemId{i}       DynamicString (UniqueId)
    if type == eHarnessComponent:
        CoveredItemFirstPin{i}   DynamicString (designator)
        CoveredItemLastPin{i}    DynamicString (designator)
```

---

## 19. Harness Color Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `Color` | u32 (BGR) | HarnessWire, HarnessWireBreak | Primary color |
| `SecondaryColor` | u32 (BGR) | HarnessWire, HarnessWireBreak | Default: `0xFFFFFFFF` (white/absent) |
| `TertiaryColor` | u32 (BGR) | HarnessWire, HarnessWireBreak | |
| `BorderColor` | u32 (BGR) | HarnessWire, HarnessWireBreak | Wire outline/border color |
| `PrimaryColorName` | DynamicString | HarnessWireBreak only | Human-readable name (e.g. `"Red"`) |
| `SecondaryColorName` | DynamicString | HarnessWireBreak only | |
| `TertiaryColorName` | DynamicString | HarnessWireBreak only | |
| `BorderColorName` | DynamicString | HarnessWireBreak only | |

---

## 20. Reuse Block Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `ReuseBlockId` | DynamicString | ReuseBlockImplementationInfo | GUID of reuse block definition |
| `ReuseBlockObjectsIds` | DynamicString | SchematicBlock, ReuseSheetSymbol | Pipe-delimited UniqueIDs of member objects |
| `IsDissolved` | bool | ReuseBlockImplementationInfo | Block has been dissolved (de-linked) |
| `IsSchematicBlockObject` | bool | Pin | Pin belongs to a schematic block (suppresses DRC) |
| `BlockServerName` | DynamicString | SchematicBlock | |
| `BlockVaultGUID` | DynamicString | SchematicBlock | |
| `BlockItemGUID` | DynamicString | SchematicBlock | |
| `BlockItemRevisionGUID` | DynamicString | SchematicBlock | |
| `SchSnippetVaultGUID` | DynamicString | ReuseBlocks stream + records | Schematic snippet vault reference |
| `SchSnippetItemGUID` | DynamicString | ReuseBlocks stream + records | |
| `SchSnippetItemRevisionGUID` | DynamicString | ReuseBlocks stream + records | |
| `PcbSnippetVaultGUID` | DynamicString | ReuseBlocksV2 stream + records | PCB snippet vault reference (V2 addition) |
| `PcbSnippetItemGUID` | DynamicString | ReuseBlocksV2 stream + records | |
| `PcbSnippetItemRevisionGUID` | DynamicString | ReuseBlocksV2 stream + records | |
| `RBServerParametersCount` / `RBServerParametersName` | count + strings | SchematicBlock | Workspace server parameter names |
| `PowerObjectsNameMappingsCount` | i32 | SchematicBlock, ReuseSheetSymbol | Count of power net name remappings |
| `PowerObjectsNameOriginal` | DynamicString | SchematicBlock, ReuseSheetSymbol | Original power net name |
| `PowerObjectsNameMapped` | DynamicString | SchematicBlock, ReuseSheetSymbol | Instance-specific mapped name |
| `ParametersCount` / `ParameterName` / `ParameterValue` | count + pairs | ReuseBlockImplementationInfo (dissolved) | Parameters captured at dissolution time |

---

## 21. Cross-Reference / Document Parameters

| Parameter | Type | Objects | Notes |
|-----------|------|---------|-------|
| `TargetFileName` | DynamicString | Component, ObjectDefinition | Target file for sheet parts / definitions |
| `TemplateFileName` | string | Document (sheet header) | Path to sheet template (`.SchDot`) |
| `SheetPartFileName` | DynamicString | Component | Internal schematic file in hierarchical design |
| `DocNum` | DynamicString | V4: file header; V5: system parameter | Document number (title block) |
| `SheetNum` | i16 | V4 only | Current sheet number (1-based) |
| `SheetCount` | i16 | V4 only | Total sheet count |
| `IndexInSheet` | i32 | All objects | Sequential index for ownership reconstruction. Default: -1 |
| `InstanceLabel` | DynamicString | FunctionalConnectionLine (ESD) | Connection instance label |
| `FilePosition` | i32 | V4 binary library only | Byte offset to component data in file |

---

## 22. Special / Internal Parameters

| Parameter | Type | Notes |
|-----------|------|-------|
| `BINARY` | instruction byte (208 / 0xD0) | Not a key-value parameter. Special serializer instruction that switches to binary mode. |
| `HEADER` | string | First parameter in almost every stream. Contains format identification string. |
| `RECORD` | instruction byte | Object type identifier (see Record Types section). |
| `RECORDEX` | i32 | Extended record type when RECORD == 254. |
| `EXTRALOCATIONCOUNT` | i16 | Vertex overflow count (vertices beyond 50). |
| `FileVersionInfo` | DynamicString | Pipe-delimited feature flag string for compatibility checks. |
| `DefaultCrossRefHidden` | bool | Global default: cross-reference annotations hidden. |
| `ConnectionPairsToSuppress` | string | NoERC: serialized connection pair suppressions. Only when SuppressAll=false. |
| `ErrorKindSetToSuppress` | bitmask | NoERC: comma-separated error kind names as TErrorKindSet. Only when SuppressAll=false. |
| `SuppressAll` | bool | NoERC: suppress all error kinds at this location. |
| `IsImageParameter` | bool | Parameter: value is an image reference, not text. |
| `ObjectDefinitionId` | DynamicString | Port, PowerPort, ObjectDefinition: links to ObjectDefinitions stream. |
| `ObjectDefinitionHash` | DynamicString | ObjectDefinition: content hash for change detection. |
| `AssociatedObjectType` | u8 | AssociatedObjects (harness): 0=Crimp, 1=Seal, 2=Plug, 3=Other |
| `FileHash` | DynamicString | FileObject: MD5 or similar hash of embedded file content. |
| `UniqueID` | string | Document-wide unique identifier. |
| `UniqueIDInReuseBlock` | string | Object's UniqueID within a reuse block context. |

---

## 23. Data Version Constants

Written as the first 4 bytes (LE i32) of each binary-blob stream payload. Checked on import; blobs with version > max are rejected.

| Constant | Value | Stream | Check |
|----------|-------|--------|-------|
| `DataVersionReuseBlocksVersion` | 2 | `ReuseBlocks`, `ReuseBlocksV2` | Rejects > 2. Version 1 uses 4-byte length prefix strings; version 2 uses .NET `BinaryWriter.Write(string)` |
| `DataVersionHarnessConnectionPointConnectorDataVersion` | 1 | `HarnessConnectionPointConnector` | Rejects > 1 |
| `DataVersionHarnessComponentCrimpsVersion` | 1 | `HarnessComponentCrimps` | Declared but no .NET reader found (may be Delphi-side) |
| `DataVersionHarnessAssociatedPartsVersion` | 1 | `HarnessAssociatedParts` | Declared but no .NET reader found (may be Delphi-side) |

---

## 24. Unit System

```csharp
public const TUnitSystem DefaultComponentUnitSystem = TUnitSystem.eMetric;
```

### TUnitSystem Enum

| Value | Name |
|-------|------|
| 0 | `eImperial` (mils) |
| 1 | `eMetric` (millimeters) |

The unit system controls default values for newly-created objects via `SetDefault(TUnitSystem)`:

| Default | eImperial (DXP units) | eMetric (DXP units) |
|---------|----------------------|---------------------|
| DefaultPinNameMargin | 500,000 (~5 mils) | 98,425 (~2.5mm) |
| DefaultPinNumberMargin | 800,000 (~8 mils) | 196,850 (~5mm) |
| DefaultCustomSizeX_Sheet | 150,000,000 (~1500 mils) | 118,110,240 (~3000mm) |
| DefaultCustomSizeY_Sheet | 95,000,000 (~950 mils) | 78,740,160 (~2000mm) |
| DefaultPortWidth | 5,000,000 (~50 mils) | 3,937,008 (~100mm) |

The document's display unit is persisted as `Display_Unit` (TUnit enum). On import, the unit system is derived from the display unit.

---

## 25. Wire Type Summary

How parameter types map to on-disk encoding:

| Parameter Type | Rust Equivalent | On-Disk Encoding |
|---------------|----------------|------------------|
| `bool` | `bool` | Single byte: `T` (0x54) or `F` (0x46) |
| `byte` (u8) | `u8` | Raw byte |
| `short int` (i16) | `i16` | Little-endian 16-bit signed |
| `long int` (i32) | `i32` | Little-endian 32-bit signed |
| `long` (i64) | `i64` | Little-endian 64-bit signed (harness lengths) |
| `string` | length-prefixed, max 255 bytes | 1-byte length then ANSI/MBCS bytes |
| `DynamicString` | same as string | With MBCS, same encoding |
| `Color` (u32) | `u32` | Windows COLORREF: `0x00BBGGRR` |
| `Coord` (i32) | `i32` | DXP coordinate units (1 unit = 10 nm) |
| `Angle` | 6-byte | Borland Turbo Pascal 6-byte floating point (NOT IEEE-754) |
| `Binary` | length-prefixed blob | 2-byte length (i16) then raw bytes |
| `Text` | 2-byte length + ASCII | 16-bit length prefix, null-terminated ASCII |
