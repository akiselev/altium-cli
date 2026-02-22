# SchDoc and SchLib File Format: Complete Loading and Saving Guide

This document describes the exact file structure and loading/saving pipelines for
Altium Designer SchDoc (schematic document) and SchLib (schematic library) files,
based on reverse engineering of the decompiled .NET source code from
`Altium.Sch.DataModel.dll` (AD26).

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [CFB Document Structure for SchDoc](#2-cfb-document-structure-for-schdoc)
3. [CFB Document Structure for SchLib](#3-cfb-document-structure-for-schlib)
4. [Record Format in the Data Stream](#4-record-format-in-the-data-stream)
5. [SchDoc Loading Pipeline](#5-schdoc-loading-pipeline)
6. [SchLib Loading Pipeline](#6-schlib-loading-pipeline)
7. [Object Hierarchy and OWNERINDEX Linking](#7-object-hierarchy-and-ownerindex-linking)
8. [Font Table Format](#8-font-table-format)
9. [FileHeader Format](#9-fileheader-format)
10. [The Alias and Redirection System](#10-the-alias-and-redirection-system)
11. [Export (Save) Pipeline](#11-export-save-pipeline)
12. [Embedded Object Container Format](#12-embedded-object-container-format)
13. [Binary Code to TObjectId Mapping](#13-binary-code-to-tobjectid-mapping)

---

## 1. Architecture Overview

### Class Hierarchy

The import/export system uses this class hierarchy:

```
SchDataImporterExporterBase              (abstract base, holds serializer + file format)
  |
  +-- SchDataImporterBaseV5              (abstract, holds BaseWarehouse/ExtendedWarehouse/AdditionalWarehouse)
  |     |
  |     +-- SchDataImporterDocumentV5    (abstract, SchDoc loading logic)
  |     |     |
  |     |     +-- SchDataImporterSheetV5 (concrete, SchDoc sheets)
  |     |
  |     (SchLib uses its own parallel hierarchy, see below)
  |
  +-- SchDataExporterBaseV5              (abstract, save logic)
  |     |
  |     +-- SchDataExporterDocumentV5    (abstract, SchDoc save logic)
  |     |     |
  |     |     +-- SchDataExporterSheetV5 (concrete, SchDoc sheet save)
  |     |
  |     +-- SchDataExporterLibraryV5     (concrete, SchLib save)
  |
  +-- SchDataImporterLibrary             (abstract)
        |
        +-- SchDataImporterLibraryV5     (concrete, SchLib loading)
```

**Source files:**
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataImporterExporterBase.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataImporterBaseV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataImporterDocumentV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataImporterSheetV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataImporterLibraryV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataExporterBaseV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataExporterDocumentV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataExporterSheetV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataExporterLibraryV5.cs`

### Serializer

The serializer (`SchDataSerializerParam`) wraps an OLE/CFB Compound File using the
OpenMcdf library. It handles bidirectional translation between CFB streams and the
parameter-based record format.

**Source:** `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Serialization/SchDataSerializerParam.cs`

### The Three Warehouses

Both SchDoc and SchLib use a three-warehouse architecture:

- **BaseWarehouse** (`ISchDataObjectList`): Contains the main objects (the sheet/component
  records, pins, wires, labels, etc.)
- **ExtendedWarehouse** (`List<SchDataEmbeddedObject>`): Contains binary embedded data
  (images for SchDoc; images for SchLib)
- **AdditionalWarehouse** (`ISchDataObjectList`): Contains supplementary objects that
  reference back into the base warehouse (implementation lists, parameter lists, etc.)

### File Format Versions

The V5 binary format is identified by its header string:

- **SchDoc**: `"Protel for Windows - Schematic Capture Binary File Version 5.0"`
- **SchLib**: `"Protel for Windows - Schematic Library Editor Binary File Version 5.0"`

Both use `TFileFormatVersion.ffv5` and `TSerializerType.stParametric`.

**Source:** `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs`

---

## 2. CFB Document Structure for SchDoc

A SchDoc file is a Microsoft Compound Binary File (CFB/OLE2) with the following
streams at the root storage level:

```
Root Storage
 |
 +-- FileHeader         (main object records + font table)
 +-- Storage            (embedded binary objects - images)
 +-- Additional         (supplementary records)
 +-- ObjectDefinitions  (optional - object definition records)
 +-- ReuseBlockInfos    (optional - reuse block info records)
 +-- ReuseBlocks        (optional - reuse block data, embedded binary)
 +-- ReuseBlocksV2      (optional - extension for reuse blocks)
 +-- HarnessConnectionPointConnector  (optional - harness data, embedded binary)
 +-- Files              (optional - attached files)
```

### Stream Names (from FileFormatConsts)

| Constant | Value |
|----------|-------|
| `StreamNameFileHeader` | `"FileHeader"` |
| `StreamNameStorage` | `"Storage"` |
| `StreamNameAdditional` | `"Additional"` |
| `StreamNameObjectDefinitions` | `"ObjectDefinitions"` |
| `StreamNameReuseBlockInfos` | `"ReuseBlockInfos"` |
| `StreamNameReuseBlocks` (embedded data name) | `"ReuseBlocks"` |
| `StreamNameReuseBlocksV2` (embedded data name) | `"ReuseBlocksV2"` |
| `StreamNameHarnessConnectionPointConnector` | `"HarnessConnectionPointConnector"` |
| `StreamNameFiles` | `"Files"` |

---

## 3. CFB Document Structure for SchLib

A SchLib file has a more complex CFB structure. It has global streams at the root level
plus per-component storages (sub-storages named after the component's LibReference,
possibly truncated/disambiguated via the SectionKey system):

```
Root Storage
 |
 +-- FileHeader                  (library header + component index + font table)
 +-- Storage                     (global embedded binary objects - images)
 +-- SectionKeys                 (optional - name-to-key mapping for long names)
 +-- LibAdditional               (optional - header for per-component additional data)
 |
 +-- <ComponentKey1>/            (CFB sub-storage for component 1)
 |    +-- Data                   (component records)
 |    +-- Additional             (optional - component additional records)
 |    +-- PinFrac                (optional - pin fractional coordinates)
 |    +-- PinDesc                (optional - pin long descriptions)
 |    +-- PinMiscData            (optional - pin misc data like PairSwapID)
 |    +-- PinTextData            (optional - pin custom text display settings)
 |    +-- PinWideText            (optional - pin wide/Unicode text fields)
 |    +-- PinSymbolLineWidth     (optional - pin symbol line widths)
 |    +-- PinPackageLength       (optional - pin package lengths)
 |    +-- PinPropagationDelay    (optional - pin propagation delays)
 |    +-- PinFunctionData        (optional - pin function data)
 |
 +-- <AliasKey1>/                (CFB sub-storage for alias 1)
 |    +-- Redirection            (redirect to canonical component name)
 |
 +-- <ComponentKey2>/
 |    +-- Data
 |    +-- ...
 ...
```

### Per-Component Stream Names

| Constant | Value | Description |
|----------|-------|-------------|
| `StreamNameData` | `"Data"` | Component records |
| `StreamNameAdditional` | `"Additional"` | Per-component additional records |
| `StreamNameRedirection` | `"Redirection"` | Alias redirect |
| `StreamNameSectionKeys` | `"SectionKeys"` | Name-to-key mapping |
| `StreamNamePinFrac` | `"PinFrac"` | Pin fractional coordinates |
| `StreamNamePinDesc` | `"PinDesc"` | Pin long descriptions |
| `StreamNamePinMiscData` | `"PinMiscData"` | Pin misc data |
| `StreamNamePinTextData` | `"PinTextData"` | Pin custom text display |
| `StreamNamePinWideText` | `"PinWideText"` | Pin wide/Unicode text |
| `StreamNamePinSymbolLineWidth` | `"PinSymbolLineWidth"` | Pin symbol line width |
| `StreamNamePinPackageLength` | `"PinPackageLength"` | Pin package length |
| `StreamNamePinPropagationDelay` | `"PinPropagationDelay"` | Pin propagation delay |
| `StreamNamePinFunctionData` | `"PinFunctionData"` | Pin function data |

---

## 4. Record Format in the Data Stream

### Binary Stream Structure

Each CFB stream (FileHeader, Data, Storage, Additional, etc.) consists of a sequence
of **blocks**. Each block has a 4-byte header followed by payload data.

```
+---+---+---+---+---------------------------+
| Length (3 bytes) | Mode (1 byte) | Payload |
+---+---+---+---+---------------------------+
```

The 4-byte integer at the start of each block encodes:
- **Bits 0-23 (lower 3 bytes)**: Length of the payload in bytes
- **Bits 24-31 (upper byte)**: Mode - `0x00` for parameter (ASCII) mode, `0x01` for binary mode

This is implemented in `SchDataSerializerParam.FlushCurrent()`:

```csharp
// For ASCII/parameter mode (mode == 0):
int num = paramsAsBytes.Length | (mode << 24);  // mode=0, so just length
streamData.Write(BitConverter.GetBytes(num), 0, 4);
streamData.Write(paramsAsBytes, 0, paramsAsBytes.Length);

// For binary mode (mode == 1):
int num2 = Convert.ToInt32(binaryBuffer.Length) | 0x1000000;  // 0x01 in upper byte
streamData.Write(BitConverter.GetBytes(num2), 0, 4);
streamData.Write(array, 0, array.Length);
```

### Parameter (ASCII) Mode Records

When the mode byte is `0x00`, the payload is a null-terminated UTF-8 string of
pipe-delimited key=value pairs. This is the format used for schematic object records.

Format: `|key1=value1|key2=value2|...\0`

Each record starts with `|RECORD=<id>` which identifies the object type. All other
fields follow as key=value pairs separated by `|`.

Example record:
```
|RECORD=31|HEADER=Protel for...|Weight=42|FontIdCount=3|...
```

### Binary Mode Records

When the mode byte is `0x01`, the payload is raw binary data. This is used for
embedded objects (images, pin sidecar data).

The binary mode block structure for embedded objects has a sub-header:

```
[4 bytes: length | 0x01000000]
[1 byte: marker = 0xD0 (208)]
[1 byte: name_length]
[name_length bytes: name string]
[4 bytes: data_length]
[data_length bytes: zlib-compressed data]
```

### RECORD Field and Binary Codes

The `RECORD` field in each parameter record is a byte (or extended int) that maps to
a `TObjectId` enum value. This is the "binary code" for the object type.

When `RECORD=254` (0xFE), the actual type is stored in a subsequent `RECORDEX` field
as a 4-byte integer. This extension mechanism was added to support object types with
codes > 255.

```csharp
// From SchDataImporterDocumentV5.ReadBaseWarehouse():
base.Serializer.Import_Instruction(ref argN, "RECORD");
if (argN == 254) {
    base.Serializer.Import_InstructionEx(ref argN6, "RECORDEX");
} else {
    argN6 = argN;
}
```

### Instruction Import Mechanism

`Import_Instruction` in `SchDataSerializerParam` calls `GetNextLine()` which:
1. Reads 4 bytes for the block header (length + mode)
2. For binary mode, performs lookahead to detect embedded objects
3. Reads the payload
4. For mode 0: parses the pipe-delimited parameters into a dictionary
5. For mode 1: loads the raw bytes into a binary buffer

Then the `RECORD` value is read from the parsed parameters as a byte.

**Source:** `SchDataSerializerParam.GetNextLine()` and `Import_Instruction()`

---

## 5. SchDoc Loading Pipeline

The SchDoc loading pipeline is orchestrated by `SchDataImporterBaseV5.Run()` calling
into `SchDataImporterDocumentV5` methods:

### Step 1: Initialize

```csharp
// SchDataImporterExporterBase.Run():
document.SetDataFormat(FileFormatUtils.GetDataFormatByParameters(
    serializer.GetSerializerType(), GetVersion()));
```

### Step 2: ImportBaseWarehouse

**Method:** `SchDataImporterDocumentV5.ReadBaseWarehouse()`

1. Open the `"FileHeader"` stream at the root storage level
2. Read the first record (the header record):
   - `RECORD` (byte, always 0 for the header)
   - `HEADER` (string, the file format identification string)
   - `Weight` (int, total number of object records that follow)
   - `MinorVersion` (int, file minor version, currently 13 for SchDoc)
   - `UniqueID` (string, document unique identifier)
3. Read the **font table** (embedded in the first record's parameters):
   - `FontIdCount` (short, number of fonts)
   - For each font `i` (1-based): `Size{i}`, `Rotation{i}`, `Underline{i}`,
     `Italic{i}`, `Bold{i}`, `StrikeOut{i}`, `FontName{i}`
4. Loop `Weight` times, reading each object record:
   - Read `RECORD` byte. If 254, read `RECORDEX` int for extended type.
   - Skip ignored objects (specific object IDs are ignored).
   - If the record code matches the document's binary code (31 for Sheet), import
     it into the document object itself rather than creating a new object.
   - Otherwise, create the object via `FileFormatUtils.CreateObjectByBinaryCode()`.
   - Import the object's fields via `FileFormat.ImportFromFile()`.
   - Call `UpdateOwner(BaseWarehouse)` to link the object into its parent container
     using the `OwnerIndex` field.
   - Add to BaseWarehouse.
5. Close the stream.

**Important:** The document/sheet object (binary code 31) is the very first object
record (after the header). All other objects reference it or each other via OwnerIndex.

### Step 3: ImportExtendedWarehouse (Storage Stream)

**Method:** `SchDataImporterBaseV5.ReadExtendedWarehouse()`

1. Open the `"Storage"` stream
2. Read header record: `RECORD`, `HEADER` ("Icon storage"), `Weight` (count)
3. Loop `Weight` times:
   - Read `BINARY` instruction (marker byte, should be 0xD0 = 208)
   - Create `SchDataEmbeddedObject` and call `ImportFromFile(serializer)`:
     - Read `Name` (dynamic string)
     - Read `Data` (binary blob, zlib-compressed)
   - Add to ExtendedWarehouse
4. Close stream

Then `ProcessImportedExtendedWarehouse()` matches embedded objects to `SchDataImage`
records in the BaseWarehouse by filename, populating the image cache.

### Step 4: ImportAdditionalWarehouse

**Method:** `SchDataImporterDocumentV5.ReadAdditionalWareHouse()`

1. Open the `"Additional"` stream (if it exists)
2. Read header: `RECORD`, `HEADER`, `Weight`
3. Loop `Weight` times, reading object records similar to BaseWarehouse
4. For each object, resolve its owner using OwnerIndex:
   - If `OwnerIndexAdditionalList` is true, the index is into AdditionalWarehouse
   - Otherwise, the index is into BaseWarehouse
5. Call `UpdateOwner()` to link into the container hierarchy

### Step 5: ImportDefinitionWarehouse

**Method:** `SchDataImporterDocumentV5.ImportDefinitionWarehouse()`

1. Open the `"ObjectDefinitions"` stream (if it exists)
2. Read header: `RECORD`, `HEADER`, `Weight`
3. Loop `Weight` times reading object definition records
4. Add to `objectDefinitionWarehouse`

### Step 6: ImportReuseBlockInfo

**Method:** `SchDataImporterDocumentV5.ImportReuseBlockInfo()`

1. Open the `"ReuseBlockInfos"` stream (if it exists)
2. Same pattern as ImportDefinitionWarehouse

### Step 7: ImportFilesWarehouse

Read the `"Files"` stream if present.

### Step 8: UpdateAfterImport

**Method:** `SchDataImporterDocumentV5.UpdateAfterImport()`

1. `UpdateAssociatedPartsAndCavitiesAfterImport()` - process harness parts
2. `UpdatePhysicalModelsAfterImport()` - process physical model parameters
3. `UpdateObjectsDefinitionsAfterImport()` - add definitions to document
4. `UpdateReuseBlockInfoAfterImport()` - add reuse block info
5. `UpdateDocumentAfterImport()` - fire event, clear warehouse

### Step 9: Extended Streams (SchDoc-specific)

The `SchDataImporterSheetV5` also reads:
- `"ReuseBlocks"` stream via `ReadBinaryBlocksData()`
- `"ReuseBlocksV2"` stream via `ReadBinaryBlocksData()`
- `"HarnessConnectionPointConnector"` stream

### Step 10: FinalizeForLoading

```csharp
document.MoveSpecialObjectsToTop();
```

---

## 6. SchLib Loading Pipeline

The SchLib loading pipeline is in `SchDataImporterLibraryV5`. It differs significantly
from SchDoc because components are stored in separate per-component sub-storages.

### Step 1: ImportBaseWarehouse

**Method:** `SchDataImporterLibraryV5.ImportBaseWarehouse()`

Calls `ReadBaseWarehouse()` then `ProcessImportedBaseWarehouse()`.

#### ReadBaseWarehouse

1. **Import the library header** via `ImportLibrary(out weight)`:
   - Open `"FileHeader"` stream at root
   - Read header record: `RECORD=0`, `HEADER`, `Weight`, `MinorVersion`, `UniqueID`
   - Read the font table (embedded in the header, same as SchDoc)
   - Import library object fields via `FileFormat.ImportFromFile(serializer, library)`
   - Close stream

2. **Load Section Keys** via `componentSectionKeyList.Load(serializer, "", "SectionKeys")`:
   - Open the `"SectionKeys"` stream (if it exists)
   - Read `RECORD`, `KeyCount`
   - For each key `i`: read `LibRef{i}` (full name) and `SectionKey{i}` (truncated key)
   - This maps long component names to short CFB storage names (max 31 chars)

3. **Iterate through component storages** using `FindFirstStream("Data")`:
   - The serializer enumerates all top-level CFB sub-storages
   - For each storage that contains a `"Data"` stream:
     a. Read the first record -- this is the Component record (binary code 1)
     b. Reset component reference location and orientation
     c. Add component to library via `library.AddComponent()`
     d. Read subsequent records in a loop until `RECORD=0` (end-of-component marker):
        - Create object, import fields, adjust OwnerIndex
        - Link into parent via `UpdateOwner(baseWarehouse)`
     e. Continue to next storage via `FindNextStream()`
   - Close the find operation via `FindCloseStream()`

**Key difference from SchDoc:** In SchLib, the OwnerIndex for child objects is
**relative within the component section**, not absolute. The code adjusts:
```csharp
schDataObject.SetOwnerIndexForSave(
    schDataObject.GetOwnerIndexForSave() + num2);  // num2 = base offset of this component
```

### Step 2: ImportExtendedWarehouse

**Method:** `SchDataImporterLibraryV5.ImportExtendedWarehouse()`

1. `ReadExtendedWarehouse()` - reads the global `"Storage"` stream (same format as SchDoc)
2. `ProcessImportedExtendedWarehouse()` - matches images to SchDataImage objects
3. **ReadAndProcessPinsExtendedData()** - the 9 pin sidecar streams

#### The 9 Pin Sidecar Streams

For each component in the baseWarehouse, the importer reads up to 9 sidecar streams.
Each stream is located in the component's CFB sub-storage (using the component's
SectionKey as the storage name).

The streams are read in this exact order:

| # | Stream Name | Data Type | Description |
|---|-------------|-----------|-------------|
| 1 | `PinFrac` | Binary (3x int32) | Pin fractional coordinate adjustments (X, Y, Length) |
| 2 | `PinDesc` | ASCII string | Long pin descriptions (overflow beyond 254 chars) |
| 3 | `PinMiscData` | Unicode string | Misc data (PairSwapID parameter) |
| 4 | `PinTextData` | Binary struct | Custom pin text display settings (position, font, color) |
| 5 | `PinWideText` | Unicode string | Wide/Unicode text overrides (Desc, Name, Desig, SwapId, etc.) |
| 6 | `PinSymbolLineWidth` | Unicode string | Symbol line width parameter |
| 7 | `PinPackageLength` | Unicode string | Pin package length parameter |
| 8 | `PinPropagationDelay` | Unicode string | Pin propagation delay parameter |
| 9 | `PinFunctionData` | Unicode string | Pin function data (selected/defined functions) |

Each sidecar stream has the same outer structure:

```
[Header record: RECORD=0, HEADER="<StreamName>", Weight=<count>]
[For each entry:]
  [BINARY=0xD0 (208)]
  [SchDataEmbeddedObject: Name=<pin_index>, Data=<payload>]
```

The `Name` field of each `SchDataEmbeddedObject` is the **pin index** (0-based,
as a decimal string). This maps the sidecar data to the corresponding pin in the
ordered pin list.

##### PinFrac Data Format

```
[4 bytes: locationX fractional adjustment (int32)]
[4 bytes: locationY fractional adjustment (int32)]
[4 bytes: length fractional adjustment (int32)]
```

These are added to the pin's existing coordinates to provide sub-unit precision.

##### PinTextData Binary Format

Two consecutive text data structures (one for Name, one for Designator):

```
[1 byte: flags]
  bit 0: positionMode (0=default, 1=custom)
  bit 1: customRotationAnchor (0=pin, 1=component) - only if bit0=1
  bits 2-3: customRotationRelative (TRotationBy90 enum) - only if bit0=1
  bit 4: fontMode (0=default, 1=custom)

[if positionMode == custom:]
  [4 bytes: customMargin (int32)]

[if fontMode == custom:]
  [2 bytes: customFontID (int16, file-format font ID, needs translation)]
  [4 bytes: customColor (uint32)]
```

##### PinWideText String Format

Unicode-encoded parameter string with key=value pairs:
```
Desc=<description>|Name=<name>|Desig=<designator>|SwapId=<swapId>|SwapIDPart=<swapIdPart>|DefValue=<defaultValue>
```

Only written when the pin text contains non-ANSI characters or exceeds 254 characters.

##### PinMiscData String Format

Unicode-encoded parameter string: `PairSwapID=<value>`

##### PinFunctionData String Format

Unicode-encoded parameter string:
```
PinSelectedFunctionsCount=<n>|PinSelectedFunction1=<val1>|...|PinDefinedFunctionsCount=<m>|PinDefinedFunction1=<val1>|...
```

### Step 3: ImportAdditionalWarehouse

**Method:** `SchDataImporterLibraryV5.ReadAdditionalWareHouse()`

1. Check for `"LibAdditional"` stream at root (if absent, skip)
2. Open `"LibAdditional"` stream and read header: `RECORD`, `HEADER`, `Weight`
3. For each component in baseWarehouse:
   - Get the component's SectionKey
   - Check if `<SectionKey>/Additional` stream exists
   - Open that per-component stream
   - Read records until `RECORD=0`:
     - Resolve OwnerIndex relative to the component's base offset
     - Link via `UpdateOwner()`
   - Close per-component stream
4. Close `"LibAdditional"` stream

### Step 4: UpdateDocumentAfterImport

Fires the after-import event and clears internal warehouse references.

### Loading a Single Component

`SchDataImporterLibraryV5.Run(string libraryReference)` loads just one component:

1. Import the library header and section keys
2. Get the SectionKey for the requested `libraryReference`
3. Resolve aliases via `GetLibraryReferenceByAliasName()`
4. Open `<SectionKey>/Data` stream directly
5. Read the component and its children
6. Still reads ExtendedWarehouse and AdditionalWarehouse

---

## 7. Object Hierarchy and OWNERINDEX Linking

### How OwnerIndex Works

Every object record contains an `OwnerIndex` field. This is a **0-based index into
the flat list (warehouse)** where the parent/owner object resides.

```
Record 0: Sheet (RECORD=31)         <- OwnerIndex=-1 (no parent, this IS the document)
Record 1: Component (RECORD=1)      <- OwnerIndex=0 (owned by Sheet at index 0)
Record 2: Pin (RECORD=2)            <- OwnerIndex=1 (owned by Component at index 1)
Record 3: Pin (RECORD=2)            <- OwnerIndex=1 (owned by Component at index 1)
Record 4: Parameter (RECORD=41)     <- OwnerIndex=1 (owned by Component at index 1)
Record 5: Wire (RECORD=27)          <- OwnerIndex=0 (owned by Sheet at index 0)
...
```

### The UpdateOwner Mechanism

When a record is loaded, `UpdateOwner(warehouse)` is called:

```csharp
// SchDataObject.UpdateOwner():
public virtual void UpdateOwner(ISchDataObjectList argList) {
    (GetOwnerFromList(argList) as ISchDataContainer)?.AddEx(this);
}

// SchDataObject.GetOwnerFromList():
protected ISchDataObject GetOwnerFromList(ISchDataObjectList list) {
    if (ownerIndexForSave < 0 || ownerIndexForSave >= list.Count())
        return null;
    return list.Get(ownerIndexForSave);
}
```

This resolves the OwnerIndex to the actual parent object, then adds the current object
as a child of that parent's container.

### Additional Fields for Hierarchy

| Field | Description |
|-------|-------------|
| `OwnerIndex` | Index into warehouse list pointing to parent object |
| `OwnerPartId` | Which part of a multi-part component this belongs to (-1 = all parts) |
| `OwnerPartDisplayMode` | Which display mode this belongs to |
| `OwnerIndexAdditionalList` | If true, OwnerIndex refers to AdditionalWarehouse instead of BaseWarehouse |
| `IndexInSheet` | Position index within the parent container |
| `IsNotAccesible` | Whether the object is accessible (inverted boolean) |

These are imported/exported by `ImportDataObject()`/`ExportDataObject()` in
`FileFormatV5`:

```csharp
private void ImportDataObject(ISchDataSerializer argSerializer, ISchDataObject argObject) {
    int argN = -1;
    argSerializer.Import_LongInt(ref argN, "OwnerIndex");
    argObject.SetOwnerIndexForSave(argN);
    bool argN2 = false;
    argSerializer.Import_Boolean(ref argN2, "IsNotAccesible");
    argObject.SetIsAccessible(!argN2);
    bool argN3 = true;
    argSerializer.Import_Boolean(ref argN3, "OwnerIndexAdditionalList");
    argObject.SetOwnerIndexForSaveAdditionalList(argN3);
    int argN4 = -1;
    argSerializer.Import_LongInt(ref argN4, "IndexInSheet");
    argObject.SetIndexInSheetForSave(argN4);
    // ...
}
```

**Source:** `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatV5.cs` (around line 5038)

### SchLib OwnerIndex Adjustment

In SchLib, each component's children have OwnerIndex values that are **relative to the
start of that component's section**. During loading, the code adjusts:

```csharp
// SchDataImporterLibraryV5.ReadBaseWarehouse():
schDataObject.SetOwnerIndexForSave(
    schDataObject.GetOwnerIndexForSave() + num2);  // num2 = component's position in warehouse
```

This converts the relative index to an absolute warehouse index.

---

## 8. Font Table Format

### Location

The font table is embedded within the **first record of the FileHeader stream** (the
document/library header record). It is part of that record's parameters.

For both SchDoc and SchLib, the font table is read/written during the import/export of
the document or library object itself, within `FileFormatV5`.

### V5 Font Table Structure

The font table is stored as parameter key=value pairs within the header record:

```
FontIdCount=<N>
Size1=<int>
Rotation1=<int>
Underline1=<T|F>
Italic1=<T|F>
Bold1=<T|F>
StrikeOut1=<T|F>
FontName1=<string>
Size2=<int>
Rotation2=<int>
...
```

`FontIdCount` is the total number of font entries. Font indices are 1-based.

### Export Process

```csharp
// FileFormatV5.ExportFontTable():
argSerializer.Export_ShortInt(instance.FlagedFontIdCount(), "FontIdCount");
int num = 1;
int fontCount = instance.GetFontCount();
for (int i = 1; i <= fontCount; i++) {
    if (instance.GetSaveFlag(i)) {
        string text = num.ToString();
        argSerializer.Export_ShortInt(instance.GetSize(i), "Size" + text);
        argSerializer.Export_ShortInt(instance.GetRotation(i), "Rotation" + text);
        argSerializer.Export_Boolean(instance.GetUnderLine(i), "Underline" + text);
        argSerializer.Export_Boolean(instance.GetItalic(i), "Italic" + text);
        argSerializer.Export_Boolean(instance.GetBold(i), "Bold" + text);
        argSerializer.Export_Boolean(instance.GetStrikeOut(i), "StrikeOut" + text);
        argSerializer.Export_String(instance.GetFontName(i), "FontName" + text);
        num++;
    }
}
```

### Import Process

```csharp
// FileFormatV5.ImportFontTable():
int argN = 0;
argSerializer.Import_ShortInt(ref argN, "FontIdCount");
for (int i = 1; i <= argN; i++) {
    string text = i.ToString();
    // Read Size, Rotation, Underline, Italic, Bold, StrikeOut, FontName
    // Default FontName to "Times New Roman" if empty
    instance.SetFontIDInTranslator(i,
        instance.GetFontID(size, rotation, underline, italic, bold, strikeOut, fontName),
        fontIdTranslator);
}
```

### Font ID Translation

Font IDs in the file are **file-local indices** (1-based, sequential as stored in the
font table). The `FontIdTranslator` maps these file-local IDs to the runtime font
manager's internal IDs.

When reading a `FontID` field from any object record:
```csharp
// SchDataSerializer.Import_FontID():
ReadShort(out var value, argName);  // Read file-local font ID
argN = instance.GetFontIDInTranslator(value, fontIdTranslator);  // Translate to runtime ID
```

When writing a `FontID` field:
```csharp
// SchDataSerializer.Export_FontID():
int fontIDOutTranslator = instance.GetFontIDOutTranslator(argN);  // Translate from runtime to file-local
WriteShort(Convert.ToInt16(fontIDOutTranslator), argName);
```

**Source:**
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatV5.cs` (lines 5236-5301)
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FontManager/SchDataFontManager.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Serialization/SchDataSerializer.cs` (lines 427, 763)

---

## 9. FileHeader Format

### SchDoc FileHeader

The FileHeader stream for a SchDoc contains a single "super-record" that includes:

1. **Header record** (the first block in the stream):
   - `RECORD=0` (byte)
   - `HEADER=<format_string>` (string)
   - `Weight=<total_object_count>` (int)
   - `MinorVersion=<version>` (int, currently 13 for SchDoc)
   - `UniqueID=<guid>` (string)

2. **Font table** (embedded in the same record as part of the document object's exported
   fields - see section 8)

3. **Object records** (subsequent blocks in the stream):
   - The first object is the Sheet/Document itself (RECORD=31)
   - All remaining objects follow in order, each with their OwnerIndex

The header record and all the document/sheet parameters (like `FontIdCount`, `SizeN`,
etc.) are actually written as **one combined parameter block** by the serializer.
The Sheet record (RECORD=31) is part of this same logical sequence: the
`FileFormat.ImportFromFile()` call reads the document-level parameters including the
font table.

### SchLib FileHeader

The FileHeader for a SchLib is more complex. It contains:

1. **Header record**:
   - `RECORD=0` (byte)
   - `HEADER=<format_string>` (string)
   - `Weight=<total_object_count>` (int)
   - `MinorVersion=<version>` (int, currently 9 for SchLib)
   - `UniqueID=<guid>` (string)

2. **Library object fields** (part of the same record, exported via
   `FileFormat.ExportToFile(serializer, library)`):
   - Font table
   - Library-level parameters

3. **Component index** (additional fields in the same record):
   - `CompCount=<N>` (int, number of components)
   - For each component `i` (0-based):
     - `LibRef{i}=<component_name>` (dynamic string)
     - `CompDescr{i}=<description>` (string)
     - `PartCount{i}=<part_count>` (short)
     - `AliasCount{i}=<alias_count>` (short)
     - For each alias `j`:
       - `Comp{i}Alias{j}=<alias_name>` (dynamic string)

This component index enables rapid enumeration of library contents without having to
open each component's Data stream.

**Source:** `SchDataExporterLibraryV5.WriteBaseWarehouseHeader()` (lines 113-161)

---

## 10. The Alias and Redirection System

### Purpose

SchLib supports component aliases -- alternative names that resolve to the same
component. This allows a single component definition to be referenced by multiple names.

### SectionKeys

CFB storage names are limited to 31 characters. Component names can be longer than
this limit. The `SectionKeys` stream provides a mapping from full component names to
truncated/disambiguated CFB storage keys.

```
SectionKeys stream format:
  RECORD=0
  KeyCount=<N>
  LibRef0=<full_component_name>
  SectionKey0=<truncated_cfb_key>
  LibRef1=<full_component_name>
  SectionKey1=<truncated_cfb_key>
  ...
```

The `SchDataComponentSectionKeyList` class manages this mapping:

```csharp
// SchDataComponentSectionKeyList.GetKey():
public string GetKey(string name) {
    if (!nameKeyMapList.ContainsKey(name))
        return name;  // Short names don't need mapping
    return nameKeyMapList[name].Item2;  // Return the truncated key
}
```

Key truncation/disambiguation:
- Names <= 31 chars: used as-is
- Names > 31 chars: truncated to 31 chars, with numeric suffix if collision occurs
- Characters `/\:*?"<>|!` are replaced with `_`

**Source:** `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport.Types/SchDataComponentSectionKeyList.cs`

### Redirection Streams

When a component has aliases, each alias gets its own CFB sub-storage containing only
a `"Redirection"` stream. This stream contains a single record:

```
RECORD=0
SectionName=<canonical_component_name>
```

This tells the loader to look up the actual component data under the canonical name's
storage.

### Alias Resolution During Load

When loading a single component by name (`Run(string libraryReference)`):

```csharp
// SchDataImporterLibraryV5.GetLibraryReferenceByAliasName():
string key = componentSectionKeyList.GetKey(aliasName);

// Check for Redirection stream
if (serializer.StreamExists(section, "Redirection")) {
    serializer.StartStream(section, "Redirection");
    serializer.Import_Instruction(ref argN, "RECORD");
    string sectionName = string.Empty;
    serializer.Import_DynamicString(ref sectionName, "SectionName");
    return sectionName;  // Return the canonical name
}

// Check if this section has a Data stream (it's a real component)
if (serializer.StreamExists(section, "Data")) {
    return aliasName;  // It's a canonical component
}

// Search through the FileHeader's component index for alias matches
serializer.StartStream("", "FileHeader");
serializer.Import_Instruction(ref argN, "RECORD");
int compCount = 0;
serializer.Import_LongInt(ref compCount, "CompCount");
for (int i = 0; i < compCount; i++) {
    string libRef = ...;  // Read LibRef{i}
    if (libRef == aliasName) return libRef;
    int aliasCount = ...;  // Read AliasCount{i}
    for (int j = 0; j < aliasCount; j++) {
        string alias = ...;  // Read Comp{i}Alias{j}
        if (alias == aliasName) return libRef;
    }
}
```

### Alias Writing During Save

```csharp
// SchDataExporterLibraryV5.WriteBaseWarehouseData():
ISchDataAliasList aliasList = schDataComponent.GetAliasList();
int count = aliasList.GetCount();
for (int j = 0; j < count; j++) {
    // Write a Redirection stream for each alias
    serializer.StartStream(
        componentSectionKeyList.GetKey(aliasList.GetValue(j)),
        "Redirection");
    serializer.Export_Byte(0, "RECORD");
    serializer.Export_DynamicString(
        schDataComponent.GetLibReference(), "SectionName");
}
```

---

## 11. Export (Save) Pipeline

### SchDoc Save Pipeline

Orchestrated by `SchDataExporterBaseV5.Run()`:

```
1. InitializeForSaving()          - Update font IDs for rotation
2. UpdateObjectBeforeExport()     - Fire pre-export event
3. FillBaseAndAdditionalWarehouses() - Flatten hierarchy into ordered lists
4. FillExtendedWarehouse()        - Collect embedded images
5. FillDefinitionWarehouse()      - Collect object definitions
6. FillReuseBlockInfoWarehouse()  - Collect reuse block info
7. FillFilesWarehouse()           - Collect attached files
8. FixBaseWarehouse()             - Fix any issues (e.g., duplicated LibRefs)
9. WriteBaseWarehouse()           - Write FileHeader stream
10. WriteExtendedWarehouse()      - Write Storage stream + sidecar data
11. WriteAdditionalWarehouse()    - Write Additional stream
12. WriteDefinitionWarehouse()    - Write ObjectDefinitions stream
13. WriteReuseBlockInfoWarehouse()- Write ReuseBlockInfos stream
14. WriteFilesWarehouseData()     - Write Files stream
15. UpdateObjectAfterExport()     - Fire post-export event
16. Clear all warehouses
17. FinalizeForSaving()           - Restore font IDs
```

#### WriteBaseWarehouse (SchDoc)

```csharp
// SchDataExporterBaseV5.WriteBaseWarehouse():
serializer.StartStream("", "FileHeader");
serializer.Export_Instruction(0, "RECORD");
serializer.Export_String(GetHeader(), "HEADER");
serializer.Export_LongInt(num, "Weight");          // Total object count
serializer.Export_LongInt(CurrentMinorVersion, "MinorVersion");
serializer.Export_String(MainDataObject.GetUniqueId(), "UniqueID");

for (int i = 0; i < num; i++) {
    ISchDataObject obj = BaseWarehouse.Get(i);
    int binaryCode = SchDataUtils.GetBinaryCodeForObject(obj);
    if (binaryCode > 255) {
        serializer.Export_Instruction(254, "RECORD");
        serializer.Export_InstructionEx(binaryCode, "RECORDEX");
    } else {
        serializer.Export_Instruction((byte)binaryCode, "RECORD");
    }
    UpdateBaseWarehouseOwnerIndex(obj);
    FileFormat.ExportToFile(serializer, obj);
}
serializer.EndStream();
```

#### WriteExtendedWarehouse (Storage)

```csharp
// SchDataExporterBaseV5.WriteExtendedWarehouseData():
serializer.StartStream("", "Storage");
serializer.Export_Instruction(0, "RECORD");
serializer.Export_String("Icon storage", "HEADER");
serializer.Export_LongInt(count, "Weight");
for (int i = 0; i < count; i++) {
    serializer.Export_Instruction(208, "BINARY");  // 0xD0 marker
    embeddedObject.ExportToFile(serializer);
}
serializer.Export_Instruction(0, "RECORD");  // End marker
serializer.EndStream();
```

### SchLib Save Pipeline

The SchLib exporter (`SchDataExporterLibraryV5`) overrides several methods:

#### WriteBaseWarehouse (SchLib)

This is split into two parts:

1. **WriteBaseWarehouseHeader()** - writes the FileHeader stream with:
   - Library header record
   - Component index (CompCount, LibRef, CompDescr, PartCount, AliasCount, aliases)

2. **WriteBaseWarehouseData()** - writes per-component Data streams:
   - For each component in BaseWarehouse:
     a. Write `RECORD=0` end marker to close previous component
     b. Write alias Redirection streams
     c. Open `<SectionKey>/Data` stream
     d. Write the Component record (RECORD=1)
     e. Write all child objects (pins as BINARY, others as RECORD)
     f. Adjust OwnerIndex to be relative to component start
   - Write final `RECORD=0` end marker
   - Write SectionKeys stream

**Important note about pins:** In the library Data stream, pin records are written
with `Export_Instruction(b, "BINARY")` instead of `"RECORD"`. This causes them to be
written in binary mode rather than parameter mode. All other child objects use `"RECORD"`.

```csharp
if (schDataContainer is ISchDataPin schDataPin) {
    val?.Add(schDataPin);
    serializer.Export_Instruction(b, "BINARY");  // Binary mode for pins!
} else {
    serializer.Export_Instruction(b, "RECORD");
}
```

#### WriteExtendedWarehouse (SchLib)

1. Write global Storage stream (same as SchDoc)
2. **PrepareAndWritePinsExtendedData()** - writes the 9 pin sidecar streams:
   - For each component, collect pin data into 9 lists
   - Write each non-empty list as a sidecar stream:
     ```
     serializer.StartStream(sectionKey, streamName);
     serializer.Export_Instruction(0, "RECORD");
     serializer.Export_String(streamName, "HEADER");
     serializer.Export_LongInt(list.Count, "Weight");
     for each entry:
       serializer.Export_Instruction(208, "BINARY");
       embeddedObject.ExportToFile(serializer);
     serializer.Export_Instruction(0, "RECORD");  // End marker
     serializer.EndStream();
     ```

#### WriteAdditionalWarehouse (SchLib)

1. **WriteAdditionalWarehouseHeader()** - writes `"LibAdditional"` header stream
2. **WriteAdditionalWarehouseData()** - writes per-component Additional streams:
   - For each component, open `<SectionKey>/Additional` stream
   - Write all Additional records belonging to that component
   - Adjust OwnerIndex to be relative

---

## 12. Embedded Object Container Format

### SchDataEmbeddedObject

The `SchDataEmbeddedObject` class is a simple container with a name and binary data.

```csharp
// SchDataEmbeddedObject:
public class SchDataEmbeddedObject {
    private byte[] data;
    private string name;

    public void ExportToFile(ISchDataSerializer serializer) {
        serializer.Export_DynamicString(name, "Name");
        serializer.Export_Binary(data, "Data");
    }

    public void ImportFromFile(ISchDataSerializer serializer) {
        serializer.Import_DynamicString(ref name, "Name");
        serializer.Import_Binary(out data, "Data");
    }
}
```

### Binary Data Encoding

In `SchDataSerializerParam` (the binary/CFB serializer), `Export_Binary` and
`Import_Binary` handle Zlib compression:

**Export (Write):**
```csharp
// WriteBinary in parameter mode (mode == 0):
MemoryStream compressed = ZlibCompress(data);
string hexString = BytesToHexString(compressed.ToArray());
WriteInt(compressed.Length, name + "_Len");
SetParameter(name, hexString);

// WriteBinary in binary mode (mode == 1):
MemoryStream compressed = ZlibCompress(data);
WriteInt(compressed.Length, name + "_Len");
WriteData(compressed.ToArray(), compressed.Length, name);
```

**Import (Read):**
```csharp
// ReadBinary in parameter mode (mode == 0):
ReadInt(out _, name + "_Len");
string hexValue = GetParameter(name);
byte[] compressed = HexStringToBytes(hexValue);
data = ZlibDecompress(compressed);

// ReadBinary in binary mode (mode == 1):
// Direct binary read from the buffer
```

### The 0xD0 Marker

Before each embedded object in a stream, a marker byte `0xD0` (208) is written as a
`BINARY` instruction:

```csharp
serializer.Export_Instruction(208, "BINARY");  // 0xD0 = binary mode marker
embeddedObject.ExportToFile(serializer);
```

When reading, the marker is checked:
```csharp
serializer.Import_Instruction(ref argN, "BINARY");
if (argN == 208) {
    SchDataEmbeddedObject obj = new SchDataEmbeddedObject();
    obj.ImportFromFile(serializer);
    // ...
}
```

### On-Disk Layout of Binary Block with Embedded Object

In the actual CFB stream bytes:

```
[4 bytes: block_length | 0x01000000]     <- binary mode block header
[1 byte: 0xD0]                           <- marker byte (the "RECORD" field in binary mode)
[1 byte: name_length]                    <- length of the Name string
[name_length bytes: Name]                <- the name (e.g., "0", "ReuseBlocks", image filename)
[4 bytes: compressed_data_length]        <- the Data_Len field
[compressed_data_length bytes: zlib data] <- zlib-compressed payload
```

**Source:** `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.Objects/SchDataEmbeddedObject.cs`

---

## 13. Binary Code to TObjectId Mapping

This is the complete mapping from binary record codes to TObjectId enum values, as
defined in `FileFormatUtils.CreateObjectByBinaryCodeInternal()`:

| Binary Code | TObjectId | Class |
|------------|-----------|-------|
| 1 | `eSchComponent` | `SchDataComponent` |
| 2 | `ePin` | `SchDataPin` |
| 3 | `eSymbol` | `SchDataSymbol` |
| 4 | `eLabel` | `SchDataLabel` |
| 5 | `eBezier` | `SchDataBezier` |
| 6 | `ePolyline` | `SchDataPolyline` |
| 7 | `ePolygon` | `SchDataPolygon` |
| 8 | `eEllipse` | `SchDataEllipse` |
| 9 | `ePie` | `SchDataPie` |
| 10 | `eRoundRectangle` | `SchDataRoundRectangle` |
| 11 | `eEllipticalArc` | `SchDataEllipticalArc` |
| 12 | `eArc` | `SchDataArc` |
| 13 | `eLine` | `SchDataLine` |
| 14 | `eRectangle` | `SchDataRectangle` |
| 15 | `eSheetSymbol` | `SchDataSheetSymbol` |
| 16 | `eSheetEntry` | `SchDataSheetEntry` |
| 17 | `ePowerObject` | `SchDataPower` |
| 18 | `ePort` | `SchDataPort` |
| 22 | `eNoERC` | `SchDataNoERC` |
| 23 | `eErrorMarker` | `SchDataErrorMarker` |
| 25 | `eNetLabel` | `SchDataNetLabel` |
| 26 | `eBus` | `SchDataBus` |
| 27 | `eWire` | `SchDataWire` |
| 28 | `eTextFrame` | `SchDataTextFrame` |
| 29 | `eJunction` | `SchDataJunction` |
| 30 | `eImage` | `SchDataImage` |
| 31 | `eSheet` | (document object, not created; imported into the document) |
| 32 | `eSheetName` | `SchDataSheetName` |
| 33 | `eSheetFileName` | `SchDataSheetFileName` |
| 34 | `eDesignator` | `SchDataDesignator` |
| 37 | `eBusEntry` | `SchDataBusEntry` |
| 39 | `eTemplate` | `SchDataTemplate` |
| 41 | `eParameter` | `SchDataParameter` |
| 42 | `eSchComponent` | `SchDataComponent` (alternate code) |
| 43 | `eParameterSet` | `SchDataParameterSet` |
| 44 | `eImplementationsList` | `SchDataImplementationList` |
| 45 | `eImplementation` | `SchDataImplementation` |
| 46 | `eImplementationMap` | `SchDataImplementationMap` |
| 47 | `eMapDefiner` | `SchDataMapDefiner` |
| 48 | `eParameterList` | `SchDataParameterList` |
| 106 | `eHarnessComponent` | `SchDataHarnessComponent` |
| 107 | `eHarnessWire` | `SchDataHarnessWire` |
| 108 | `eHarnessSplice` | `SchDataHarnessSplice` |
| 109 | `eHarnessLayoutLabel` | `SchDataHarnessLayoutLabel` |
| 110 | `eHarnessLayoutConnectionPoint` | `SchDataHarnessLayoutConnectionPoint` |
| 111 | `eHarnessBundle` | `SchDataHarnessBundle` |
| 112 | `eHarnessLogicalSignal` | `SchDataHarnessLogicalSignal` |
| 113 | `eHarnessPin` | `SchDataHarnessPin` |
| 114 | `eHarnessWireLabel` | `SchDataHarnessWireLabel` |
| 115 | `eHarnessWireData` | `SchDataHarnessWireData` |
| 116 | `eHarnessSpliceData` | `SchDataHarnessSpliceData` |
| 117 | `eHarnessShield` | `SchDataHarnessShield` |
| 118 | `eHarnessTwist` | `SchDataHarnessTwist` |
| 119 | `eHarnessNoConnect` | `SchDataHarnessNoConnect` |
| 120 | `eHarnessNoConnectData` | `SchDataHarnessNoConnectData` |
| 121 | `eHarnessShieldData` | `SchDataHarnessShieldData` |
| 122 | `eHarnessTwistData` | `SchDataHarnessTwistData` |
| 123 | `eHarnessCable` | `SchDataHarnessCable` |
| 124 | `eHarnessCableData` | `SchDataHarnessCableData` |
| 125 | `eHarnessAssociatedParts` | `SchDataHarnessAssociatedParts` |
| 126 | `eLineView` | `SchDataLineView` |
| 128 | `eHarnessCovering` | `SchDataHarnessLayoutCovering` |
| 129 | `eObjectDefinition` | `SchDataObjectDefinition` |
| 130 | `eHarnessWireBreak` | `SchDataHarnessWireBreak` |
| 131 | `eAssociatedObjects` | `SchDataAssociatedObjects` |
| 132 | `eElectronicsSystemDesignDocument` | `SchDataElectronicsSystemDesignDocument` |
| 133 | `eFunctionalBlock` | `SchDataFunctionalBlock` |
| 134 | `eFunctionalConnectionLine` | `SchDataFunctionalConnectionLine` |
| 135 | `eFunctionalTextFrame` | `SchDataFunctionalTextFrame` |
| 136 | `eSchematicBlock` | `SchDataSchematicBlock` |
| 137 | `eReuseSheetSymbol` | `SchDataReuseSheetSymbol` |
| 138 | `eReuseBlockImplementationInfo` | `SchDataReuseBlockImplementationInfo` |
| 209 | `eNote` | `SchDataNote` |
| 210 | `eProbe` | `SchDataProbe` |
| 211 | `eCompileMask` | `SchDataCompileMask` |
| 215 | `eHarnessConnector` | `SchDataHarnessConnector` |
| 216 | `eHarnessEntry` | `SchDataHarnessEntry` |
| 217 | `eHarnessConnectorType` | `SchDataHarnessConnectorType` |
| 218 | `eSignalHarness` | `SchDataSignalHarness` |
| 220 | `eHighLevelCodeSymbol` | `SchDataSheetSymbol` (reuses SheetSymbol class) |
| 221 | `eSheetEntry` | `SchDataSheetEntry` (for high-level code) |
| 222 | `eSheetName` | `SchDataSheetName` (for high-level code) |
| 223 | `eSheetFileName` | `SchDataSheetFileName` (for high-level code) |
| 225 | `eBlanket` | `SchDataBlanket` |
| 226 | `eHyperlink` | `SchDataHyperlink` |
| 240 | `eRichTextDocument` | `SchDataRichTextDocument` |
| 241 | `eRTFLink` | `SchDataRTFLink` |
| 254 | (extension marker) | Indicates RECORDEX follows with extended code |

Special codes:
- **0** = End-of-component marker (SchLib) / no object
- **31** = Sheet/Document object (SchDoc) -- imported into the existing document object,
  not created as a new object
- **254** = Extended record marker -- actual type in subsequent RECORDEX field

**Source:** `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatUtils.cs` (lines 249-444)

---

## Appendix A: CFB Name Sanitization

CFB storage/stream names cannot contain certain characters and are limited to 31
characters. The serializer sanitizes names:

```csharp
// SchDataSerializerParam.FixName():
// Replace invalid chars with '_'
char[] invalidNameChars = { '/', '\\', ':', '*', '?', '"', '<', '>', '|', '!' };

// Truncate to 31 characters (actually 30 with room for null)
if (name.Length > 31)
    return name.Substring(0, 30);
```

This is why the SectionKeys system exists -- component names longer than 31 characters
need a truncated key for their CFB storage name.

## Appendix B: Coordinate System

Altium uses an internal coordinate unit of 1/10000 mil (0.1 mil = 0.00254 mm).
In the binary format, coordinates are stored as int16 in units of 10 mil (100000
internal units per unit):

```csharp
// Export:
WriteShort(Convert.ToInt16(argN / 100000), argName);

// Import:
ReadShort(out var value, argName);
argN = value * 100000;
```

The `PinFrac` sidecar stream provides sub-unit precision as int32 adjustments to
these truncated coordinates.

## Appendix C: Minor Versions

| File Type | Current Minor Version | Source |
|-----------|----------------------|--------|
| SchDoc (Sheet) | 13 | `SchDataExporterSheetV5.CurrentDocumentMinorVersion` |
| SchLib (Library) | 9 | `SchDataExporterLibraryV5.CurrentLibraryMinorVersion` |
