# Sidecar Streams Deep Dive

Reverse-engineered from Altium Designer 26 via decompiled .NET code
(`AD26-dotnet/`) and Ghidra analysis of Delphi DLLs (`altium26` project).

---

## 1. Architectural Overview

Altium stores supplementary primitive data in **sidecar streams** -- separate
OLE/CFB streams that extend the main record data. In Altium's runtime model
these streams are purely a **serialization concern**: on load the sidecar data
is merged into the primitive objects; on save it is split back out. The split
exists for backwards compatibility -- new fields were added via new streams so
older readers could ignore them without choking.

### Key principle

> There is **no distinction** in Altium's runtime model between "core record
> data" and "sidecar data." The primitive object has all fields directly. The
> sidecar streams are format-evolution artifacts.

---

## 2. PCB Sidecar Streams (PcbDoc / PcbLib)

### 2.1 Stream Registration

The BinaryLoader DLL registers 70+ streams. The sidecar-relevant ones:

| Index | Stream Name                        | Format     | Purpose                          |
|-------|------------------------------------|------------|----------------------------------|
| 29    | `WideStrings`                      | Binary TLV | Unicode string table             |
| 37    | `ExtendedPrimitiveInformation`     | ParamTable | Per-primitive property overrides |
| 38    | `ExtendedPrimitiveIndices`         | Binary     | Index into above                 |
| 43    | `UniqueIDPrimitiveInformation`     | ParamTable | Per-primitive unique IDs         |

Version-6 variants (`WideStrings6`, etc.) also exist for format version >= 6.

### 2.2 WideStrings -- Unicode Text for Primitives

**Source:** `Advpcb.dll` function at `0x548920` (Ghidra decompilation confirmed).

#### Binary TLV Encoding (PcbDoc board-level)

Each entry in the WideStrings stream uses a Type-Length-Value encoding:

| Type byte | Length field             | Data encoding    | Notes                              |
|-----------|-------------------------|------------------|------------------------------------|
| `0x06`    | 1 byte (u8)             | ASCII bytes      | Short ASCII strings (len <= 255)   |
| `0x0C`    | 4 bytes (u32 LE)        | ASCII bytes      | Long ASCII strings                 |
| `0x12`    | 4 bytes (u32 LE, chars) | UTF-16LE bytes   | Unicode, length is in chars not bytes |
| `0x14`    | 4 bytes (u32 LE)        | UTF-8 bytes      | Unicode, length is in bytes        |

**Decompiled read logic (pseudocode):**

```
fn read_wide_string(stream) -> String:
    type_byte = stream.read_byte()
    match type_byte:
        0x06 => len = stream.read_u8()
                data = stream.read_bytes(len)
                return decode_ascii(data)
        0x0C => len = stream.read_u32_le()
                data = stream.read_bytes(len)
                return decode_ascii(data)
        0x12 => char_count = stream.read_u32_le()
                data = stream.read_bytes(char_count * 2)
                return decode_utf16le(data)
        0x14 => byte_len = stream.read_u32_le()
                data = stream.read_bytes(byte_len)
                return decode_utf8(data)
        _    => error("unknown string type")
```

**Write optimization:** The writer prefers ASCII (`0x06`/`0x0C`) when possible,
falls back to UTF-8 (`0x14`) or UTF-16LE (`0x12`) depending on which is shorter.

Each primitive that has a text field (e.g. `PcbTextRecord`) references its wide
string by **zero-based index** into this table.

#### PcbLib Footprint-Level WideStrings

PcbLib footprint-level `WideStrings` streams use a **completely different format**:
parameter blocks (`ENCODEDTEXT0=...`) rather than the binary TLV format. The
`ENCODEDTEXT{N}` values are comma-separated integer sequences representing
encoded text.

Format:
```
[4-byte block header: flags(1) | length(3)]
[NUL-terminated parameter string]
  e.g. |ENCODEDTEXT0=65,66,67|ENCODEDTEXT1=...|
```

This is a sequence of parameter blocks, one per WideStrings entry.

### 2.3 UniqueIDPrimitiveInformation -- Primitive Identity

**Format:** Parameter-block table with `Header` (u32 count) + `Data` (parameter blocks).

Each entry contains:

| Key              | Type   | Purpose                                  |
|------------------|--------|------------------------------------------|
| `PRIMITIVEINDEX` | u32    | Zero-based index within the type stream  |
| `UNIQUEID`       | string | Unique identifier string                 |

**Load lifecycle:**
1. BinaryLoader reads `Header` stream to get entry count
2. BinaryLoader reads `Data` stream, parsing `count` parameter blocks
3. For each entry, looks up the primitive at `PRIMITIVEINDEX` in its type stream
4. Calls `primitive.SetState_UniqueID(value)` to set the ID

**Save lifecycle:**
1. Iterate all primitives that have a UniqueID set
2. For each, emit a parameter block with `PRIMITIVEINDEX` + `UNIQUEID`
3. Write `Header` with count, `Data` with serialized blocks

**Cross-document purpose:** UniqueIDs are used for schematic-to-PCB linking.
When ECO (Engineering Change Order) operations synchronize between SchDoc and
PcbDoc, the UniqueID is the identity key that maps schematic pins/components
to their PCB counterparts.

The `IPCB_Primitive` interface exposes:
- `GetState_UniqueId() -> string`
- `SetState_UniqueID(string)`

And `IPCB_Board2` exposes `GenerateUniqueID() -> string` for creating new IDs.

### 2.4 ExtendedPrimitiveInformation -- Property Overrides

**Format:** Parameter-block table with `Header` (u32 count) + `Data` (parameter blocks).
Works with `ExtendedPrimitiveIndices` for fast lookup.

Each entry contains per-primitive property overrides:

| Key                             | Type                  | Purpose                    |
|---------------------------------|-----------------------|----------------------------|
| `PRIMITIVEINDEX`                | u32                   | Primitive index            |
| `PASTEMASKEXPANSIONMODE`        | `TMaskExpansionMode`  | Paste mask mode            |
| `PASTEMASKEXPANSION_MANUAL`     | coord (i32)           | Manual paste expansion     |
| `SOLDERMASKEXPANSIONMODE`       | `TMaskExpansionMode`  | Solder mask mode           |
| `SOLDERMASKEXPANSION_MANUAL`    | coord (i32)           | Manual solder expansion    |

#### TMaskExpansionMode Enum

From `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TMaskExpansionMode.cs`:

```csharp
enum TMaskExpansionMode : byte {
    eMaskExpansionMode_NoMask  = 0,  // No mask expansion
    eMaskExpansionMode_Rule    = 1,  // Use design rule
    eMaskExpansionMode_Manual  = 2,  // Manual override value
}
```

Serialized as the string `"None"` / `"Rule"` / `"Manual"` in parameter blocks
(confirmed by `ListPair_Rules.cs` which searches for `"SOLDERMASKEXPANSIONMODE=None|PASTEMASKEXPANSIONMODE=None"`).

#### Property Resolution Chain

From `IPCB_Primitive2`, the mask expansion value a consumer sees depends on mode:

1. **`NoMask` (0)**: No expansion applied.
2. **`Rule` (1)**: Query matching `IPCB_SolderMaskExpansionRule` or
   `IPCB_PasteMaskExpansionRule` from the board design rules.
3. **`Manual` (2)**: Use the primitive's local expansion value directly.

The `MaskExpansion(TV7_Layer)` method computes the actual value for a given
layer, accounting for the mode and per-layer overrides.

#### Additional Paste Mask Properties (IPCB_Primitive2)

```csharp
bool   GetState_PasteMaskEnabled()
bool   GetState_PasteMaskUsePercent()
double GetState_PasteMaskPercent()
double GetState_PasteMaskManualPercent()
bool   GetState_PasteMaskManualEnabled()
Guid   GetGUID()
```

#### ExtendedPrimitiveIndices

The `ExtendedPrimitiveIndices` stream provides a lookup table for fast
random access into `ExtendedPrimitiveInformation`. This avoids linear
scanning when looking up a specific primitive's extended properties.

**Note:** The indices stream is an optimization. The `Data` stream itself
contains `PRIMITIVEINDEX` in each parameter block, so the information is
self-contained.

### 2.5 PrimitiveGuids (PcbLib only)

PcbLib footprints also have a `PrimitiveGuids/{Header,Data}` stream pair.

**Binary format per entry (24 bytes):**
```
[4 bytes] tag    (u32 LE) -- format-specific type tag
[4 bytes] index  (u32 LE) -- primitive index
[16 bytes] guid  (raw GUID bytes)
```

This stream assigns a persistent GUID to each primitive within a footprint,
separate from the UniqueID string mechanism. The GUIDs survive across
library updates and are used for `IPCB_Primitive2.GetGUID() / SetGUID()`.

### 2.6 Primitive Attribute Enum (TPrimitiveAttribute)

From `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/TPrimitiveAttribute.cs`.

Altium enumerates all primitive properties via a `TPrimitiveAttribute` enum
(533 values total). Mask-related indices:

| Index | Attribute                                |
|-------|------------------------------------------|
| 61    | `SolderMaskOverride`                     |
| 62    | `UseSeparateSolderMaskExpansion`          |
| 63    | `SolderMaskExpansion`                    |
| 64    | `SolderMaskExpansionTop`                 |
| 65    | `SolderMaskExpansionBottom`              |
| 66    | `SolderMaskExpansionMode`                |
| 67    | `SolderMaskExpansionFromHoleEdge`        |
| 70    | `PasteMaskOverride`                      |
| 71    | `PasteMaskEnabled`                       |
| 72    | `TopPasteMaskEnabled`                    |
| 73    | `BottomPasteMaskEnabled`                 |
| 74    | `PasteMaskExpansion`                     |
| 75    | `PasteMaskUsePercent`                    |
| 76    | `PasteMaskPercent`                       |
| 77    | `PasteMaskExpansionMode`                 |

This confirms that mask expansion data is an integral part of the primitive's
property set -- it just happens to be serialized into a separate stream.

---

## 3. Schematic Sidecar Streams (SchLib)

### 3.1 File Scope

**Critical finding:** Pin sidecar streams exist ONLY in **SchLib** files, NOT
in SchDoc files. SchDoc files store all pin data inline in the main `Data`
stream. This was confirmed by examining both `SchDataImporterDocumentV5` (SchDoc)
and `SchDataImporterLibraryV5` (SchLib) -- only the library importer calls
`ReadPinsExtendedData()`.

### 3.2 Stream Names

Defined as constants in
`AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs`:

| Stream Name             | Purpose                                        |
|-------------------------|------------------------------------------------|
| `PinFrac`               | Fractional coordinate parts for pins           |
| `PinDesc`               | Long descriptions (>254 chars, ASCII)          |
| `PinMiscData`           | Misc data (SwapID pairs)                       |
| `PinTextData`           | Custom text display settings per pin           |
| `PinWideText`           | Wide (Unicode) text for all pin string fields  |
| `PinSymbolLineWidth`    | Symbol line width per pin                      |
| `PinPackageLength`      | Package-specific pin length                    |
| `PinPropagationDelay`   | Signal propagation delay                       |
| `PinFunctionData`       | Pin function metadata                          |
| `Redirection`           | Alias resolution (component aliases)           |

### 3.3 Embedded Object Container Format

All sidecar streams use the same envelope (`SchDataEmbeddedObject`):

```
[header block] -- parameter string: RECORD=0|HEADER=<StreamType>|Weight=<count>|
[entry 0]      -- compressed tag 0xD0 + id (pin index as string) + binary data
[entry 1]      -- ...
[entry N]      -- ...
```

**Outer block structure (per block):**
```
[4 bytes] header: flags(1 byte, bits 31-24) | length(3 bytes, bits 23-0)
[N bytes] payload
```

- Header block: flags = `0x00`, payload = NUL-terminated parameter string
- Entry blocks: flags = `0x01`, payload = compressed object

**Compressed object structure (within entry payload):**
```
[1 byte]  0xD0 tag (CFB_COMPRESSED_TAG)
[1 byte]  id_length
[N bytes] id (pin index as Win-1252 string, e.g. "0", "1", "2")
[4 bytes] inner header: flags(1) | length(3)
[N bytes] inner data (stream-specific binary data)
```

The `id` field is the pin index as a string ("0", "1", "2", ...).
Ordering must match the pin ordering within the component.

### 3.4 Load/Save Lifecycle

**Import (Load) order in `SchDataImporterLibraryV5`:**

1. Main `Data` stream records are read first
2. Components and their pins are identified
3. For each component, sidecar streams are read in this order:
   - `PinFrac` -> `UpdatePinsFractionalCoords()`
   - `PinDesc` -> `UpdatePinsLongDescriptions()`
   - `PinMiscData` -> `UpdatePinsMiscData()`
   - `PinTextData` -> `UpdatePinsCustomTextDisplay()`
   - `PinWideText` -> `UpdatePinsWideText()`
   - `PinSymbolLineWidth` -> `UpdatePinsSymbolLineWidth()`
   - `PinPackageLength` -> `UpdatePinPackageLengths()`
   - `PinPropagationDelay` -> `UpdatePinPropagationDelays()`
   - `PinFunctionData` -> `UpdatePinFunctionData()`

**Export (Save) order in `SchDataExporterLibraryV5`:**

For each component, all 9 sidecar lists are populated per-pin, then written:
```
AddPinFractionalPartsData()     -> WritePinsExtendedData("PinFrac", ...)
AddPinLongDescriptionData()     -> WritePinsExtendedData("PinDesc", ...)
AddPinMiscDataData()            -> WritePinsExtendedData("PinMiscData", ...)
AddPinCustomTextDisplayData()   -> WritePinsExtendedData("PinTextData", ...)
AddPinWideTextData()            -> WritePinsExtendedData("PinWideText", ...)
AddPinSymbolLineWidthData()     -> WritePinsExtendedData("PinSymbolLineWidth", ...)
AddPinPackageLengthData()       -> WritePinsExtendedData("PinPackageLength", ...)
AddPinPropagationData()         -> WritePinsExtendedData("PinPropagationDelay", ...)
AddPinFunctionData()            -> WritePinsExtendedData("PinFunctionData", ...)
```

Each `WritePinsExtendedData()` call skips writing if the list is empty.

### 3.5 PinFrac -- Fractional Coordinates

**Binary data per entry: 12 bytes**

```
[4 bytes] locationX_frac (i32 LE)
[4 bytes] locationY_frac (i32 LE)
[4 bytes] length_frac    (i32 LE)
```

**Import merge:**
```csharp
pin.Location.X += locationX_frac;
pin.Location.Y += locationY_frac;
pin.PinLength  += length_frac;
```

**Export condition:** Only written when pin coordinates don't fit on DXP2004SP1
unit grid. The `PinFitsOnUnit_DXP2004SP1()` method calculates the fractional
remainder; if non-zero, it emits the sidecar entry.

**Purpose:** Backwards compatibility with DXP2004SP1 which only stored
whole-unit coordinates. The fractional parts were added later as a sidecar
stream to avoid breaking older readers.

### 3.6 PinDesc -- Long Descriptions

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] ASCII text (the overflow portion beyond 254 chars)
```

**Import merge:** Appends to existing description:
```csharp
pin.SetDescription(pin.GetDescription() + value);
```

**Export condition:** Only written when `description.Length > 254`.
The exported data contains `description.Substring(254)` (everything
after the first 254 chars).

### 3.7 PinMiscData -- Swap ID Pairs

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string (e.g. "PairSwapID=value")
```

**Import merge:**
```csharp
if (GetParameterValue(value, "PairSwapID", out value2))
    pin.SetSwapIdPair(value2);
```

**Export condition:** Only written when `pin.GetSwapIdPair()` is non-empty.

### 3.8 PinTextData -- Custom Text Display

**Binary data: two consecutive text-data structs (Name, then Designator)**

Each struct:
```
Byte 1: Flags
  Bit 0: PositionMode    (1=Custom, 0=Default)
  Bit 1: RotationAnchor  (1=raComponent, 0=raPin)
  Bits 2-3: RotationRelative (enum 0-3, TRotationBy90)
  Bit 4: FontMode         (1=Custom, 0=Default)

If PositionMode == Custom (bit 0 set):
  [4 bytes] customMargin (i32 LE)

If FontMode == Custom (bit 4 set):
  [2 bytes] customFontID (i16 LE)
  [4 bytes] customColor  (u32 LE)
```

**Variable-length:** Each struct is 1 byte minimum (both defaults), up to
11 bytes (both custom: 1 + 4 + 2 + 4). Two structs concatenated means
2-22 bytes total.

**Fields merged into `SchDataPin`:**

For Name:
- `pin.namePositionMode`
- `pin.nameCustomPositionMargin`
- `pin.nameCustomRotationRelative` (TRotationBy90: eRotate0..eRotate270)
- `pin.nameCustomRotationAnchor` (TPinTextRotationAnchor: raPin/raComponent)
- `pin.nameFontMode`
- `pin.nameCustomFontID`
- `pin.nameCustomColor`

For Designator: same set with `designator` prefix.

**Export condition:** Only written when at least one of the four modes
(namePosition, nameFont, designatorPosition, designatorFont) is Custom.

### 3.9 PinWideText -- Unicode Text

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string
```

**Parameter keys and target fields:**

| Key          | Target Field                    | Override Behavior    |
|--------------|---------------------------------|----------------------|
| `Desc`       | `pin.description`               | Full replacement     |
| `Name`       | `pin.name`                      | Full replacement     |
| `Desig`      | `pin.designator`                | Full replacement     |
| `SwapId`     | `pin.swapIdPin`                 | Full replacement     |
| `SwapIDPart` | `pin.swapIdPartAndPartPin`      | Full replacement     |
| `DefValue`   | `pin.defaultValue`              | Full replacement     |

**Import merge:** Unlike PinDesc (which appends), PinWideText **fully replaces**
the target field. This is because PinWideText contains the complete text
(including non-ASCII characters or text exceeding 254 chars).

**Export condition:** Only written when at least one field is non-empty.

**Import order matters:** PinWideText is imported AFTER PinDesc, so its full
replacement overwrites the append done by PinDesc. PinWideText is the
authoritative source for text when present.

### 3.10 PinSymbolLineWidth

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string (e.g. "SymBol_LineWidth=3")
```

**Import merge:**
```csharp
pin.SetSymbolLineWidth((TSize)int.Parse(value));
```

**Export condition:** Only written when `pin.GetSymbolLineWidth() != TSize.eZeroSize`.

### 3.11 PinPackageLength

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string (e.g. "PinPackageLength=200000")
```

**Import merge:**
```csharp
pin.SetPinPackageLength(int.Parse(value));
```

**Export condition:** Only written when `pin.GetPinPackageLength() != 0`.

### 3.12 PinPropagationDelay

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string (e.g. "PinPropagationDelay=1.5E-9")
```

**Import merge:**
```csharp
pin.SetPropagationDelay(StrUtils.TryParseExponent(value));
```

**Export condition:** Only written when `pin.GetPropagationDelay() != 0.0`.
Uses `StrUtils.DelayToString()` for scientific notation serialization.

### 3.13 PinFunctionData -- Pin Alternate Functions

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string
```

**Parameter keys:**

| Key                         | Purpose                           |
|-----------------------------|-----------------------------------|
| `PinSelectedFunctionsCount` | Number of selected functions      |
| `PinSelectedFunction1..N`   | Selected function names (1-based) |
| `PinDefinedFunctionsCount`  | Number of defined functions       |
| `PinDefinedFunction1..N`    | Defined function names (1-based)  |

**Import merge:**
```csharp
for (int i = 1; i <= count; i++)
    pinFunctions.AddFunction(GetParameterValue("PinSelectedFunction" + i));
```

**Note:** Function indices are **1-based**, unlike most other Altium indices.

**Export condition:** Only written when at least one function list has entries.

### 3.14 Redirection -- Component Aliases

**Format:** Single parameter block in a separate stream.

**Location:** `/<alias_section_key>/Redirection`

**Content:** `SECTIONNAME=<canonical_component_name>`

**Import logic (from `SchDataImporterLibraryV5.GetLibraryReferenceByAliasName`):**
1. Check if `/<section>/Redirection` stream exists
2. If yes, read `SECTIONNAME` parameter -> resolve to canonical component
3. If no Redirection stream but `/<section>/Data` exists, use the section name directly
4. Otherwise, fall back to searching `FileHeader` for alias mappings

**Purpose:** For each component alias, Altium creates a CFB section `/<alias>/`
with a `Redirection` stream pointing to the canonical component. The canonical
component has an `AliasList` (`SchDataAliasList`) tracking all its aliases.

---

## 4. Corrections and Additions to SIDECARS.md

### 4.1 Corrections

1. **PinFrac data size:** The existing doc correctly states 12 bytes.

2. **PinWideText replacement vs append:** The existing doc doesn't explicitly
   state that PinWideText REPLACES fields rather than appending. This is
   important because PinDesc (which runs first) appends to the description,
   but PinWideText (which runs after) fully replaces it.

3. **Import ordering:** The document doesn't emphasize that the import order
   matters. PinWideText must be processed AFTER PinDesc because it replaces
   the description entirely.

### 4.2 Missing Information Now Documented

1. **SchDoc vs SchLib scope:** Pin sidecar streams are SchLib-only. SchDoc
   files do not use them.

2. **PinFunctionData stream:** Not previously documented. Contains pin
   alternate function definitions (selected and defined function lists).

3. **PcbLib WideStrings format difference:** The existing doc mentions this
   but the ENCODEDTEXT parameter format wasn't fully specified.

4. **PrimitiveGuids stream:** Not previously documented. 24-byte binary
   entries with tag/index/GUID for footprint primitives.

5. **Export conditions:** Each sidecar stream has specific conditions for
   when data is emitted (non-default values, non-empty strings, etc.).

6. **Full binary format for PinTextData:** The bit-packed flags and variable-
   length nature of the struct is now fully documented.

7. **Compressed block envelope:** The CFB_COMPRESSED_TAG (0xD0), id length
   encoding, and inner header structure are now documented at the byte level.

---

## 5. Source References

### .NET Decompiled Sources

| File | Purpose |
|------|---------|
| `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs` | Stream name constants |
| `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataImporterLibraryV5.cs` | SchLib import with all 9 sidecar merges |
| `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataExporterLibraryV5.cs` | SchLib export with all 9 sidecar writes |
| `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Objects/SchDataEmbeddedObject.cs` | Embedded object container |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_Primitive.cs` | UniqueId get/set on primitives |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_Primitive2.cs` | Mask expansion, paste mask, GUID |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_WideStrings.cs` | WideStrings list interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TMaskExpansionMode.cs` | Mask expansion mode enum |
| `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/TPrimitiveAttribute.cs` | Full primitive attribute enum |
| `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_WideStrings.cs` | SDK WideStrings interface |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Board2.cs` | Board-level operations |
| `AD26-dotnet/InteractiveProperties.Providers.PCB.DataModel/*/PcbPrimitiveDataObject.cs` | UI mask expansion handling |

### Ghidra Decompiled (Delphi DLLs)

| DLL | Address | Function | Purpose |
|-----|---------|----------|---------|
| `Advpcb.dll` | `0x548920` | `FUN_00548920` | WideStrings binary TLV read |

### Existing Codebase

| File | Purpose |
|------|---------|
| `crates/altium-format/src/documents/schlib_streams.rs` | SchLib sidecar stream codecs |
| `crates/altium-format/src/documents/pcblib_streams.rs` | PcbLib sidecar stream codecs |
