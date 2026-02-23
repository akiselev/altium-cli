# Sidecar Streams

PcbDoc files contain several **sidecar streams** that extend the core primitive data
stored in the main section Data streams. These exist as a backwards-compatibility
mechanism: new data fields were added in separate streams so older readers could still
parse the core binary records.

At load time, sidecar data is merged into primitives by index. At runtime, there is no
distinction between core and sidecar data.

## Overview

| Stream | Format | Purpose |
|--------|--------|---------|
| `WideStrings6/Header` + `Data` | Binary: index + length + UTF-16LE | Unicode text for primitives |
| `UniqueIDPrimitiveInformation/Header` + `Data` | Parameter blocks | Per-primitive identity strings for ECO sync |
| `ExtendedPrimitiveInformation/Header` + `Data` | Parameter blocks | Per-primitive mask expansion overrides |
| `PrimitiveGuids/Header` + `Data` | Binary: packed TPrimitiveGUID (24 bytes each) | Persistent GUIDs per primitive |
| `PrimitiveParameters/Header` + `Data` | Parameter blocks (hierarchical) | Component-level imported parameters |

All sidecar streams use the `Header` + `Data` sub-stream pattern within a top-level
CFB storage of the same name:

```
/WideStrings6/
    Header          (4 bytes: u32 LE entry count)
    Data            (flat binary or parameter blocks)
```

---

## WideStrings6

**Streams**: `WideStrings6/Header` + `WideStrings6/Data`

### CRITICAL: PcbDoc vs PcbLib format difference

PcbDoc WideStrings6 uses a **flat binary format** with UTF-16LE encoded strings.
PcbLib footprint-level WideStrings uses a **completely different parameter-block format**
(`ENCODEDTEXT0=...`). These share NO structure and require separate parser implementations.

### Header

4 bytes: `u32 LE` entry count.

Example from LimeSDR_Mini_1v3: `3F 05 00 00` = 1343 entries.

### Data format

The Data stream is **NOT block-framed** (no standard 4-byte block headers with flags).
It is a flat binary sequence of entries, one per primitive that has a text field.

Each entry:

```
[4 bytes] u32 LE: primitive index (sequential 0, 1, 2, ...)
[4 bytes] u32 LE: byte_length (UTF-16LE byte count, includes NUL terminator)
[byte_length bytes] UTF-16LE encoded string (NUL-terminated, NUL included in byte_length)
```

### Decoded example (LimeSDR_Mini_1v3)

```
Entry 0:  idx=0,  len=252, text="Assembly note:\r\nLED2 can be mounted sticking out of..."
Entry 1:  idx=1,  len=24,  text=".Designator"
Entry 2:  idx=2,  len=24,  text=".Designator"
...
Entry 1342: idx=1342, len=6, text="NC"
```

Total: 1343 entries, 34824 bytes (consumes the entire Data stream exactly).

### Hex dump of entries 0-1

```
Offset  Bytes                                             Interpretation
------  -----                                             --------------
0x0000  00 00 00 00                                       index = 0
0x0004  FC 00 00 00                                       byte_length = 252
0x0008  41 00 73 00 73 00 65 00 6D 00 62 00 6C 00 79 00  "A.s.s.e.m.b.l.y."
        20 00 6E 00 6F 00 74 00 65 00 3A 00 0D 00 0A 00  " .n.o.t.e.:.\r.\n."
        ...                                               (252 bytes total)
0x0104  01 00 00 00                                       index = 1
0x0108  18 00 00 00                                       byte_length = 24
0x010C  2E 00 44 00 65 00 73 00 69 00 67 00 6E 00 61 00  ".Designator"
        74 00 6F 00 72 00 00 00                           (NUL terminator)
```

### Which primitives reference WideStrings

The `WideStringObjects` constant in `Consts.cs` (line 70) defines an array of 4 TObjectId
values identifying which primitive types participate in the WideStrings table. From context
and observation, these are primitives that carry text content:

- `eTextObject` (5) -- text string primitives
- `ePadObject` (2) -- pad name/designator
- `eComponentObject` (9) -- component designator/comment
- `eDimensionObject` (13) -- dimension annotation text

Each such primitive stores a WideStrings index field. During load, the runtime calls
`AddWSForLoadList(index, text)` via `IPCB_StructuredStorage` to register each entry.
During save, `AddTextsForSaveList(primitive)` collects primitives that need WideStrings entries.

### Empty strings

Entry index 0 in the LimeSDR file has a 252-byte text. An entry with `byte_length=0`
represents an empty string (the primitive has no wide text). The index field is still
present.

### Encoding notes

- All observed entries in the test file use UTF-16LE encoding
- The byte_length includes the UTF-16LE NUL terminator (2 bytes: `00 00`)
- Strings that contain only ASCII characters are still stored as UTF-16LE in this format

### Historical note on TLV format

The existing Ghidra analysis at `Advpcb.dll:0x548920` documents a binary TLV encoding
with type bytes `0x06` (ASCII u8 length), `0x0C` (ASCII u32 length), `0x12` (UTF-16LE),
`0x14` (UTF-8). This TLV format may be used by older PcbDoc format versions or by the
Delphi reader internals. The LimeSDR_Mini_1v3 file (PCB 6.0 format) uses the simpler
`[index][length][UTF-16LE]` format described above. Further investigation across format
versions is needed to determine when the TLV encoding applies.

---

## UniqueIDPrimitiveInformation

**Streams**: `UniqueIDPrimitiveInformation/Header` + `UniqueIDPrimitiveInformation/Data`

### Purpose

Assigns unique identity strings to primitives for schematic-to-PCB linking. When ECO
(Engineering Change Order) operations synchronize between SchDoc and PcbDoc, the UniqueID
is the identity key that maps schematic pins/components to their PCB counterparts.

### Header

4 bytes: `u32 LE` entry count.

Example from LimeSDR_Mini_1v3: `EB 06 00 00` = 1771 entries.

### Data format

Standard parameter block format: a flat sequence of `[u32 LE length] [NUL-terminated
parameter string]` entries. One entry per primitive that has a unique ID assigned.

### Parameter keys

| Key | Example | Description |
|-----|---------|-------------|
| `PRIMITIVEINDEX` | `0` | Zero-based index of the primitive within its section |
| `PRIMITIVEOBJECTID` | `Pad` | Object type name (text string, not numeric TObjectId) |
| `UNIQUEID` | `DSEHSGPL` | 8-character unique identity string |

### PRIMITIVEOBJECTID values

The `PRIMITIVEOBJECTID` field uses text names, not numeric IDs:

| PRIMITIVEOBJECTID | TObjectId value |
|-------------------|-----------------|
| `Arc` | 1 (eArcObject) |
| `Pad` | 2 (ePadObject) |
| `Via` | 3 (eViaObject) |
| `Track` | 4 (eTrackObject) |
| `Text` | 5 (eTextObject) |
| `Fill` | 6 (eFillObject) |
| `Region` | 11 (eRegionObject) |
| `ComponentBody` | 12 (eComponentBodyObject) |

### Decoded example (LimeSDR_Mini_1v3)

```
Block 0:  len=58, "|PRIMITIVEINDEX=0|PRIMITIVEOBJECTID=Pad|UNIQUEID=DSEHSGPL"
Block 1:  len=58, "|PRIMITIVEINDEX=1|PRIMITIVEOBJECTID=Pad|UNIQUEID=PCCNLDTN"
Block 2:  len=58, "|PRIMITIVEINDEX=2|PRIMITIVEOBJECTID=Pad|UNIQUEID=PSGAIGFH"
...
Block 1770: (last entry)
```

Total: 1771 entries matching Header count, 114005 bytes consumed exactly.

### Hex dump of block 0

```
Offset  Bytes
------  -----
0x0000  3A 00 00 00                                       length = 58
0x0004  7C 50 52 49 4D 49 54 49 56 45 49 4E 44 45 58 3D  |PRIMITIVEINDEX=
0x0014  30 7C 50 52 49 4D 49 54 49 56 45 4F 42 4A 45 43  0|PRIMITIVEOBJEC
0x0024  54 49 44 3D 50 61 64 7C 55 4E 49 51 55 45 49 44  TID=Pad|UNIQUEID
0x0034  3D 44 53 45 48 53 47 50 4C 00                    =DSEHSGPL\0
```

### Observations (LimeSDR_Mini_1v3)

- All 1771 entries are Pads -- only pad primitives have UniqueIDs in this board
- PRIMITIVEINDEX values are unique per PRIMITIVEOBJECTID section (not global)
- UniqueID strings are always 8 uppercase ASCII characters

### .NET interface

From `IPCB_Primitive`:
- `GetState_UniqueId() -> string`
- `SetState_UniqueID(string)`

From `IPCB_Board2`:
- `GenerateUniqueID() -> string` (creates new 8-char unique IDs)

---

## ExtendedPrimitiveInformation

**Streams**: `ExtendedPrimitiveInformation/Header` + `ExtendedPrimitiveInformation/Data`

### Purpose

Provides per-primitive property overrides for mask expansion and other properties that
were added in later format versions. When a primitive needs mask expansion values
different from the board defaults, this stream stores the override.

### Header

4 bytes: `u32 LE` entry count.

Example from LimeSDR_Mini_1v3: `01 00 00 00` = 1 entry.

### Data format

Same parameter block format as UniqueIDPrimitiveInformation: `[u32 LE length]
[NUL-terminated parameter string]`.

### Parameter keys

| Key | Example | Description |
|-----|---------|-------------|
| `PRIMITIVEINDEX` | `332` | Zero-based primitive index within its section |
| `PRIMITIVEOBJECTID` | `Region` | Object type name |
| `TYPE` | `Mask` | Extended information category |
| `SOLDERMASKEXPANSIONMODE` | `Manual` | Solder mask expansion mode |
| `SOLDERMASKEXPANSION_MANUAL` | `1.9685mil` | Manual solder mask expansion value |
| `PASTEMASKEXPANSIONMODE` | `None` | Paste mask expansion mode |
| `PASTEMASKEXPANSION_MANUAL` | (coord value) | Manual paste mask expansion |

### TMaskExpansionMode enum

From `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TMaskExpansionMode.cs`:

| Enum Value | Byte | Serialized String |
|-----------|------|-------------------|
| `eMaskExpansionMode_NoMask` | 0 | `"None"` |
| `eMaskExpansionMode_Rule` | 1 | `"Rule"` |
| `eMaskExpansionMode_Manual` | 2 | `"Manual"` |

### Decoded example (LimeSDR_Mini_1v3)

Only 1 entry in this file:

```
Block 0: len=151
"|PRIMITIVEINDEX=332|PRIMITIVEOBJECTID=Region|TYPE=Mask|SOLDERMASKEXPANSIONMODE=Manual|SOLDERMASKEXPANSION_MANUAL=1.9685mil|PASTEMASKEXPANSIONMODE=None"
```

### Hex dump

```
Offset  Bytes
------  -----
0x0000  97 00 00 00                                       length = 151
0x0004  7C 50 52 49 4D 49 54 49 56 45 49 4E 44 45 58 3D  |PRIMITIVEINDEX=
0x0014  33 33 32 7C 50 52 49 4D 49 54 49 56 45 4F 42 4A  332|PRIMITIVEOBJ
0x0024  45 43 54 49 44 3D 52 65 67 69 6F 6E 7C 54 59 50  ECTID=Region|TYP
0x0034  45 3D 4D 61 73 6B 7C ...                          E=Mask|...
```

### Property resolution chain

From `IPCB_Primitive2`, the mask expansion value a consumer sees depends on mode:

1. **NoMask (None)**: No mask expansion applied at all
2. **Rule**: Query matching design rule (`IPCB_SolderMaskExpansionRule` or
   `IPCB_PasteMaskExpansionRule`) from the board design rules
3. **Manual**: Use the primitive's local expansion value directly

### ExtendedPrimitiveIndices (optional)

The `ExtendedPrimitiveIndices` stream (stream index 38 in the PCB stream name table)
provides a fast lookup table for random access into ExtendedPrimitiveInformation.
This avoids linear scanning when looking up a specific primitive's extended properties.

**Format**: Packed `TReferenceToGroup` entries (16 bytes each):

```
struct TReferenceToGroup {  // 16 bytes, pack=8
    TPrimitiveKey Prim;       // 8 bytes: (i32 ObjectId, i32 IndexForSave)
    TPrimitiveKey PrimGroup;  // 8 bytes: (i32 ObjectId, i32 IndexForSave)
}
```

The indices stream is an optimization. The Data stream itself contains PRIMITIVEINDEX in
each parameter block, so the information is self-contained without the indices.

Not present in the LimeSDR_Mini_1v3 test file.

---

## PrimitiveGuids

**Streams**: `PrimitiveGuids/Header` + `PrimitiveGuids/Data`

### Purpose

Assigns a persistent GUID to each primitive, separate from the UniqueID string mechanism.
GUIDs survive across library updates and are used for `IPCB_Primitive2.GetGUID()` /
`SetGUID()`.

### Header

4 bytes: `u32 LE` entry count (number of 24-byte records in Data).

### Data format

Packed binary records, 24 bytes each:

```
struct TPrimitiveGUID {     // 24 bytes, pack=1
    i32 ObjectId;           // 4 bytes: TObjectId enum value (primitive type)
    i32 IndexForSave;       // 4 bytes: primitive index within its section
    [16 bytes] GUID;        // 16 bytes: standard Windows GUID (little-endian)
}
```

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPrimitiveGUID.cs`

### Matching process

GUIDs are applied to primitives via `IPCB_BinarySection.ApplyGUIDs()`:

1. Read Header to get count
2. Read count * 24 bytes from Data as packed `TPrimitiveGUID` records
3. For each record, locate the primitive at `(ObjectId, IndexForSave)`
4. Call `primitive.SetGUID(guid)` to assign the GUID

### .NET interface

From `IPCB_BinarySection`:
- `GuidsCount() -> int`
- `GetGUID(index) -> TPrimitiveGUID`
- `AddGUID(TPrimitiveGUID)`
- `ApplyGUIDs()`

From `IPCB_Primitive2`:
- `GetGUID() -> Guid`

### Notes

Not present in the LimeSDR_Mini_1v3 test file (stream does not exist in the CFB
container). PrimitiveGuids may be more common in PcbLib files where footprint primitives
need persistent identity across library updates.

---

## PrimitiveParameters

**Streams**: `PrimitiveParameters/Header` + `PrimitiveParameters/Data`

### Purpose

Stores component-level imported parameters (BOM data, manufacturer info, specifications).
These are parameters that were imported from the schematic or component library and
attached to component primitives in the PCB.

### Header

4 bytes: `u32 LE` component count (number of component header blocks, NOT total
parameter blocks).

Example from LimeSDR_Mini_1v3: `A3 01 00 00` = 419 components.

### Data format

Hierarchical parameter block format. The Data stream contains groups of parameter blocks,
where each group starts with a **component header block** followed by N **parameter blocks**:

```
[Component header block]
    |PRIMITIVEID=<id>|VARIANTGUID=<guid>|COUNT=<N>|
[Parameter block 1]
    |NAME=<name>|VALUE=<value>|ISIMPORTED=TRUE|
[Parameter block 2]
    |NAME=<name>|VALUE=<value>|ISIMPORTED=TRUE|
...
[Parameter block N]
    |NAME=<name>|VALUE=<value>|ISIMPORTED=TRUE|
[Next component header block]
...
```

Each block uses the standard `[u32 LE length] [NUL-terminated parameter string]` format.

### Component header keys

| Key | Example | Description |
|-----|---------|-------------|
| `PRIMITIVEID` | `ABFAMIIA` | Component's unique ID (links to UniqueIDPrimitiveInformation) |
| `VARIANTGUID` | (empty or GUID) | Design variant identifier |
| `COUNT` | `18` | Number of parameter blocks that follow |

### Parameter block keys

| Key | Example | Description |
|-----|---------|-------------|
| `NAME` | `Manufacturer` | Parameter name |
| `VALUE` | `Taiyo Yuden` | Parameter value |
| `ISIMPORTED` | `TRUE` | Whether the parameter was imported from schematic |

### Decoded example (LimeSDR_Mini_1v3)

```
>>> Component 1: |PRIMITIVEID=ABFAMIIA|VARIANTGUID=|COUNT=18
       |NAME=Assembly Info|VALUE=|ISIMPORTED=TRUE
       |NAME=Category|VALUE=Filters|ISIMPORTED=TRUE
       |NAME=Current Rating|VALUE=270mA|ISIMPORTED=TRUE
       |NAME=DC Resistance (DCR)|VALUE=750 mOhm Max|ISIMPORTED=TRUE
       |NAME=Manufacturer|VALUE=Taiyo Yuden|ISIMPORTED=TRUE
       |NAME=Manufacturer Part Number 1|VALUE=BK0603TS601-T|ISIMPORTED=TRUE
       ... (18 parameters total)

>>> Component 2: |PRIMITIVEID=AQEIHYFO|VARIANTGUID=|COUNT=25
       |NAME=Applications|VALUE=General Purpose|ISIMPORTED=TRUE
       ... (25 parameters total)

... (419 components total, 9807 blocks total)
```

### Statistics (LimeSDR_Mini_1v3)

- Components: 419 (matches Header count)
- Total blocks: 9807 (419 header blocks + 9388 parameter blocks)
- Data stream size: 602562 bytes (consumed exactly)

---

## Sidecar merging process

### During load

1. Parse all primitives from each section's `Data` stream, assigning each a sequential
   0-based `IndexForSave` within its section.

2. **WideStrings6**: Read Header for count. Read Data as sequential
   `[index][length][UTF-16LE]` entries. For each entry, call
   `AddWSForLoadList(index, text)` to register the Unicode text.

3. **UniqueIDPrimitiveInformation**: Read Header for count. Parse Data as parameter
   blocks. For each entry, look up the primitive at `(PRIMITIVEOBJECTID, PRIMITIVEINDEX)`
   and call `primitive.SetState_UniqueID(UNIQUEID)`.

4. **ExtendedPrimitiveInformation**: Read Header for count. Parse Data as parameter
   blocks. Merge mask expansion and other extended properties into primitives by
   `(PRIMITIVEOBJECTID, PRIMITIVEINDEX)`.

5. **PrimitiveGuids**: Read Header for count. Parse Data as packed 24-byte records.
   Call `ApplyGUIDs()` on each section to match GUIDs to primitives by
   `(ObjectId, IndexForSave)`.

6. **PrimitiveParameters**: Read Header for component count. Parse Data as hierarchical
   parameter blocks. Match components by PRIMITIVEID to component primitives.

### During save (reverse process)

1. Primitives are assigned sequential `IndexForSave` values per section
2. Core binary records are written to each section's Data stream
3. `AddTextsForSaveList()` collects text primitives that need WideStrings entries
4. UniqueIDs, extended properties, GUIDs, and parameters are split out from each
   primitive and written to the appropriate sidecar streams

### Key interfaces

| Interface | Method | Purpose |
|-----------|--------|---------|
| `IPCB_StructuredStorage` | `AddWSForLoadList(index, text)` | Register a WideString entry |
| `IPCB_StructuredStorage` | `AddTextsForSaveList(primitive)` | Collect primitives for WideStrings save |
| `IPCB_BinarySection` | `ApplyGUIDs()` | Merge PrimitiveGuids into loaded primitives |
| `IPCB_BinarySection` | `ApplyExtendedIndices()` | Apply extended group indices |

---

## Comparison with PcbLib sidecar streams

PcbLib footprints have the same sidecar stream types but with important differences:

| Feature | PcbDoc | PcbLib |
|---------|--------|--------|
| **WideStrings format** | Binary: `[u32 idx][u32 len][UTF-16LE]` | Parameter blocks: `ENCODEDTEXT0=...` (comma-separated byte values) |
| **WideStrings stream name** | `WideStrings6/Data` | `<FootprintName>/WideStrings` |
| **WideStrings header** | Separate `Header` sub-stream | Single block in same stream |
| **Scope** | Global (all board primitives) | Per-footprint |
| **PrimitiveGuids** | Global `PrimitiveGuids/` | Per-footprint `<name>/PrimitiveGuids/` |
| **PrimitiveParameters** | Present (component BOM data) | Not applicable (library level) |

The PcbLib and PcbDoc WideStrings formats share NO structure and require completely
separate parser implementations.

---

## Source references

### .NET decompiled sources

| File | Purpose |
|------|---------|
| `Altium.Edp.Interfaces/RT_PCB/IPCB_StructuredStorage.cs` | Storage interface (AddWSForLoadList, AddTextsForSaveList) |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_BinarySection.cs` | Section interface (ApplyGUIDs, ApplyExtendedIndices) |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_WideStrings.cs` | WideStrings list interface (Add, Get, GetCount) |
| `Altium.Edp.Interfaces/RT_PCB/TPrimitiveGUID.cs` | 24-byte GUID struct layout |
| `Altium.Edp.Interfaces/RT_PCB/TPrimitiveKey.cs` | 8-byte primitive key (ObjectId + IndexForSave) |
| `Altium.Edp.Interfaces/RT_PCB/TReferenceToGroup.cs` | 16-byte extended index struct |
| `Altium.Edp.Interfaces/RT_PCB/TMaskExpansionMode.cs` | Mask expansion mode enum |
| `Altium.Edp.Interfaces/RT_PCB/Consts.cs` | WideStringObjects array (line 70) |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_Primitive2.cs` | Mask expansion, paste mask, GUID properties |

### Ghidra decompiled (Delphi DLLs)

| DLL | Address | Purpose |
|-----|---------|---------|
| `Advpcb.dll` | `0x548920` | WideStrings binary TLV read function |
| `BinaryLoader.dll` | `0x019679e0` | `TWideStringsSection.Create` constructor |
| `BinaryLoader.dll` | `0x01918020` | `RegisterAllSectionsForExporting` (section registration) |

### Existing codebase

| File | Purpose |
|------|---------|
| `crates/altium-format/src/wide_strings_tlv.rs` | WideStrings6 TLV parser (may need revision for observed format) |
| `crates/altium-format/src/documents/pcblib_streams.rs` | PcbLib sidecar stream codecs |

### Test data

All hex dumps and statistics in this document are from `data/LimeSDR_Mini_1v3_Rounded.PcbDoc`.
