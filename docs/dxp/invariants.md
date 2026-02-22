# Altium File Serialization Invariants & Ordering

How Altium maintains deterministic output when saving/loading documents: parameter
ordering within records, record ordering within streams, stream ordering within OLE
containers, and all invariants that must be preserved for round-trip stability.

Sources: `FileFormatV5.cs`, `SchDataExporterBaseV5.cs`, `SchDataContainer.cs`,
`SchDataObjectComparator.cs`, `SchDataDocument.cs`, `SchDataImporterExporterUtils.cs`,
`ParameterUtils.cs` (all in AD26-dotnet/Altium.Sch.DataModel/), plus Ghidra analysis of
`Altium.PCB.BinaryLoader.dll` (`RegisterAllSectionsForExporting` at `0x01918020`).

---

## 1. Parameter Order Within Records (Schematic)

Parameter order is **explicitly hard-coded per record type** in `FileFormatV5.cs` (5575
lines). It is NOT alphabetical, NOT Delphi RTTI-driven, NOT property-declaration-order.

Each `Export_*` call on the `ISchDataSerializer` appends a `|KEY=VALUE|` pair in call
order. The serializer accumulates pairs sequentially, and `ExportToSingleString()` emits
them in that exact order.

### Common Prefix: ExportDataObject

All objects inherit from this base export, which writes:

```
OwnerIndex → IsNotAccesible → OwnerIndexAdditionalList → IndexInSheet →
[IgnoreOnLoad] → [WiringDiagramOriginUniqueId] → IsSchematicBlockObject →
[UniqueIDInReuseBlock]
```

### Common Prefix: ExportGraphicalObject (extends ExportDataObject)

Most visible objects use this, which appends after ExportDataObject:

```
OwnerPartId → OwnerPartDisplayMode → SelectionMemory → UnionIndex →
GraphicallyLocked
```

### Per-Record Field Order

#### Pin (RECORD=2) — custom order, does NOT call ExportGraphicalObject

```
OwnerIndex → OwnerPartId → OwnerPartDisplayMode → SymBol_InnerEdge →
SymBol_OuterEdge → SymBol_Inner → SymBol_Outer → Description → FormalType →
Electrical → PinConglomerate → PinLength → Location.X → Location.Y → Color →
Name → Designator → SwapIdPin → SwapIDPart → DefaultValue → SwapIdPair →
[conditional position fields] → SymBol_LineWidth → PinPackageLength →
PinPropagationDelay → UniqueID → HidePinNameAsFunction → PinSelectedFunctions →
PinDefinedFunctions → PinSymbolicName → ShowPinSymbolicNameAsFunction
```

#### Arc (RECORD=12) — ExportGraphicalObject then:

```
Location.X → Location.Y → Radius → LineWidth → StartAngle → EndAngle →
Color → UniqueID
```

#### Wire (RECORD=27) — ExportGraphicalObject then:

```
LineWidth → Color → UnderlineColor → UniqueID → AssignedInterface →
AssignedInterfaceSignal → Vertices
```

#### Component (RECORD=1) — mixed:

```
LibReference → ComponentDescription → PartCount → DisplayModeCount →
[ExportGraphicalObject] → Location.X → Location.Y → DisplayMode → IsMirrored →
Orientation → CurrentPartId → ShowHiddenFields → ShowHiddenPins → LibraryPath →
SourceLibraryName → DatabaseTableName → SheetPartFileName → TargetFileName →
UniqueID → AreaColor → Color → PinColor → OverideColors → DisplayFieldNames →
DesignatorLocked → PartIDLocked → PinsMoveable → AliasList → NotUseLibraryName →
NotUseDBTableName → DesignItemId → VaultGUID → ItemGUID → RevisionGUID →
SymbolVaultGUID → SymbolItemGUID → SymbolRevisionGUID →
GenericComponentTemplateGUID → HasOnlyCurrentPartInfo → AllPinCount →
KeyComponentUniqueId → ComponentKind → CustomDisplayModeName*
```

#### Sheet/Document Header (RECORD=31):

```
[ExportStyleAndFontTable] → UseMBCS → IsBOC → HotSpotGridOn → HotSpotGridSize →
SheetStyle → SystemFont → DocumentBorderStyle → WorkspaceOrientation → BorderOn →
TitleBlockOn → SheetNumberSpaceSize → Color → AreaColor → SnapGridOn →
SnapGridSize → VisibleGridOn → VisibleGridSize → CustomX → CustomY →
UseCustomSheet → ShowHiddenPins → ReferenceZonesOn → CustomXZones →
CustomYZones → CustomMarginWidth → ShowTemplateGraphics → TemplateFileName →
...VaultGUIDs... → Display_Unit → ReferenceZoneStyle → FileVersionInfo
```

### Dispatch Mechanism

`FileFormatBase.ExportToFileByObjectId` is a giant switch on `TObjectId` that dispatches
each object to its type-specific export method in `FileFormatV5`.

---

## 2. Parameter Encoding Invariants

| Rule | Detail |
|---|---|
| Delimiter | `\|` between key=value pairs |
| Key=Value separator | `=` |
| Escaping in values | `[]` decodes as `\|`, `{}` decodes as `=` |
| Text encoding | Windows-1252 by default |
| Unicode keys | Prefixed with `%UTF8%` (e.g. `%UTF8%DESCRIPTION=...`) |
| Booleans | `T`/`F` (short form) or `TRUE`/`FALSE` (long form) |
| Case sensitivity | Keys are **case-insensitive** during parsing (first occurrence wins) |
| Fractional coords | Two params: `LOCATION.X=100` + `LOCATION.X_FRAC=5000` |
| Indexed coords | `LOCATIONCOUNT=N` then `X1`, `Y1`, `X2`, `Y2`, ... (1-based) |
| Nesting | Level 0 uses `\|` delimiter; level 1 uses backtick (`` ` ``) |
| Extended records | RECORD >= 256 written as `RECORD=254` + `RECORDEX=<actual_value>` |
| NUL terminator | Each record string is followed by a NUL byte |
| Duplicate keys | First occurrence wins in the parser's case-insensitive Dictionary |

---

## 3. Record Ordering Within Streams (Schematic)

### Save Pipeline

From `SchDataExporterBaseV5.Run()`:

```
 1. InitializeForSaving()              — update font IDs
 2. FillBaseAndAdditionalWarehouses()   — collect records into flat list
 3. FillExtendedWarehouse()             — embedded images → "Storage" stream
 4. FillDefinitionWarehouse()           — definitions
 5. FillReuseBlockInfoWarehouse()       — reuse blocks
 6. FillFilesWarehouse()                — attached files
 7. FixBaseWarehouse()                  — post-processing fixups
 8. WriteBaseWarehouse()                — → "FileHeader" stream
 9. WriteExtendedWarehouse()            — → "Storage" stream
10. WriteAdditionalWarehouse()          — → "Additional" stream
11. WriteDefinitionWarehouse()          — → custom streams
12. WriteReuseBlockInfoWarehouse()      — → custom streams
13. WriteFilesWarehouseData()           — → file streams
14. FinalizeForSaving()
```

### How Records Fill the Warehouse

Records are collected via **depth-first tree traversal** with child sorting.

`SchDataContainer.AddToListForSave()` (line 265):

```
1. Add self to the base warehouse list
2. Record OwnerIndex = current position in the flat list
3. Collect all first-level children via FillChildrenListForSave()
   (excludes objects with IgnoreOnSave flag)
4. Sort children using SchDataObjectComparator
5. For each sorted child:
   - If child's ObjectID is in AdditionalObjectObjectIdSet → additional warehouse
   - Otherwise → recurse AddToListForSave (depth-first)
```

### The Sorting Comparator

`SchDataObjectComparator.Compare()`:

```csharp
int codeX = SchDataUtils.GetBinaryCodeForObject(x);
int codeY = SchDataUtils.GetBinaryCodeForObject(y);
if (codeX > 225 || codeY > 225) {
    return codeX - codeY;  // Extended records: sort by RECORD type ascending
}
return x.GetOwnerIndexForSave() - y.GetOwnerIndexForSave();  // Standard: insertion order
```

**Rules:**
- Standard records (RECORD <= 225): **preserve insertion order** (stable by OwnerIndex)
- Extended records (RECORD > 225, e.g. Hyperlink=226, RTFLink=241): **sort ascending by
  record type**
- If one child is standard and the other is extended, the extended sorts by its code vs.
  the standard's owner index — meaning extended records generally sort to the end

### Pre-Save Mutations

1. **`MoveSpecialObjectsToTop()`** — despite the name, moves 3 specific object types to
   the **end** of the document's child list before the sort step runs. The exact object
   IDs are loaded from a compiled array via `RuntimeHelpers.InitializeArray`.

2. **Component implementation extraction** — before `AddToListForSave`, components extract
   `Implementation` objects into a temporary `SchDataImplementationList` that gets its own
   entry in the base warehouse after the component's children.

3. **`AddAutoJunctions()`** — after `FillBaseAndAdditionalWarehouses`, auto-junction
   objects (`eJunction` type) are created for wire intersections with 3+ connections and
   appended at the **very end** of the base warehouse.

### WriteBaseWarehouse Format

```
Block 0: RECORD=0 header with HEADER=..., Weight=<total_record_count>,
         MinorVersion=..., UniqueID=...
Block 1..N: Each record written as:
  - RECORD=<byte> (or RECORD=254 + RECORDEX=<int> for extended)
  - Followed by all parameters from ExportToFile in the hard-coded order
  - NUL terminated
```

---

## 4. PCB Binary Format Stream Ordering

PCB files use **little-endian binary structs**, not pipe-delimited text. Each primitive
type has its own CFB storage with `Header` (u32 record count) and `Data` (packed records)
sub-streams.

### Primary Sections (Fixed Order)

From `TPCBBinaryFile::RegisterAllSectionsForExporting` in `Altium.PCB.BinaryLoader.dll`:

| # | CFB Storage Name | Content |
|---|---|---|
| 1 | Board6 | Board configuration |
| 2 | ECO Options6 | ECO options |
| 3 | Output Options6 | Output options |
| 4 | Printer Options6 | Printer options |
| 5 | Gerber Options6 | Gerber options |
| 6 | Advanced Placer Options6 | Placer options |
| 7 | DRC Options6 | Design rule checker options |
| 8 | Classes6 | Net/component classes |
| 9 | Nets6 | Net definitions |
| 10 | Components6 | Component records |
| 11 | Polygons6 | Polygon pours |
| 12 | Dimensions6 | Dimension annotations |
| 13 | Coordinates6 | Coordinate markers |
| 14 | Connections6 | Ratsnest connections |
| 15 | Rules6 | Design rules |
| 16 | FromTos6 | From-to definitions |
| 17 | Embeddeds6 | Embedded objects |
| 18 | Arcs6 | Arc primitives |
| 19 | Pads6 | Pad primitives |
| 20 | Vias6 | Via primitives |
| 21 | Tracks6 | Track primitives |
| 22 | Texts6 | Text primitives |
| 23 | Fills6 | Fill primitives |

Sections 1-7 are configuration/options. Sections 8-17 are structural data. Sections
18-23 are graphical primitives.

### PCB Binary Record Format

Each record within a section's `Data` stream:

```
[u8 object_id][struct fields in fixed little-endian layout]
```

Field order is determined by the binary struct layout — fixed and immutable. Coordinates
are i32 (10,000 units = 1 mil). All records within a section are packed contiguously with
no padding between records (size-prefixed blocks wrap each record).

### PCB Sidecar Streams (After All Primary Sections)

| Stream | Format | Purpose |
|---|---|---|
| WideStrings6 | Binary TLV | Unicode text for primitive fields |
| UniqueIDPrimitiveInformation | Parameter blocks | Per-primitive identity strings |
| ExtendedPrimitiveInformation | Parameter blocks | Mask expansion overrides |
| PrimitiveGuids | 24-byte records | Persistent GUIDs per primitive |

#### WideStrings6 TLV Types

| Type byte | Length encoding | Payload |
|---|---|---|
| `0x06` | u8 length | ASCII |
| `0x0C` | u32 LE length | ASCII |
| `0x12` | u32 LE char count | UTF-16LE |
| `0x14` | u32 LE byte count | UTF-8 |

### PCB Parameter Serialization

PCB primitives also support parameter-based serialization via the COM interface
`IPCB_PrimitiveSerialize` (Delphi side in `Advpcb.dll`):

```csharp
void ExportToParameters(IWideParameterList argParameters);
void ImportFromParameters(IWideParameterList argParameters);
```

The parameter order from `ExportToParameters` is hard-coded in the Delphi implementation
of each primitive. This is used for non-binary contexts (clipboard, scripting, etc.) but
not for the primary `.PcbDoc` binary file format.

---

## 5. PcbLib Per-Footprint Stream Ordering

Each footprint `/<PatternName>/` contains streams in this order:

| Stream | Format | Purpose |
|---|---|---|
| `Data` | Packed binary records | Main footprint primitives |
| `WideStrings` | **Parameter-block format** | Unicode text (NOT binary TLV like PcbDoc!) |
| `PrimitiveGuids/{Header,Data}` | 24-byte records | Persistent GUIDs |
| `UniqueIDPrimitiveInformation/{Header,Data}` | Parameter-block table | Unique IDs |
| `ExtendedPrimitiveInformation/{Header,Data}` | Parameter-block table | Mask expansion |

**Important difference from PcbDoc:** PcbLib `WideStrings` uses parameter-block format,
NOT the binary TLV encoding used by PcbDoc's `WideStrings6`.

---

## 6. SchDoc Extended Stream Ordering

Global streams written after `FileHeader`:

| # | Stream | Purpose |
|---|---|---|
| 1 | `/Storage` | Embedded images (SchDataEmbeddedObject format) |
| 2 | `/ReuseBlocks` | Reuse block vault references (V1) |
| 3 | `/ReuseBlocksV2` | PCB snippet references (extends V1) |
| 4 | `/HarnessConnectionPointConnector` | Harness connector mappings |
| 5 | `/Additional` | Overflow/additional objects |
| 6 | `/ObjectDefinitions` | Object definition records |
| 7 | `/ReuseBlockInfos` | Dissolved reuse block info |

---

## 7. SchLib Sidecar Stream Ordering (Per-Component)

Each component `/<SectionKey>/` has 9 sidecar streams loaded/saved in this exact order:

| # | Stream | Purpose | Key invariant |
|---|---|---|---|
| 1 | `PinFrac` | Fractional coordinates (3 doubles: x, y, length) | |
| 2 | `PinDesc` | Long descriptions (>254 chars, ASCII) | Overwritten by PinWideText |
| 3 | `PinMiscData` | Swap ID pair data | |
| 4 | `PinTextData` | Custom text display settings (binary) | |
| 5 | `PinWideText` | Unicode text | **Authoritative** — fully replaces PinDesc |
| 6 | `PinSymbolLineWidth` | Symbol line width | |
| 7 | `PinPackageLength` | Pin package length | |
| 8 | `PinPropagationDelay` | Signal propagation delay | |
| 9 | `PinFunctionData` | Alternate pin functions | |

**Critical ordering dependency:** PinWideText (stream 5) is the authoritative source for
text data. When present, it **fully replaces** fields that PinDesc (stream 2) set. During
save, both must be written; during load, PinWideText must be applied after PinDesc.

All sidecar streams are optional — existence is checked with `StreamExists()` before
opening.

---

## 8. SchLib Component Indexing Invariants

- `FileHeader` contains a library header + font table + component index
- Each component has its own sub-storage with a `/Data` stream
- Within each component's Data stream:
  - First record: Component object (RECORD=1)
  - Following records: All child objects
- **OWNERINDEX values are relative to the component start**, not absolute
- During load, these are adjusted: `ownerindex += component_base_offset`
- During save, the reverse adjustment must be applied

---

## 9. TObjectId to RECORD Byte Mapping

From `SchDataUtils.GetBinaryCodeByObjectId()`:

| TObjectId | RECORD | TObjectId | RECORD |
|---|---|---|---|
| eSchComponent | 1 | ePin | 2 |
| eSymbol | 3 | eLabel | 4 |
| eBezier | 5 | ePolyline | 6 |
| ePolygon | 7 | eArc | 12 |
| eSheetSymbol | 15 | eSheetEntry | 16 |
| ePowerObject / eCrossSheetConnector | 17 | ePort | 18 |
| eNoERC | 22 | eErrorMarker | 23 |
| eNetLabel | 25 | eBus | 26 |
| eWire | 27 | eSheet | 31 |
| eSheetName | 32 | eSheetFileName | 33 |
| eDesignator | 34 | eTemplate | 39 |
| eTaskHolder | 40 | eParameter / eImageParameter | 41 |
| eParameterSet | 43 | eImplementationsList | 44 |
| eImplementation | 45 | eImplementationMap | 46 |
| eMapDefiner | 47 | eParameterList | 48 |
| eSchLib | 200 | eSignalHarness | 218 |
| eBlanket | 225 | eHyperlink | 226 |
| eRTFLink | 241 | — | — |
| (RECORD >= 256) | Written as RECORD=254 + RECORDEX=value | | |

**Sorting boundary:** RECORD <= 225 = standard (preserve insertion order), RECORD > 225 =
extended (sort by type ascending).

---

## 10. Summary of All Invariants

| Invariant | Detail |
|---|---|
| **Parameter order** | Hard-coded per record type in `FileFormatV5.cs`. Never alphabetical. |
| **Record order** | Depth-first tree traversal, children sorted by `SchDataObjectComparator` |
| **Child sort rule** | RECORD <= 225: preserve insertion order. RECORD > 225: sort ascending by type. |
| **Auto-junctions** | Appended at the very end of the base warehouse |
| **Special objects** | 3 specific types moved to end of child list before sorting |
| **Component implementations** | Extracted into separate ImplementationList entry after children |
| **PCB stream order** | Fixed 23-section order from `RegisterAllSectionsForExporting` |
| **PCB sidecar order** | WideStrings6 → UniqueID → Extended → Guids |
| **PCB binary fields** | Fixed struct layout, immutable order |
| **SchLib sidecars** | 9 streams per component in fixed order; PinWideText is authoritative |
| **SchDoc extended streams** | Storage → ReuseBlocks → ReuseBlocksV2 → Harness → Additional → Definitions → ReuseBlockInfos |
| **RECORD >= 256** | Written as `RECORD=254` + `RECORDEX=<actual_value>` |
| **Block headers** | 4-byte `flags\|length` prefix mandatory on every block |
| **Key case** | Case-insensitive during parsing, first occurrence wins for duplicates |
| **Encoding** | Windows-1252; `%UTF8%` prefix for Unicode keys |
| **Value escaping** | `[]` → `\|`, `{}` → `=` within values |
| **NUL terminator** | Every record string ends with NUL |
| **PcbLib WideStrings** | Parameter-block format, NOT binary TLV (differs from PcbDoc) |
| **SchLib OWNERINDEX** | Relative to component start; adjusted to absolute during load |
| **Sidecar existence** | Optional; checked with `StreamExists()` before opening |
| **Font table** | Written as part of RECORD=31 (Sheet) via `ExportStyleAndFontTable` |
| **Weight field** | RECORD=0 header includes `Weight=<total_record_count>` |
