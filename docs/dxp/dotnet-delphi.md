# .NET-Delphi Interop in Altium Designer 26

Analysis of how .NET managed code interacts with native Delphi code, and how binary file formats are read/written.

## Key Finding: COM Interop, Not P/Invoke

The .NET code does **not** use `DllImport` / P/Invoke to call `SchApi_*` or `PcbApi_*` exports.
Instead, all interop uses **COM interfaces** with `[MarshalAs(UnmanagedType.Interface)]`.
The Delphi DLLs expose COM servers; the .NET side consumes them through interface definitions.

The only `DllImport` calls found in the entire codebase target:
- Windows system DLLs (`kernel32.dll`, `user32.dll`, `gdi32.dll`, `shell32.dll`, etc.)
- `System\SimDataEngine.dll` (simulation engine, found in `InteractiveProperties.Providers.SCH.DataModel.cs:16323`)

## Binary Architecture

| Binary | Size | Type | Role |
|--------|------|------|------|
| AdvSch.dll | 41MB | Native Delphi | Schematic engine, 134 `SchApi_*` exports |
| Advpcb.dll | 114MB | Native Delphi | PCB engine, 384 `PcbApi_*` exports |
| Altium.PCB.BinaryLoader.dll | 29MB | Native Delphi | PCB binary file loader |
| Altium.PCB.DataModel.dll | 14MB | Native Delphi | PCB data model (`TPcbArc`, `TPcbPad`, etc.) |
| WorkspaceManager.dll | 62MB | Native Delphi | Workspace/project management |
| X2.EXE | 40MB | Native Delphi | Main application |
| Altium.Sch.DataModel.dll | .NET | .NET | Schematic serialization/deserialization |
| Altium.SDK.Interfaces.dll | .NET | .NET | COM interface definitions |
| Altium.Edp.Interfaces.dll | .NET | .NET | Extended interface definitions |
| Altium.Edp.Classes.dll | .NET | .NET | Utility classes, parameter parsing |

## File Format: OLE Compound Files + Parameter Records

Altium files (`.SchDoc`, `.SchLib`, `.PcbDoc`, `.PcbLib`) are **OLE compound documents** (Microsoft Structured Storage). The .NET code uses **OpenMCDF** to read/write them.

### OLE Structure

```
CompoundFile (*.SchDoc, *.SchLib)
├── Root Storage
│   ├── Storage "FileHeader"
│   │   └── Stream "Data"
│   ├── Storage "<ComponentName>" (or "Sheet1", etc.)
│   │   └── Stream "Data"
│   │       ├── [4B header] [NB record 0]
│   │       ├── [4B header] [NB record 1]
│   │       └── ... (sequential records)
│   └── Storage "<AnotherComponent>"
│       └── Stream "Data"
```

### Record Format (within streams)

Each record is prefixed with a 4-byte header:

```
[4 bytes: size | (mode << 24)] [N bytes: payload]

mode = 0: ASCII parameter format  (pipe-delimited KEY=VALUE)
mode = 1: Binary format (raw bytes, may include ZLIB compression)
```

**ASCII mode (0):** Payload is parsed by `StrUtils.ParseWideData()` into `Dictionary<string, string>` key-value pairs. The `RECORD` key identifies the object type (maps to `TObjectId` enum).

**Binary mode (1):** Payload is read field-by-field using typed Read methods (`ReadShort`, `ReadInt`, `ReadDouble`, etc.). A `0xD0` byte prefix indicates ZLIB-compressed data with a checksum.

## Serialization Architecture

### File: `Altium.Sch.DataModel.cs`

#### Serializer Hierarchy (Lines 18763-23022)

```
SchDataSerializer (abstract, line 18763)
├── SchDataSerializerAscii (line 20735)      — Legacy ASCII format
├── SchDataSerializerBinary (line 21149)     — Direct binary (no OLE)
├── SchDataSerializerParam (line 21280)      — PRIMARY: OLE + param records
├── SchDataSerializerParamAscii (line 22285) — ASCII param variant
└── SchDataSerializerParamJSON (line 23022)  — JSON variant
```

**`SchDataSerializerParam`** is the production format reader/writer. Key fields:
- `CompoundFile compound` (line 21300) — OLE compound file handle
- `CFStorage storage` (line 21310) — Current storage being read
- `CFStream stream` (line 21312) — Current data stream
- `int mode` (line 21314) — 0=ASCII parameters, 1=binary data
- `byte[] loadBuffer` — Reusable read buffer

#### Core Record Reading: `GetNextLine()` (Lines 21512-21560)

This is the critical method that reads the next record from a stream:

1. Read 4-byte header → extract `size` (lower 24 bits) and `mode` (upper 8 bits)
2. If `mode == 1`: check for `0xD0` ZLIB checksum marker
3. Read `size` bytes of payload into `loadBuffer`
4. Null-terminate the buffer
5. If `mode == 0`: parse as ASCII parameters via `StrUtils.ParseWideData()` into `Dictionary<string, string>`
6. If `mode == 1`: write raw bytes into `binaryBuffer` MemoryStream

#### Stream Navigation (Lines 21354-21461)

- `FindFirstStream(string name)` — Find first matching storage in compound file
- `FindNextStream()` — Iterate to next matching storage
- `StartStream(string section, string name)` — Open specific stream within storage
- `EndStream()` — Close current stream, flush if writing

### Import/Export Methods (Lines 19851-20533)

Bidirectional typed field access:

| Method | ASCII Mode | Binary Mode | Size |
|--------|-----------|-------------|------|
| `Import_Instruction` / `Export_Instruction` | `RECORD=N` parameter | `ReadByte` | 1B |
| `Import_InstructionEx` / `Export_InstructionEx` | `RECORD=N` parameter | `ReadInt` | 4B |
| `Import_Boolean` / `Export_Boolean` | `"T"/"F"` string | - | - |
| `Import_Coord` / `Export_Coord` | Integer string | `ReadShort` + optional frac | 2-4B |
| `Import_Double` / `Export_Double` | Float string | `ReadDouble` | 8B |
| `Import_Long` / `Export_Long` | Integer string | `ReadLong` | 8B |
| `Import_String` / `Export_String` | Value string | `ReadString` (byte-prefixed) | 1+NB |
| `Import_DynamicString` / `Export_DynamicString` | Value string | `ReadString` | 1+NB |

### Primitive Binary Readers (Lines 19658-19757)

All little-endian, using `BitConverter`:

| Method | Bytes | .NET Type |
|--------|-------|-----------|
| `ReadShort` | 2 | Int16 |
| `ReadInt` | 4 | Int32 |
| `ReadUInt` | 4 | UInt32 |
| `ReadFloat` | 4 | Single |
| `ReadDouble` | 8 | Double |
| `ReadLong` | 8 | Int64 |
| `ReadPascalReal` | 6 | Double (Pascal real48 format) |
| `ReadText` | 2+N | Short(len) + ASCII bytes |
| `ReadString` | 1+N | Byte(len) + MBCS bytes |
| `ReadBinary` | 4+N | Int(len) + ZLIB-compressed bytes |

### ZLIB Compression (Lines 19748-19771)

Binary fields use ZLIB compression:
```
[4 bytes: compressed_size] [N bytes: zlib_compressed_data]
```
Decompressed via `System.IO.Compression.ZlibStream`.

## Object Type System

### Schematic TObjectId Enum

**File:** `Altium.SDK.Interfaces.cs:124786` (namespace `Rt_Schematic`)

| ID | Name | Description |
|----|------|-------------|
| 0 | eFirstObjectID | Container/placeholder |
| 1 | eClipBoardContainer | Clipboard container |
| 2 | eNote | Note annotation |
| 3 | eProbe | Probe marker |
| 4 | eRectangle | Rectangle shape |
| 5 | eLine | Line shape |
| 6 | eConnectionLine | Electrical connection line |
| 7 | eBusEntry | Bus entry point |
| 8 | eArc | Arc shape |
| 9 | eEllipticalArc | Elliptical arc |
| 10 | eRoundRectangle | Rounded rectangle |
| 11 | eImage | Image object |
| 12 | ePie | Pie shape |
| 13 | eTextFrame | Text frame |
| 14 | eRichTextDocument | Rich text |
| 15 | eEllipse | Ellipse shape |
| 16 | eJunction | Wire junction |
| 17 | ePolygon | Polygon shape |
| 18 | ePolyline | Polyline shape |
| 19 | eWire | Wire (electrical) |
| 20 | eBus | Bus (electrical) |
| 21 | eBezier | Bezier curve |
| 22 | eLabel | Text label |
| 23 | eHyperlink | Hyperlink |
| 24 | eNetLabel | Net label |
| 25 | eDesignator | Component designator |
| 26 | eSchComponent | Component instance |
| 27 | eParameter | Parameter/attribute |
| 28 | eParameterSet | Parameter set |
| 29 | eParameterList | Parameter list |
| 30 | eSheetName | Sheet name field |
| 31 | eSheetFileName | Sheet filename field |
| 32 | eSheet | Schematic sheet |
| 33 | eSchLib | Library container |
| 34 | eSymbol | Symbol definition |
| 35 | eNoERC | No-ERC marker |
| 36 | eErrorMarker | Error marker |
| 37 | ePin | Component pin |
| 38 | ePort | Port connector |
| 39 | ePowerObject | Power symbol |
| 40 | eSheetEntry | Sheet entry |
| 41 | eSheetSymbol | Sheet symbol |
| 42 | eTemplate | Template object |
| 43 | eTaskHolder | Task holder |
| 44 | eMapDefiner | Map definer |
| 45 | eImplementationMap | Implementation map |
| 46 | eImplementation | Implementation ref |
| 47 | eImplementationsList | Implementations list |
| 48 | eCrossSheetConnector | Cross-sheet connector |
| 49 | eCompileMask | Compile mask |
| ... | ... | (continues to 116) |

**NOTE:** In the `RECORD=N` parameter string, the record ID corresponds to these enum values.
However, IMPORTANT: the RECORD values in actual files appear to be **1-indexed** relative to the
enum (i.e., `RECORD=1` = header, `RECORD=2` = component, etc. — verify against actual files).

### PCB TObjectId Enum

**File:** `Altium.SDK.Interfaces.cs:68947` (namespace `Rt_PCB`)

| ID | Name | Description |
|----|------|-------------|
| 0 | eNoObject | No object |
| 1 | eArcObject | Arc |
| 2 | ePadObject | Pad |
| 3 | eViaObject | Via |
| 4 | eTrackObject | Track |
| 5 | eTextObject | Text |
| 6 | eFillObject | Fill |
| 7 | eConnectionObject | Ratsnest connection |
| 8 | eNetObject | Net |
| 9 | eComponentObject | Component |
| 10 | ePolyObject | Polygon |
| 11 | eRegionObject | Region |
| 12 | eComponentBodyObject | 3D body |
| 13 | eDimensionObject | Dimension |
| 14 | eCoordinateObject | Coordinate |
| 15 | eClassObject | Class |
| 16 | eRuleObject | Design rule |
| 17 | eFromToObject | From-To |
| 18 | eDifferentialPairObject | Differential pair |
| 19 | eViolationObject | DRC violation |
| 20 | eEmbeddedObject | Embedded object |
| 21 | eEmbeddedBoardObject | Embedded board |
| 22 | eSplitPlaneObject | Split plane |
| 23 | eTraceObject | Trace |
| 24 | eSpareViaObject | Spare via |
| 25 | eBoardObject | Board |
| 26 | eBoardOutlineObject | Board outline |

### Object Factory

**File:** `Altium.Sch.DataModel.cs:996-1088`

`CreateDataModelObject(ISch_BasicContainer owner, TObjectId objectId)` uses a switch expression
to instantiate the correct `SchData*` class based on the TObjectId value. All classes inherit from `SchDataObject`.

## Key Data Classes

### SchDataComponent (Line 33272+)

```
Properties: libReference, libraryPath, footprint, designator, comment,
            currentPartID, partCount, itemGUID, vaultGUID, ...
Children:   SchDataPin[], SchDataParameter[], SchDataDesignator
```

### SchDataPin (Line 39137)

```
Properties: name, designator, location (X,Y), pinLength, showPinName,
            orientation (TRotationBy90), connectorKind, pinColor, partVisible
```

### SchDataParameter (inherits SchDataParametrizedGroup)

```
Properties: name, value, readOnlyState, isHidden, location, orientation
```

## Parameter Parsing

**File:** `Altium.Edp.Classes.cs:9757-9779`

`ParseParametersStateString(string parameters)` parses pipe-delimited parameter strings:

```
Input:  "|RECORD=26|LIBREF=Resistor|COMPPREFIX=R|LOCATION.X=500|LOCATION.Y=300|"
Output: Dictionary { "RECORD":"26", "LIBREF":"Resistor", "COMPPREFIX":"R", ... }
```

Split on `|`, then split each token on first `=`. Keys are case-insensitive. First occurrence wins.

## COM Interface Locations

| Interface | File | Line | Purpose |
|-----------|------|------|---------|
| `IPCB_ServerInterface` | Altium.SDK.Interfaces.cs | 45962 | PCB server COM interface |
| `ISch_ServerInterface` | Altium.SDK.Interfaces.cs | 100981 | Schematic server COM interface |
| `ISchDocument` | Altium.Edp.Interfaces.cs | 49448 | Schematic document interface |
| `ISchLib` | Altium.Edp.Interfaces.cs | 51208 | Schematic library interface |
| `ISchLibraryComponent` | Altium.Edp.Interfaces.cs | 53405 | Library component interface |
| `IPCB_ServerInterface` | Altium.Edp.Interfaces.cs | 172141 | PCB server (extended) |
| `PcbApi_Export_ToPainter` | Altium.Edp.Interfaces.cs | 287651 | PCB export via COM |

## Complete Data Flow

```
FILE (*.SchDoc / *.SchLib)
  │
  ▼
CompoundFile (OpenMCDF)
  │
  ├── FindFirstStream() → locate storage by name
  ├── StartStream() → open "Data" stream within storage
  │
  ▼
SchDataSerializerParam.GetNextLine()
  │
  ├── Read 4-byte header → size (24 bits) + mode (8 bits)
  ├── Read N bytes payload
  │
  ├── mode=0 (ASCII): ParseWideData() → Dict<string,string>
  │   └── RECORD key → TObjectId → CreateDataModelObject()
  │
  └── mode=1 (BINARY): raw bytes → binaryBuffer
      └── ReadShort/ReadInt/ReadDouble/ReadString/ReadBinary
  │
  ▼
SchData* object populated via Import_*() methods
  │
  ├── Import_Instruction("RECORD") → identifies object type
  ├── Import_String("LIBREF") → component reference
  ├── Import_Coord("LOCATION.X") → X coordinate
  ├── Import_Boolean("ISMIRRORED") → mirror state
  └── ... (all fields populated sequentially)
```

## Where to Start for File Format RE

1. **Schematic records:** `Altium.Sch.DataModel.cs` lines 996-1088 (object factory) maps `RECORD` values to classes. Each class's Import/Export methods define the exact field layout.

2. **Parameter format:** Already well-understood: `|KEY=VALUE|KEY2=VALUE2|` — see `Altium.Edp.Classes.cs:9757`.

3. **Binary field layout:** `Altium.Sch.DataModel.cs` lines 19658-19757 define all primitive read methods. Cross-reference with each `SchData*` class's serialization methods to get exact byte layouts.

4. **PCB records:** The PCB side uses the same pattern but through Delphi COM objects. The `TObjectId` enum at `Altium.SDK.Interfaces.cs:68947` defines all PCB record types. The actual binary reading is in the native Delphi DLLs (`Altium.PCB.BinaryLoader.dll`, `Altium.PCB.DataModel.dll`) — these need Ghidra analysis.

5. **OLE structure:** `OpenMCDF.cs` — the compound file library. Storage/stream hierarchy mirrors the document's object hierarchy.

6. **Validation:** Compare the field names from Import/Export methods against what `altium-format` already parses to find gaps.

## Critical Architecture Finding: Two Completely Different Loading Paths

### Schematic files (.SchDoc/.SchLib) — Fully in .NET

The entire schematic loading path is implemented in **managed C# code**. All field names, record codes, and serialization logic are in readable C# source:
- `SchDataSerializerParam` reads OLE compound files via OpenMCDF
- Records are `|KEY=VALUE|` ASCII or binary with explicit field names
- The object factory and all `SchData*` classes define the exact format
- **No Ghidra analysis needed** — the .NET source is the complete spec

### PCB files (.PcbDoc/.PcbLib) — Delphi Native Code

The PCB loading path is entirely in **native Delphi code**:
- `Altium.PCB.BinaryLoader.dll` provides OLE structured storage abstraction (via `StgOpenStorage`)
- `Advpcb.dll` creates objects and reads binary field data
- The .NET layer only accesses PCB data through COM interface wrappers
- **No parameter name strings** exist in the Delphi binaries — field access is through vtable offsets
- **Ghidra analysis required** to understand binary record layouts

## PCB Loading Trace (Ghidra)

### Entry Point: `PcbApi_LoadBoardByFullFileName` (Advpcb.dll @ 0x03d20660)

```
PcbApi_LoadBoardByFullFileName(filename)
  │
  ├── PcbApi_GetBoardHandleFromFullFileName(filename)
  │   └── Returns existing board handle if already open
  │
  └── If not open:
      ├── GetStorageManager() → IPCB_StructuredStorage singleton
      ├── vtable[+0x188] → Import_FromFile(filename, ...)
      │   └── StgOpenStorage() → opens OLE compound file
      └── FUN_0434adc0() → creates board from loaded data
```

### Object Factory: `PcbApi_CreateObject` (Advpcb.dll @ 0x03d22900)

Takes a single byte (TObjectId) and returns a new Delphi object via VMT class pointer:

| Type ID | Enum Name        | VMT Base     | Delphi Class       | Parent Class           |
|---------|------------------|--------------|--------------------|------------------------|
| 1       | Arc              | 0x0137d300   | TArc               | TKeepoutPrimitive      |
| 2       | Pad              | 0x045c4070   | TPad               | TStackObject           |
| 3       | Via              | 0x0462ae98   | TVia               | TStackObject           |
| 4       | Track            | 0x0133ac80   | TTrack             | TKeepoutPrimitive      |
| 5       | Text             | 0x0128e250   | TText              | TRectangularPrimitive  |
| 6       | Fill             | 0x0133ba00   | TFill              | TRectangularPrimitive  |
| 7       | Connection       | 0x013a8b50   | TConnection        | TPrimitive             |
| 8       | Net              | 0x01379430   | TNet               | TGroup                 |
| 9       | Component        | 0x0445d8b0   | TComp              | TGroup                 |
| 10      | Polygon          | 0x013551d8   | TSignalLayerPolygon| TPourablePolygon       |
| 13      | Dimension        | 0x019659c0   | TOriginalDimension | TDimension             |
| 14      | Coordinate       | 0x020e6f30   | TCoordinate        | TGroup                 |
| 17      | FromTo           | 0x01a33e00   | TFromTo            | TPrimitive             |
| 18      | DifferentialPair | 0x01f8efe0   | TDifferentialPair  | TPrimitive             |
| 20      | Embedded         | 0x017a5800   | TEmbedded          | TPrimitive             |
| 22      | SplitPlane       | 0x01361be0   | TSplitPlane        | TPourablePolygon       |

Types NOT in PcbApi_CreateObject (created through specialized factory functions or internally):

| Type ID | Enum Name        | VMT Base     | Delphi Class       | Parent Class           | Factory Function               |
|---------|------------------|--------------|--------------------|------------------------|---------------------------------|
| 11      | Region           | 0x013660b0   | TBoardRegion       | TPolyRegion            | (internal)                     |
| 12      | ComponentBody    | 0x013a4380   | TComponentBody     | TPolyRegion            | (internal)                     |
| 15      | Class            | 0x0469bf08   | TObjectClass       | TObjectClassBase       | PcbApi_CreateClassObject       |
| 16      | Rule             | 0x0137fb90   | TRule              | TPrimitive             | PcbApi_CreateRuleObject        |
| 19      | Violation        | 0x0137eb18   | TViolation         | TAbstractViolation     | (internal)                     |
| 21      | EmbeddedBoard    | 0x01a21478   | TEmbeddedBoard     | TRectangularPrimitive  | (internal)                     |
| 25?     | Board            | 0x046fe388   | TBoard             | TPrimitive             | PcbApi_CreateRule_FromParameters|
| 26?     | BoardOutline     | 0x03e0b520   | TBoardOutline      | TSignalLayerPolygon    | PcbApi_CreateBoardOutline      |

**Note:** The existing PcbObjectId enum in the Rust code has Board=24 and BoardOutline=25. Types 22 (SplitPlane) and 23 (Trace/SpareVia) are marked as internal in comments, but SplitPlane IS in PcbApi_CreateObject. Board/BoardOutline type IDs need verification against actual PcbLib/PcbDoc files.

### Storage Manager: `GetStorageManager` (BinaryLoader.dll @ 0x01b774f0)

Singleton pattern — creates the OLE storage manager once, reuses on subsequent calls.
Uses `StgOpenStorage` from OLE32.dll to open compound files.

### COM Bridge Mechanism

```
DXP.Client.GetServerModuleByName("PCB" or "SCH")
  → Loads DLL, calls ServerFactory export
    → Advpcb.dll ServerFactory @ 0x05ae6f80
    → AdvSch.dll  ServerFactory @ 0x021b6c60
  → Creates Delphi object, returns pointer at object + 0x88 (vtable offset)
  → .NET casts to COM interface (IPCB_ServerInterface / ISch_ServerInterface)
```

## PCB RE Strategy

Since PCB binary format reading is in Delphi native code with no string-based parameter names:

1. **Start with known VMT addresses** from `PcbApi_CreateObject` — each VMT defines a TPcb* class
2. **Find the virtual `ReadFromStream`/`LoadFromStream` methods** in each VMT — these define the binary field layout
3. **Cross-reference with .NET interface definitions** — `IPCB_Arc`, `IPCB_Pad`, etc. define the property names and types, even though the Delphi code doesn't use strings
4. **Use `ghidra type get` and `ghidra x-ref`** to trace from VMT to read methods
5. **Compare with `altium-format`'s existing PCB parser** to validate field offsets

### Key Ghidra Commands for PCB RE

```bash
# List VMT methods for a PCB object class (e.g., TPcbArc at VMT 0x0137d300)
ghidra x-ref from 0x0137d300 --project testfix --program Advpcb.dll

# Find all functions that reference a VMT (callers that create/use this type)
ghidra x-ref to 0x0137d300 --project testfix --program Advpcb.dll

# Decompile a specific function
ghidra decompile 0x0137d300 --project testfix --program Advpcb.dll

# Search for ReadFromStream patterns
ghidra find function "*ReadFrom*" --project testfix --program Advpcb.dll
ghidra find function "*LoadFrom*" --project testfix --program Advpcb.dll
ghidra find function "*SaveTo*" --project testfix --program Advpcb.dll
```

## VMT Analysis Results (Ghidra Deep Trace)

### Key Finding: VMT Objects Are Thin COM Wrappers

The VMT addresses from `PcbApi_CreateObject` create **thin COM wrapper objects**, NOT the actual data-bearing objects. The wrapper has only 40 bytes (instance size = 0x28) containing:

| Offset | Size | Content |
|--------|------|---------|
| +0 | 8 | VMT pointer |
| +8 | 8 | Inner COM interface pointer (IPCB_Primitive/IPCB_Text/etc.) |
| +16 | 8 | Additional COM interface pointer (released in destructor via vtable[0x18]) |
| +24-39 | 16 | Additional fields/padding |

### VMT Layout (37 slots, 296 bytes)

All four analyzed types (Text, Fill, Connection, Net) share this layout:

| Slot | Offset | Content | Notes |
|------|--------|---------|-------|
| 0 | +0 | CLASS-SPECIFIC DATA | NOT code - RTTI/metadata (Ghidra produces "bad instruction data") |
| 1-3 | +8-24 | NULL (0x0000000000000000) | Unused interface table slots |
| 4 | +32 | CLASS-SPECIFIC DATA | NOT code |
| 5 | +40 | NULL | |
| 6 | +48 | CLASS-SPECIFIC DATA | NOT code |
| 7 | +56 | NULL | |
| 8 | +64 | CLASS-SPECIFIC DATA | NOT code |
| 9 | +72 | 0x28 (40 decimal) | Instance size (shared across all types) |
| 10 | +80 | PARTIALLY VARIES | Text/Fill: FUN_01255400, Connection: FUN_04693580, Net: FUN_0469a518 |
| 11-20 | +88-160 | TObject base methods | FUN_00411ce0..FUN_00411850 (shared) |
| 21 | +168 | FUN_010d4d90 | Destructor (shared) |
| 22-24 | +176-192 | TObject methods | FUN_00411870..FUN_00411890 (shared) |
| **25** | **+200** | **CLASS-SPECIFIC** | **Dispatcher through global object** (only class-specific CODE) |
| 26 | +208 | FUN_010d4bd0 | SafeCallException handler (returns 0x80004002 = E_NOINTERFACE) |
| **27** | **+216** | **FUN_0469d2c0** | **Constructor** (calls _ClassCreate + FUN_010d4730) |
| 28-35 | +224-280 | FUN_04692c20..FUN_04692dc0 | COM delegation methods (all call inner object at self+8) |
| 36 | +288 | FUN_046a0c10 | COM delegation (calls inner vtable[0xcf0]) |

**Slots 37+ are NOT virtual methods** — they contain RTTI class metadata (ASCII strings like "Fill", "Connection", "GlyphText" etc. mixed with data).

**Negative offsets are NOT standard Delphi RTTI** — they contain x86-64 machine code from surrounding functions. This is because the VMT is NOT laid out with the standard Delphi negative-offset RTTI metadata (vmtSelfPtr, vmtClassName, etc.). The addresses appear to be inside the .text section, packed among code.

### Constructor (FUN_010d4730)

```c
void FUN_010d4730(undefined8 *param_1) {
    auStack_20[0] = 0;
    (**(code **)*param_1)(param_1, auStack_20);  // Call VMT[0] (but VMT[0] is data!)
    FUN_0041c580(param_1 + 1, auStack_20[0]);    // Store COM interface at self+8
    if (param_1[1] != 0) {
        uVar1 = (**(code **)(*(longlong *)param_1[1] + 0x20))
                ((longlong *)param_1[1], param_1);  // Call inner->vtable[0x20](inner, self)
        FUN_0041c980(uVar1);  // OleCheck(HRESULT)
    }
    FUN_0041c540(auStack_20);  // Release temp interface
}
```

**Note:** The call to VMT[0] as code is puzzling since VMT[0] decompiles as data. This might indicate the addresses we have are offset from the true VMT base, or the constructor is called differently than analyzed.

### Destructor (FUN_010d4d90, Slot 21)

```c
void FUN_010d4d90(longlong param_1, uint param_2) {
    FUN_004126e0(param_1, param_2);           // TObject.FreeInstance
    FUN_010d46c0(param_1, param_2 & 0xfc);    // Cleanup inner objects
    if (*(longlong *)(param_1 + 0x10) != 0) {
        (**(code **)(**(longlong **)(param_1 + 0x10) + 0x18))
            (*(longlong **)(param_1 + 0x10));  // Release object at self+0x10
    }
    if ('\0' < (char)param_2) {
        FUN_00412660(param_1);                 // _ClassDestroy
    }
}
```

### COM Delegation Pattern (Slots 28-36)

All shared methods delegate to the inner COM object at `self+8`:

```c
// Slot 28: FUN_04692c20
void slot28(longlong param_1, ...) {
    inner = *(longlong *)(param_1 + 8);
    result = (**(code **)(*(longlong *)inner + 0x40))(inner, ...);  // inner->vtable[0x40]
    FUN_0041c980(result);  // OleCheck
}
```

COM delegation vtable offsets used:
| Slot | Inner vtable offset | IUnknown offset equivalent |
|------|--------------------|-----------------------------|
| 28 | +0x40 | Method 8 |
| 29 | +0x68 | Method 13 |
| 30 | +0x70 | Method 14 |
| 31 | +0x78 | Method 15 |
| 32 | +0x80 | Method 16 |
| 33 | +0x88 | Method 17 |
| 34 | +0x90 | Method 18 |
| 35 | +0x98 | Method 19 (with extra param) |
| 36 | +0xcf0 | Method 414 |

### Slot 25 Dispatcher (Class-Specific)

Each type's slot 25 calls through a global object at `0x062a8538` with a type-specific vtable offset:

| Type | Slot 25 Function | Global vtable offset |
|------|-----------------|---------------------|
| eTextObject(5) | FUN_01292cd0 | 0x98 |
| eFillObject(6) | FUN_0133bbd0 | 0x90 |
| eConnectionObject(7) | FUN_013a9150 | 0x168 |
| eNetObject(8) | FUN_0137ac10 | 0x698 |

The global object at 0x062a8538 is in BSS/uninitialized memory (points to 0x0638f4a8 which is not in mapped PE sections).

### Helper Functions

| Function | Purpose | Signature |
|----------|---------|-----------|
| FUN_0041c580 | COM interface assignment (AddRef new, store, Release old) | `void(longlong *dest, longlong *newValue)` |
| FUN_0041c540 | COM interface release (IntfClear) | `void(longlong *intf)` |
| FUN_0041c980 | OleCheck / SafeCallResult (raises exception if HRESULT < 0) | `int(int hresult)` |
| FUN_004126e0 | TObject.FreeInstance | `void(longlong self, uint flags)` |
| FUN_00412660 | _ClassDestroy | `void(longlong self)` |
| FUN_00412640 | _ClassCreate | `longlong(longlong vmtAddr)` |

### Revised PCB RE Strategy

The original strategy of finding `ReadFromStream`/`LoadFromStream` in the VMT methods does **not work** because:

1. The VMT addresses are for thin COM wrappers, not data objects
2. All serialization symbols are stripped (no `ReadFrom*`, `SaveTo*`, `LoadFrom*` string matches)
3. The actual data resides in the inner COM object (accessed at self+8)
4. Binary reading is done by `Altium.PCB.BinaryLoader.dll` (native Delphi, 107K functions)
5. The data model is in `Altium.PCB.DataModel.dll` (native Delphi)

**Updated approach:**
1. Use `.NET interface definitions` (IPCB_Text, IPCB_Fill, etc.) to understand field names and types
2. Use `Export_ToParameters` (IPCB_Primitive DispId 504540) to export objects as parameter strings — this reveals the ASCII field names
3. For binary format RE, trace from `Altium.PCB.BinaryLoader.dll`'s `GetStorageManager` export and follow the OLE structured storage path
4. Validate binary layouts by comparing `altium-format`'s existing parser output against real `.PcbDoc` files
5. Use the `.NET COM interface vtable ordering` (DispId values) to correlate with Ghidra's vtable analysis of the inner COM objects

### Delphi 64-bit VMT Layout (Corrected)

The VMT base addresses from PcbApi_CreateObject point to the **class info base**, NOT the start of user virtual methods. The standard Delphi 64-bit VMT layout:

```
Offset  Field                  Description
------  ---------------------  -----------------------------------------
+0x00   vmtSelfPtr             Pointer to VMT[0] (base + 0xC8)
+0x08   vmtIntfTable           Interface table pointer
+0x10   vmtAutoTable           Auto table pointer
+0x18   vmtInitTable           Init table pointer
+0x20   vmtTypeInfo            Extended type info pointer
+0x28   (reserved)             Usually 0 or another table pointer
+0x30   vmtFieldTable          Field RTTI table
+0x38   vmtMethodTable         Method RTTI table
+0x40   vmtClassName           Pointer to Delphi short string (len byte + ASCII)
+0x48   vmtInstanceSize        Instance size in bytes
+0x50   vmtParent              Pointer to parent class info base
+0x58   TObject.Destroy        [VMT[-0x70]] Standard TObject virtual methods
+0x60   TObject.FreeInstance    [VMT[-0x68]]
+0x68   TObject.NewInstance     [VMT[-0x60]]
+0x70   TObject.DefaultHandler [VMT[-0x58]]
+0x78   TObject.Dispatch       [VMT[-0x50]]
+0x80   TObject.BeforeDestruction [VMT[-0x48]]
+0x88   TObject.AfterConstruction [VMT[-0x40]]
+0x90   TObject.SafeCallException [VMT[-0x38]]
+0x98   TObject.GetHashCode    [VMT[-0x30]]
+0xA0   TObject.Equals         [VMT[-0x28]]
+0xA8   TObject.ToString       [VMT[-0x20]]
+0xB0   TObject.ClassName(virt)[VMT[-0x18]]
+0xB8   TObject.InstanceSize   [VMT[-0x10]]
+0xC0   TObject.InheritsFrom   [VMT[-0x08]]
+0xC8   User virtual method 0  [VMT[0x00]] <-- vmtSelfPtr points here
+0xD0   User virtual method 1  [VMT[0x08]]
...
```

This explains the earlier confusing VMT slot analysis: what appeared as "data" in slots 0-10 is actually the RTTI metadata at the class info base, while user virtual methods start 0xC8 bytes later.

### Complete Class Hierarchy

```
TObject
  TBaseWrapper
    TAbstractObject
      TContainedObject
        TPrimitive                          <-- base for all PCB objects
          TConnection (7)
          TFromTo (17)
          TDifferentialPair (18)
          TEmbedded (20)
          TRule (16)
          TBoard (25?)
          TKeepoutPrimitive
            TArc (1)
            TTrack (4)
            TPolyRegion
              TBoardRegion (11)
              TComponentBody (12)
            TRectangularPrimitive
              TText (5)
              TFill (6)
              TEmbeddedBoard (21)
          TGroup
            TNet (8)
            TComp (9)
            TDimension (13)
              TOriginalDimension (13, subtype 8)
              TLinearDimension (subtype 1)
              TAngularDimension (subtype 2)
              TRadialDimension (subtype 3)
              TLeaderDimension (subtype 4)
              TDatumDimension (subtype 5)
              TBaselineDimension (subtype 6)
              TCenterDimension (subtype 7)
              TLinearDiameterDimension (subtype 9)
              TRadialDiameterDimension (subtype 10)
            TCoordinate (14)
            TAbstractPolygon
              TPourablePolygon
                TSignalLayerPolygon (10)
                  TBoardOutline (26?)
                TSplitPlane (22)
          TAbstractViolation
            TViolation (19)
          TObjectClassBase
            TObjectClass (15)
          TStackObject
            TPad (2)
            TVia (3)
```

### Wrapper-Implementation Architecture

All PCB objects use a thin wrapper that delegates to an inner implementation object:

- **Wrapper** (TArc/TPad/etc): 40 bytes, stores VMT pointer + inner object pointer
  - offset 0: VMT pointer → VMT[0] at base + 0xC8
  - offset 8: pointer to inner implementation object

- **Inner object**: Created by a runtime-initialized global manager at VA 0x062a8538
  - Has its own vtable with serialization/property methods
  - Created through the wrapper's user_vmethod_0 dispatch

- **Manager dispatch**: Each type overrides user_vmethod_0 to call the manager at a type-specific vtable offset:

| Delphi Class        | Manager Offset | Notes                              |
|---------------------|---------------|------------------------------------|
| TPrimitive          | 0x028         | Base class                         |
| TGroup              | 0x030         | Group container                    |
| TObjectClass        | 0x040         | Also overrides vmethod_2           |
| TKeepoutPrimitive   | 0x078         | Keepout-aware base                 |
| TTrack              | 0x080         |                                    |
| TArc                | 0x088         |                                    |
| TFill               | 0x090         |                                    |
| TText               | 0x098         |                                    |
| TEmbeddedBoard      | 0x0A8         |                                    |
| TRectangularPrim    | 0x0B0         |                                    |
| TSplitPlane         | 0x0B8         |                                    |
| TPolyRegion         | 0x0D8         |                                    |
| TFromTo             | 0x0E0         |                                    |
| TBoardRegion        | 0x0E8         |                                    |
| TComponentBody      | 0x0F0         |                                    |
| TAbstractPolygon    | 0x0F8         |                                    |
| TPourablePolygon    | 0x100         |                                    |
| TSignalLayerPolygon | 0x108         |                                    |
| TBoardOutline       | 0x130         |                                    |
| TConnection         | 0x168         |                                    |
| TDifferentialPair   | 0x180         |                                    |
| TEmbedded           | 0x1C0         |                                    |
| TBoard              | 0x1D8         |                                    |
| TViolation          | 0x2C8         |                                    |
| TComp               | 0x690         |                                    |
| TNet                | 0x698         |                                    |
| TPad                | 0x6A8         |                                    |
| TVia                | 0x6B8         |                                    |

### Serialization via Inner Objects

The actual serialization (binary read/write) happens through the **inner implementation object**, NOT the wrapper. From PcbApi_QueryObjectParameters:

- Inner object vtable offset 0x58: Write properties to parameter string (called with `(inner, params, -1)`)
- Inner object vtable offset 0xA0: Get properties/status
- Inner object vtable offset 0x20: Set back-pointer to wrapper

The global manager at 0x062a8538 is a runtime-initialized factory. Its class cannot be determined from static analysis alone — the serialization methods require dynamic analysis (debugging at runtime) to trace further.

### Dimension Subtypes (via PcbApi_CreateDimensionObject)

| Kind ID | Class                       | VMT Base     |
|---------|-----------------------------|--------------|
| 1       | TLinearDimension            | 0x01963dc8   |
| 2       | TAngularDimension           | 0x019641b0   |
| 3       | TRadialDimension            | 0x019645c8   |
| 4       | TLeaderDimension            | 0x01964930   |
| 5       | TDatumDimension             | 0x01964dd0   |
| 6       | TBaselineDimension          | 0x01965028   |
| 7       | TCenterDimension            | 0x01965768   |
| 8       | TOriginalDimension          | 0x019659c0   |
| 9       | TLinearDiameterDimension    | 0x01965dc8   |
| 10      | TRadialDiameterDimension    | 0x01965f58   |

### Class Object Mapping (via PcbApi_CreateClassObject)

PcbApi_CreateClassObject uses a single Delphi class TObjectClass (0x0469bf08) for all class types, calling FUN_046a1370 with a class kind parameter:

| Param | Class Kind | Description          |
|-------|-----------|----------------------|
| 0x02  | 3         | Pad class            |
| 0x08  | 0         | Net class            |
| 0x09  | 1         | Component class      |
| 0x11  | 2         | From-to class        |

### Other Discovered Types

- **TWirebond**: VMT 0x017102b8, inherits TTrack, created via `PcbApi_CreateObjectByViewableObjectId(0x74)`
- The non-PCB TRegion at 0x02472fc0 (inherits TObject, instSize=32) is a general-purpose geometric region class, NOT the PCB Region primitive. The PCB Region is TBoardRegion (0x013660b0, inherits TPolyRegion).
- The non-PCB TPolygon at 0x0126d350 (inherits TInterfacedObject, instSize=128) is NOT the PCB Polygon primitive. The PCB Polygon is TSignalLayerPolygon (0x013551d8).

### COM Interface Field Summary (from .NET SDK)

#### IPCB_Connection (extends IPCB_Primitive)
| DispId | Method | Type |
|--------|--------|------|
| 506842 | GetState_X1 | int (Coord) |
| 506843 | GetState_Y1 | int (Coord) |
| 506844 | GetState_X2 | int (Coord) |
| 506845 | GetState_Y2 | int (Coord) |
| 506846 | Internal_GetState_Layer1 | int |
| 506847 | Internal_GetState_Layer2 | int |
| 506848 | Internal_GetState_Mode | int |

#### IPCB_Fill (extends IPCB_RectangularPrimitive)
| DispId | Method | Type |
|--------|--------|------|
| 514255 | GetState_Width | int (Coord) |
| 514256 | GetState_Length | int (Coord) |
| 514259 | GetState_LocationX | int (Coord) |
| 514260 | GetState_LocationY | int (Coord) |

#### IPCB_Net (extends IPCB_Group, IPCB_Primitive)
| DispId | Method | Type |
|--------|--------|------|
| 505466 | GetState_Color | uint |
| 505467 | GetState_Name | string |
| 505468 | GetState_ConnectsVisible | bool |
| 505476 | GetState_LoopRemoval | bool |

#### IPCB_Text (extends IPCB_RectangularPrimitive)
| DispId | Method | Type |
|--------|--------|------|
| 505506 | GetState_Size | int (Coord) |
| 505507 | GetState_FontID | short |
| 505508 | GetState_Text | string |
| 505509 | GetState_Width | int (Coord = stroke width) |
| 505510 | GetState_Mirror | bool |
| 505513 | GetState_UseTTFonts | bool |
| 505514 | GetState_Bold | bool |
| 505515 | GetState_Italic | bool |
| 505516 | GetState_FontName | string |
| 505517 | GetState_Inverted | bool |
| 505518 | GetState_InvertedTTTextBorder | int (Coord) |
| 505544 | GetState_UseInvertedRectangle | bool |
| 505542 | GetState_InvRectWidth | int |
| 505543 | GetState_InvRectHeight | int |
| 505561 | Internal_GetState_TextKind | int |

## PCB Binary Serialization in BinaryLoader.dll (Ghidra Analysis)

### Architecture

`Altium.PCB.BinaryLoader.dll` (29MB, native Delphi, 107K functions) is responsible for reading and writing PCB binary data within OLE compound file streams. It exports a `GetStorageManager` function (ordinal 4, VA `0x01b774f0`) that returns a singleton COM storage manager interface.

The binary serialization uses a **section-based architecture** with three format versions per object type:

| Class Pattern | Format | Description |
|---|---|---|
| `TTracksSection` (Binary_Version3) | V3 | Legacy binary format |
| `TTracksSection` (Binary_Version4) | V4 | Newer binary format |
| `TTracksSection` (Section_Tracks) | Section | Latest format with iteration |

Each section class has a VMT with type-specific virtual methods:
- **VMT[16] (+128)** = Read method (reads fields from COM object into binary record buffer)
- **VMT[17] (+136)** = Write method (writes fields from binary record buffer to COM object)

### VMT Addresses in BinaryLoader.dll

#### V3 Binary Format Classes

| Class | VMT Address | Read Method | Write Method |
|---|---|---|---|
| TTracksSection_V3 | 0x18f20a8 | FUN_018f9310 | FUN_018f93e0 |
| TArcsSection_V3 | 0x18f2bb0 | FUN_018fb890 | FUN_018fb980 |
| TPadsSection_V3 | 0x18f2510 | FUN_018fac50 | FUN_018fae90 |
| TViasSection_V3 | 0x18f22e0 | FUN_018f95e0 | FUN_018f96d0 |

#### V4 Binary Format Classes

| Class | VMT Address | Read Method | Write Method |
|---|---|---|---|
| TTracksSection_V4 | 0x1907920 | FUN_0190d030 (shared) | FUN_0190ec90 |
| TArcsSection_V4 | 0x1908450 | FUN_0190d030 (shared) | FUN_01911370 |
| TPadsSection_V4 | 0x1907d98 | FUN_0190d030 (shared) | FUN_01910730 |
| TViasSection_V4 | 0x1907b60 | FUN_0190d030 (shared) | FUN_0190ef00 |

Note: V4 classes share a single read dispatcher (`FUN_0190d030`) that iterates records and calls the per-record handler via VMT[4] (`*param_1 + 0x20`).

#### Section_* Format Classes

| Class | VMT Address | Read Method | Write Method |
|---|---|---|---|
| TTracksSection_Sect | 0x1948478 | FUN_018825f0 | FUN_0189fb10 |
| TArcsSection_Sect | 0x19487b8 | FUN_018825f0 | FUN_0189fb10 |
| TPadsSection_Sect | 0x1966db8 | FUN_01967110 | FUN_0189fb10 |
| TViasSection_Sect | 0x1967288 | FUN_018825f0 | FUN_0189fb10 |

### Binary Record Layout: Common Base Fields

All PCB primitives share a 14-byte (0x0e) common header read by `FUN_018f7ae0`:

```
Offset  Size  Field
------  ----  -----
+0x00   1     Layer (converted from Altium internal layer ID)
+0x01   1     Flags byte 1 (ObjectID/flags via FUN_01905ad0)
+0x02   1     Flags byte 2 (additional flags via FUN_01905ab0)
+0x03   1     Unknown (from COM vtable+0x60 call via FUN_01905ac0)
+0x04   2     NetIndex (default: 0xFFFF = no net)
+0x06   2     ComponentIndex (default: 0xFFFF = free primitive)
+0x08   2     PolygonIndex (default: 0xFFFF = not in polygon)
+0x0A   2     CoordinateIndex (default: 0xFFFF)
+0x0C   2     DimensionIndex (default: 0xFFFF)
```

**Existing Rust implementation (`PcbPrimitiveCommon`, 13 bytes):**
```
+0x00   1     layer (Layer byte)
+0x01   2     flags (PcbFlags u16)
+0x03   10    [0xFF; 10] (placeholder bytes)
```

The Rust implementation treats the 10 net/component/polygon/coord/dim index bytes as fixed 0xFF values. The BinaryLoader analysis shows these are actually conditional indices populated when the primitive belongs to a net, component, etc.

### Track Binary Layout (V3)

**Decompiled from: `FUN_018f9310` (TTracksSection_V3 Read)**

```
Offset  Size  Field                    COM Interface Method
------  ----  -----                    --------------------
+0x00   14    Common fields            (see base layout above)
+0x0E   4     X1 (start X)            IPCB_Track.GetState_X1 (vtable+0x448)
+0x12   4     Y1 (start Y)            IPCB_Track.GetState_Y1 (vtable+0x450)
+0x16   4     X2 (end X)              IPCB_Track.GetState_X2 (vtable+0x458)
+0x1A   4     Y2 (end Y)              IPCB_Track.GetState_Y2 (vtable+0x460)
+0x1E   4     Width                   IPCB_Track.GetState_Width (vtable+0x468)
+0x??   4     SubnetID                FUN_0184e8a0 -> FUN_01905b10
+0x??   4     UnionIndex              vtable+0x118 -> FUN_01905b00
+0x24   2     UserRouted/Flags        FUN_0180caa0
```

**Existing Rust implementation (`PcbTrack`):**
```
+0x00   13    PcbPrimitiveCommon
+0x0D   4     start.x (Coord)
+0x11   4     start.y (Coord)
+0x15   4     end.x (Coord)
+0x19   4     end.y (Coord)
+0x1D   4     width (Coord)
+0x21   16    unknown (Vec<u8>)
```

The Rust implementation is off by 1 byte in its common header (13 vs 14 bytes), but the type-specific fields match. The 16 "unknown" bytes are SubnetID(4) + UnionIndex(4) + UserRouted(2) + likely padding(6).

**COM vtable offset calculation:** IPCB_Primitive has 130 methods. With 3 IUnknown methods preceding them, the first IPCB_Track method (GetState_X1) is at offset `(3 + 130) * 8 = 0x448`, confirming the decompiled code.

### Arc Binary Layout (V3)

**Decompiled from: `FUN_018fb890` (TArcsSection_V3 Read)**

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   14    Common fields            Base class read
+0x0E   4     CenterX                  VMT method (vtable+0xb0 on source object)
+0x12   4     CenterY                  VMT method (vtable+0xb8 on source object)
+0x16   4     Radius                   FUN_0180ce80
+0x1A   6     StartAngle (Extended)    FUN_0180cf00 -> FUN_013e5870 (Delphi Extended->f64)
+0x20   6     EndAngle (Extended)      FUN_0180cf50 -> FUN_013e5870
+0x26   4     LineWidth                FUN_0180cfa0
+0x2A   2     Unknown (short)          FUN_0180ceb0
```

**Existing Rust implementation (`PcbArc`):**
```
+0x00   13    PcbPrimitiveCommon
+0x0D   4     location.x (Coord)
+0x11   4     location.y (Coord)
+0x15   4     radius (Coord)
+0x19   8     start_angle (f64)
+0x21   8     end_angle (f64)
+0x29   4     width (Coord)
```

**Important discrepancy:** The BinaryLoader stores angles as 6-byte Delphi `Extended` (Real48) values, converted to f64 via `FUN_013e5870`. The Rust implementation reads them as 8-byte f64 directly. This difference suggests the V3 format may use Real48 while newer formats may use f64, or the Rust implementation handles a different binary format version.

### Via Binary Layout (V3)

**Decompiled from: `FUN_018f95e0` (TViasSection_V3 Read)**

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   14    Common fields            Base class read
+0x0E   4     XLocation                VMT method (vtable+0xb0 on source object)
+0x12   4     YLocation                VMT method (vtable+0xb8 on source object)
+0x16   4     HoleSize                 FUN_01813370
+0x1A   4     Diameter/Size            FUN_018131c0
+0x??   4     SubnetID                 FUN_0184e8a0 -> FUN_01905b30
+0x1F   1     FromLayer                FUN_01813460 -> layer ID conversion
+0x20   1     ToLayer                  FUN_01813420 -> layer ID conversion
```

**Existing Rust implementation (`PcbVia`):** The Rust Via implementation is significantly more complex with a multi-block format including thermal relief fields, solder mask expansion, 32-layer diameter arrays, and extended trailer data (112-142 bytes). The V3 format in BinaryLoader shows a much simpler layout, suggesting the complex format is from a later version or a different binary format (PcbLib vs PcbDoc).

### Pad Binary Layout (V3)

**Decompiled from: `FUN_018fac50` (TPadsSection_V3 Read)**

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   14    Common fields            Base class read
+0x0E   4     XLocation                VMT method (vtable+0xb0 on source object)
+0x12   4     YLocation                VMT method (vtable+0xb8 on source object)
+0x16   4     TopSizeX                 FUN_016bdb60 (multi-field reader)
+0x1A   4     TopSizeY                 FUN_016bdb60
+0x1E   4     MidSizeX                 FUN_016bdb60
+0x22   4     MidSizeY                 FUN_016bdb60
+0x26   4     BottomSizeX              FUN_016bdb60
+0x2A   4     BottomSizeY              FUN_016bdb60
+0x2E   4     HoleSize                 FUN_01811240
+0x32   1     TopShape                 FUN_016bdb60 (default: 1=Round)
+0x33   1     MidShape                 FUN_016bdb60 (default: 1=Round)
+0x34   1     BottomShape              FUN_016bdb60 (default: 1=Round)
+0x35   4     Designator (4-byte hash) FUN_018115b0 -> string -> FUN_00415360
+0x3A   6     Rotation (Extended)      FUN_018121e0 -> FUN_013e5870 (Delphi Extended->f64)
+0x??   4     StackMode                FUN_01811750 -> FUN_01905b50
+0x41   1     IsPlated                 FUN_01812120
+0x42   40    PadCache                 FUN_01818a60 -> FUN_01400c10 (5x8-byte fields)
```

**Pad shape/size reader (`FUN_016bdb60`):** This complex function determines which layer's pad is being described and reads 3 groups of (SizeX, SizeY, Shape) for Top, Mid, and Bottom layers. It uses the current layer context to decide which group to populate:
- If on the reference layer: populate Top fields
- If on the opposite surface layer: populate Bottom fields
- Otherwise: populate Mid fields

### Key Conversion Functions

| Function | Purpose | Input -> Output |
|---|---|---|
| FUN_013e5870 | Convert Delphi Extended (Real48) to IEEE double | 6 bytes -> 8 bytes f64 |
| FUN_013ddb40 | Convert Altium layer ID to binary layer byte | int -> u8 |
| FUN_013de1e0 | Convert pad shape enum to byte | int -> u8 |
| FUN_01905ad0/ab0/ac0 | Store flag/ID fields into record buffer | Various |
| FUN_01905b10/b00/b30/b50 | Store SubnetID/UnionIndex/StackMode | Various |
| FUN_0041b3f0 | OleCheck (HRESULT error checking) | HRESULT -> void |
| FUN_0041b010 | COM interface assignment (IntfAssign) | IUnknown -> void |

### Relationship: BinaryLoader <-> Advpcb.dll

The BinaryLoader reads fields from Advpcb.dll PCB objects through two mechanisms:

1. **Direct VMT calls** (Arc, Pad, Via): `(**(code **)(*param_2 + offset))(param_2)` - calls virtual methods directly on the Advpcb.dll Delphi object's VMT. Shared offsets across types: vtable+0xb0 = GetLocationX, vtable+0xb8 = GetLocationY.

2. **COM interface wrapper calls** (Track): `(**(code **)(**(param_2 + 0x60) + offset))(*(param_2 + 0x60))` - accesses a COM sub-interface at param_2+0x60, then calls IPCB_Track-specific methods. This uses the full 130+N method IPCB_Primitive/IPCB_Track vtable.

The COM vtable offset for IPCB_Track methods:
- IPCB_Primitive: 130 methods (offsets 0x18 through 0x440 after 3 IUnknown methods)
- First IPCB_Track method: offset 0x448 = GetState_X1
- 0x450 = GetState_Y1, 0x458 = GetState_X2, 0x460 = GetState_Y2, 0x468 = GetState_Width

### Additional V3 Section Classes (from TraceSerialize8.java scan)

| Class | classRef | Read Method | Write Method |
|---|---|---|---|
| TTextsSection | 0x018f26e8 | FUN_018fb1d0 | FUN_018fb340 |
| TFillsSection | 0x018f2920 | FUN_018fb600 | FUN_018fb6b0 |
| TConnectionsSection | 0x018f2d88 | FUN_018fbbb0 | FUN_018fbc90 |
| TNetsSection | 0x018f3668 | FUN_018fdca0 | FUN_018fdd90 |
| TDimensionsSection | 0x018f2fc0 | FUN_018fd230 | FUN_018fd300 |
| TComponentsSection | 0x018f3430 | FUN_018fd830 | FUN_018fd9d0 |
| TPolygonsSection | 0x018f3898 | FUN_018fdfc0 | FUN_018fe460 |
| TClassesSection | 0x018f3ad0 | FUN_018fe930 | FUN_018fead0 |
| TEmbeddedsSection | 0x018f3d08 | FUN_018fecc0 | FUN_018fee80 |
| TRulesSection | 0x018f4178 | FUN_018ffcf0 | FUN_018ffec0 |
| TRegionsSection | 0x0194a9c0 | FUN_018825f0 | FUN_0189fb10 |

### Net/Group Common Header (FUN_018f86c0, 23 bytes)

Nets, Components, Dimensions, and Polygons use a different header reader than normal primitives. This header is 23 bytes (0x17), read by `FUN_018f86c0`:

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   1     Visible                  FUN_01905c20 from FUN_0184d3a0 (COM vtable+0x30 via param+0x28)
+0x01   1     PrimitiveLock            FUN_01905bf0 from FUN_0184f330 (COM vtable+0xf0 via param+0x28)
+0x02   1     Layer                    FUN_013ddb40(FUN_0184ccb0) - layer ID conversion
+0x03   1     Unknown                  FUN_01905c00 from vtable+0x60 on param_2
+0x04   2     NetIndex                 vtable+0x50 on param_2 (2 bytes)
+0x06   4     BoundingRect X1          FUN_0184fde0 (COM vtable+0x390 via param+0x60, dword 0)
+0x0A   4     BoundingRect Y1          FUN_0184fe10 (COM vtable+0x390 via param+0x60, dword 1)
+0x0E   4     BoundingRect X2          FUN_0184fd80 (COM vtable+0x390 via param+0x60, dword 2)
+0x12   4     BoundingRect Y2          FUN_0184fdb0 (COM vtable+0x390 via param+0x60, dword 3)
+0x16   1     ConnectsVisible          FUN_01905c10 from FUN_018502a0 (COM vtable+0x458 via param+0x60)
```

The bounding rectangle fields (X1/Y1/X2/Y2) all come from the same COM method at vtable+0x390, which returns a 16-byte struct. Each helper function extracts a different 4-byte word from the result. The ConnectsVisible field at +0x16 is only meaningful for Nets.

### Text Binary Layout (V3)

**Decompiled from: `FUN_018fb1d0` (TTextsSection Read)**

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   14    Common fields            FUN_018f7ae0 (primitive common header)
+0x0E   4     LocationX                vtable+0xb0 (shared GetLocationX)
+0x12   4     LocationY                vtable+0xb8 (shared GetLocationY)
+0x16   4     Height                   FUN_01815320
+0x1A   2     StrokeFont               FUN_01823670 (returns short)
+0x1C   6     Rotation (Extended)      FUN_01816e50 -> FUN_013e5870 (Delphi Extended->f64)
+0x22   1     Mirrored                 FUN_018239b0 -> FUN_01905bb0
+0x23   255   Text (string, 0xFF max)  FUN_01815450 -> FUN_00416970 (copy short string)
+0x123  4     StrokeWidth              FUN_018154f0
+0x127  ?     Flags via FUN_01823810   FUN_01905b90, FUN_01905ba0
```

**Comparison with Rust `PcbTextBaseBinary`:**
The Rust implementation reads: `PcbPrimitiveCommon(13b) + Corner1(8b) + Height(4b) + StrokeFont(2b) + Rotation(8b f64) + Mirrored(1b) + StrokeWidth(4b)`.

Key discrepancies:
1. **Common header**: 13 bytes (Rust) vs 14 bytes (decompiled). Rust is off-by-one.
2. **Corner1 vs Location**: Rust reads Corner1 as a CoordPoint(x,y), decompiled reads LocationX/LocationY. Semantically the same.
3. **Rotation**: Rust reads 8 bytes (f64), decompiled reads 6 bytes (Delphi Extended/Real48). This is the V3 format difference.
4. **Text string**: The V3 format has a 255-byte inline text string at +0x23. The current Rust implementation does not include this inline text; it reads text from a separate string block. This is likely because the Rust parser handles a newer format where text is stored separately.
5. **StrokeWidth location**: At +0x123 in V3 (after the 255-byte text), at +0x24 in Rust (immediately after mirrored).

### Fill Binary Layout (V3)

**Decompiled from: `FUN_018fb600` (TFillsSection Read)**

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   14    Common fields            FUN_018f7ae0 (primitive common header)
+0x0E   4     Corner1X                 FUN_01816e80
+0x12   4     Corner1Y                 FUN_01816ee0
+0x16   4     Corner2X                 FUN_01816eb0
+0x1A   4     Corner2Y                 FUN_01816f10
+0x1E   6     Rotation (Extended)      FUN_01816e50 -> FUN_013e5870 (Delphi Extended->f64)
```

**Comparison with Rust `PcbRectangularBase` (used by `PcbFill`):**
The Rust reads: `PcbPrimitiveCommon(13b) + Corner1(8b) + Corner2(8b) + Rotation(8b f64)`.

Key discrepancies:
1. **Common header**: 13 vs 14 bytes (same off-by-one as Text).
2. **Rotation**: 8 bytes f64 (Rust) vs 6 bytes Extended (V3). Same discrepancy as Arc/Text.
3. **Total size**: 37 bytes (Rust) vs 36 bytes (V3 decompiled: 14+4+4+4+4+6). The 2-byte Rotation size difference and 1-byte header difference nearly cancel out.

### Connection Binary Layout (V3)

**Decompiled from: `FUN_018fbbb0` (TConnectionsSection Read)**

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   14    Common fields            FUN_018f7ae0 (primitive common header)
+0x02   1     Flags override (=0x22)   Hardcoded (overwrites flags byte 2)
+0x0E   4     FromX                    FUN_01815080
+0x12   4     FromY                    FUN_018150e0
+0x16   4     ToX                      FUN_018150b0
+0x1A   4     ToY                      FUN_01815110
+0x1E   1     FromLayer                FUN_01813c30 -> FUN_013ddb40 (layer ID convert)
+0x1F   1     ToLayer                  FUN_01814640 -> FUN_013ddb40 (layer ID convert)
```

Total: 32 bytes (0x20).

**Comparison with Rust `PcbConnection` (43 bytes hardcoded):**
The Rust implementation reads 43 bytes as a fixed-size record. The V3 decompiled format shows only 32 bytes of meaningful data. The Rust parser places coordinates at offsets 8-23 (after skipping 8 bytes for net+padding), but the decompiled layout shows coordinates at offsets 14-29 (after the 14-byte common header).

Key discrepancies:
1. **Size**: 43 bytes (Rust) vs 32 bytes (V3 decompiled).
2. **Coordinate layout**: Rust skips to position 8 for From X/Y, decompiled starts at +0x0E.
3. **Layer fields**: The V3 format has FromLayer and ToLayer as single bytes at +0x1E and +0x1F. The Rust implementation does not parse layer fields.
4. **Flags override**: V3 hardcodes byte at +0x02 to 0x22 after writing common header.
5. **Component/pad indices**: The Rust implementation reads component and pad indices from fixed positions (28, 32, 35, 39). These are not in the V3 decompiled format, suggesting they may be in a newer format version or stored differently.

### Net Binary Layout (V3)

**Decompiled from: `FUN_018fdca0` (TNetsSection Read)**

Uses the **Net/Group header** (FUN_018f86c0, 23 bytes) instead of the primitive common header.

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   23    Net/Group header         FUN_018f86c0 (see Net/Group Common Header above)
+0x17   4     Color                    FUN_01816fe0
+0x1B   20    Name (string, 0x14 max)  FUN_018170e0 -> FUN_013ddf20 -> FUN_00415360
+0x2F   ?     Flags                    FUN_01817010 -> FUN_01905c80
```

**Comparison with Rust `PcbNet`:**
The Rust implementation uses parameter-based parsing (from_params), not binary parsing. It reads from a ParameterCollection with keys like NAME, COLOR, LAYER, etc. This is the correct approach for PcbDoc files, which store nets as parameter strings in the `Nets6/Data` stream. The V3 binary layout above is for the legacy compact binary format used internally by BinaryLoader.

### Dimension Binary Layout (V3)

**Decompiled from: `FUN_018fd230` (TDimensionsSection Read)**

Uses the **Net/Group header** (23 bytes).

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   23    Net/Group header         FUN_018f86c0
+0x17   4     LocationX                vtable+0xb0 (shared GetLocationX)
+0x1B   4     LocationY                vtable+0xb8 (shared GetLocationY)
+0x1F   4     X1/RefPoint1X            FUN_016b7580 (COM vtable+0x4f8)
+0x23   4     Y1/RefPoint1Y            FUN_016b7610 (COM vtable+0x500)
+0x27   4     X2/RefPoint2X            FUN_016b6bf0 (COM vtable+0x508)
+0x2B   4     Y2/RefPoint2Y            FUN_016b6ad0 (COM vtable+0x510)
+0x2F   4     Height                   FUN_016b6f80 (COM vtable+0x518)
+0x33   4     LineWidth                FUN_016b7340 (COM vtable+0x520)
+0x37   4     TextHeight (int from 2b) FUN_016b6da0 (COM vtable+0x528, returns short, cast to int)
+0x3B   1     DimensionKind            FUN_016b6c80 (COM vtable+0x5b0)
```

Total: 60 bytes (0x3C).

**Comparison with Rust `PcbDimension`:**
The Rust implementation uses parameter-based parsing (from_params), reading from a ParameterCollection with keys like X1, Y1, X2, Y2, HEIGHT, LINEWIDTH, DIMENSIONKIND, etc. It has many more fields than the V3 binary format captures (text properties, arrow properties, extension line properties, references, font settings). The V3 binary format is a compact representation with only the essential geometry.

### Component Binary Layout (V3)

**Decompiled from: `FUN_018fd830` (TComponentsSection Read)**

Uses the **Net/Group header** (23 bytes).

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   23    Net/Group header         FUN_018f86c0
+0x17   4     Channel OffsetX?         FUN_018294a0 (COM vtable+0x818 via param+0x68)
+0x1B   4     Channel OffsetY?         FUN_018294f0 (COM vtable+0x828 via param+0x68)
+0x1F   4     Unknown Coord 1          FUN_01829540 (COM vtable+0x838 via param+0x68)
+0x23   4     Unknown Coord 2          FUN_01829590 (COM vtable+0x848 via param+0x68)
+0x27   4     LocationX                vtable+0xb0 (shared GetLocationX)
+0x2B   4     LocationY                vtable+0xb8 (shared GetLocationY)
+0x2F   255   Designator (0xFF max)    FUN_0180d370 (COM vtable+0x500 via param+0x68) -> FUN_00416970
+0x12F  1     DesignatorOn             FUN_01905c60 from FUN_0180d340 (COM vtable+0x508)
+0x130  1     CommentOn                FUN_01905c50 from FUN_0180f4f0 (COM vtable+0x510)
+0x131  2     SourceLibraryIndex       FUN_0180fa30 (COM vtable+0x520 via param+0x68)
+0x133  2     SourceDesignatorCount    FUN_0180f550 (separate COM query via FUN_0041b060)
+0x135  6     Rotation (Extended)      FUN_0180d410 (COM vtable+0x528) -> FUN_013e5870
+0x13B  4     Fixed value (0x24242403) Hardcoded
```

Total: 319 bytes (0x13F).

**Comparison with Rust `PcbComponent`:**
The Rust implementation uses parameter-based parsing (import_from_parameters), reading PATTERN, HEIGHT, DESCRIPTION, etc. The V3 binary format contains the compact representation with a 255-byte inline designator string and specific field offsets. The Rust implementation stores primitives in a Vec and metadata separately.

### Polygon Binary Layout (V3)

**Decompiled from: `FUN_018fdfc0` (TPolygonsSection Read)**

Uses the **Net/Group header** (23 bytes). The polygon format has a **variable-length body** with an array of outline vertices.

**Fixed header:**
```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   23    Net/Group header         FUN_018f86c0
+0x17   2     NetIndex                 vtable+0x50 on inner (default: 0xFFFF)
+0x19   1     IsSplitPlane             0=normal polygon, 1=split plane
+0x1A   1     PourOver                 FUN_01905cf0 from FUN_0181b950 (COM vtable+0x4f0)
+0x1B   1     RemoveDead               FUN_01905d00 from FUN_0181bb10 (COM vtable+0x4f8?)
+0x1C   1     IsKeepout                FUN_01905ce0 from vtable+0x248
+0x1D   4     GridSize                 FUN_018107d0 (COM vtable+0x510, default: 200000 for split plane)
+0x21   4     TrackWidth               FUN_0181bae0 (COM vtable+0x518, default: 80000 for split plane)
+0x25   4     MinPrimLength            FUN_01810860 (COM vtable+0x520, default: 30000 for split plane)
+0x29   1     HatchStyle               FUN_01810c20 (COM vtable+0x538, default: 5 for split plane)
+0x2A   1     UseOctagons              FUN_01905cd0 from vtable+0x238
+0x2B   2     VertexCount              FUN_013fe030 on outline object
```

**Vertex array (0x21 = 33 bytes per vertex):**
```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   1     VertexKind               FUN_013fe0f0 -> uStack_60 (0=Line, 1=Arc)
+0x01   4     VX (vertex X)            FUN_013fe0f0 -> uStack_5f
+0x05   4     VY (vertex Y)            FUN_013fe0f0 -> uStack_5b
+0x09   4     CX1                      FUN_013fe0f0 -> uStack_57
+0x0D   4     CY1                      FUN_013fe0f0 -> uStack_53
+0x11   4     CX2/Radius               FUN_013fe0f0 -> uStack_4f
+0x15   6     Angle1 (Extended)        FUN_013fe0f0 -> FUN_013e5870 (Delphi Extended->f64)
+0x1B   6     Angle2 (Extended)        FUN_013fe0f0 -> FUN_013e5870
```

Total per vertex: 33 bytes. Total polygon record: `45 + VertexCount * 33` bytes.

**Split plane default values:** When the polygon is a split plane (IsSplitPlane=1), the GridSize, TrackWidth, MinPrimLength, and HatchStyle are set to hardcoded defaults: 200000, 80000, 30000, and 5 respectively. PourOver and RemoveDead are set to 0.

**Comparison with Rust `PcbPolygon`:**
The Rust implementation uses parameter-based parsing (from_params), reading from a ParameterCollection with keys like LAYER, GRIDSIZE, TRACKWIDTH, HATCHSTYLE, and indexed vertex keys (KIND0, VX0, VY0, CX0, CY0, SA0, EA0, R0). The V3 binary vertex format has 7 fields per vertex matching the parameter-based format: Kind, VX, VY, CX, CY, plus two angle fields stored as 6-byte Extended values. The parameter-based Rust implementation also reads a Radius field (R0) which is not explicitly separate in the V3 binary format (it may be one of the CX1/CX2 fields).

### Summary of Format Differences: V3 Binary vs Current Rust Implementation

| Aspect | V3 Binary (BinaryLoader) | Rust Implementation |
|--------|--------------------------|---------------------|
| Common header | 14 bytes (layer + flags + 5x u16 indices) | 13 bytes (layer + flags + 10x 0xFF) |
| Rotation encoding | 6 bytes (Delphi Extended/Real48) | 8 bytes (IEEE f64) |
| String encoding | Inline fixed-length (255 bytes typical) | Separate string blocks or parameters |
| Net/Group header | 23 bytes (visible, flags, layer, bounds, etc.) | Parameter-based (from_params) |
| Net/Component/Dimension/Polygon | Binary V3 format | Parameter-based format |
| Track/Arc/Via/Pad/Fill/Text/Connection | Binary V3 format | Binary format (newer version) |

The V3 format is the **legacy compact binary format** used by older versions of Altium. Modern PcbDoc files use a combination of:
1. **Binary records** for primitive types (Track, Arc, Via, Pad, Fill, Text, Connection) stored in streams like `Tracks6/Data`, using a newer format with IEEE f64 instead of Real48.
2. **Parameter-based records** for container/metadata types (Net, Component, Dimension, Polygon, Class, Rule, etc.) stored as pipe-delimited parameter strings.

The key architectural insight is that the BinaryLoader.dll serves as a **format compatibility layer** - it reads V3/V4 format data and presents it through COM interfaces, which the rest of the application accesses uniformly regardless of the on-disk format version.

### Coordinate V3 Section -- Does Not Exist

No `TCoordinateSection` or `TCoordinatesSection` class exists in BinaryLoader.dll. String searches for "TCoordinate" returned zero results. The TCoordinate class (PCB object type 14, inherits TGroup) does not have a dedicated V3 binary section. Coordinates are either stored through the generic Section format or handled through parameter-based serialization.

### Region V3 Section -- Uses Section Format, Not V3

`TRegionsSection` exists at VMT 0x0194a9c0 but its Read method is `FUN_018825f0` -- the **shared Section-format reader** (same as TTracksSection_Sect, TArcsSection_Sect, TViasSection_Sect). This is NOT a V3 binary format. It uses an iterator pattern:

1. Call `FUN_018a0320` to initialize iteration
2. Loop: get next object via COM vtable+0x28, process via `FUN_01882500`
3. Also iterate through a secondary list at `param_1+0x50` using `FUN_016e6810` iterator
4. Finalize via `FUN_018a0c40`

Regions do not have a V3 compact binary format -- they only support the newer Section format.

### Embedded Binary Layout (V3)

**Section class:** TEmbeddedsSection (VMT 0x018f3d08)
**Read:** FUN_018fecc0 | **Write:** FUN_018fee80

The Embedded section does NOT use the standard 14-byte or 23-byte common headers. It has a custom layout specific to embedded objects.

**Record buffer layout (fixed part: 0x106 = 262 bytes):**

```
Offset  Size  Field                    Source (Read)
------  ----  -----                    ------
+0x00   1     Visible                  FUN_01905d50 from FUN_0184d3a0 (COM vtable+0x30 via param+0x28)
+0x01   1     PrimitiveLock            FUN_01905d30 from FUN_0184f330 (COM vtable+0xf0 via param+0x28)
+0x02   1     ObjectType               FUN_013ddb40(FUN_00df1a40(FUN_00df3a90(FUN_0184ccb0(param))))
                                       Chain: COM vtable+0x428 -> variant type decode -> type mapping
+0x03   1     UnknownFlag              FUN_01905d40 from vtable+0x60 on param_2
+0x04   256   Name (ShortString)       FUN_01821cf0 -> COM vtable+0x448 -> FUN_00415330
                                       Pascal ShortString: byte[0]=length, byte[1..]=chars
+0x104  2     ChildCount               From FUN_01821c30 (COM vtable+0x450): array length + 1
```

**Variable-length data (follows fixed part):**

After the fixed 262-byte record, the section writes:
1. A u16 section index/count (from context+0x100 in section header)
2. A variant data blob read via `FUN_00450590` -- this is the serialized embedded child data (byte array from COM vtable+0x450)
3. Another u16 count value (array length + 1 from the variant data)

**Write method field order (cross-validation):**

The Write method (FUN_018fee80) reads the record buffer fields in the following order, confirming the layout:
1. Read u16 section index from context (FUN_01900f80)
2. Get record buffer from context+0x2038
3. Create embedded object (FUN_0184b430 with class _UNK_01801098 = TEmbedded)
4. Set Visible: read bool at buffer+0 (FUN_01905d20)
5. Convert ObjectType: read byte at buffer+2, apply FUN_013dd7d0 (reverse mapper), switch on 83 possible type codes to set component type string
6. Set UnknownFlag: read bool at buffer+3 (FUN_01905d10)
7. Write Name: copy ShortString from buffer+4 via FUN_01821ea0
8. Send object to output stream
9. Write ChildCount: read u16 at buffer+0x104 (FUN_01900f80)
10. Write variant data blob (FUN_01821dc0)

**ObjectType mapping (FUN_013ddb40, byte -> internal type):**

The ObjectType byte at +0x02 maps through FUN_013ddb40, a sparse mapping:
- 0x00-0x0F: identity (maps to self)
- 0x10->0x20, 0x11->0x21, ..., 0x15->0x25
- 0x16->0x26, 0x17->0x27, 0x18->0x28, 0x19->0x29, 0x1a->0x2a
- 0x1b->0x37, 0x1c->0x38, 0x1d->0x39, 0x1e->0x3a, 0x1f->0x3b, 0x20->0x3c
- 0x21->0x49, 0x22->0x4a, ..., 0x2a->0x52

The reverse mapping (FUN_013dd7d0, used in Write) is the inverse. The Write method also has an 83-entry switch statement that converts the internal type ID to a component type string via helper functions (FUN_00df1d20 through FUN_00df3540 and FUN_00df4320/FUN_00df4490 with various parameters).

### Classes Binary Layout (V3)

**Section class:** TClassesSection (VMT 0x018f3ad0)
**Read:** FUN_018fe930 | **Write:** FUN_018fead0

The Classes section does NOT serialize individual fields into a flat binary record. Instead, it uses the **generic COM persistence framework** to serialize entire class objects through FUN_016ba9b0 -- a giant dispatcher with 100+ type checks.

**Read method (FUN_018fe930) flow:**

```
1. Get context via FUN_018f5f30(param_1) -> param_1+0x10
2. Get COM class iterator at context+0x2040
3. Clear iterator: call vtable+0xa0 on iterator object, OleCheck result
4. Read class data: FUN_01851900(param_2, context+0x2040)
   -> Delegates to FUN_016ba970 -> FUN_016ba9b0 (giant type dispatcher)
   -> 100+ class type checks, each dispatches to appropriate serializer method
5. Get record buffer at context+0x2038
6. Set buffer[0..2] = 0 (reset first 2-byte field)
7. Get count: call vtable+0x50 on iterator -> aiStack_20[0] (int), OleCheck result
8. Set buffer[2..4] = count + 1 (2-byte field: number of children + 1)
9. Write section index via FUN_01900fc0 (from section header at context+0x100)
10. Get serialized class data from iterator: call vtable+0x60, OleCheck result
11. Copy serialized data: FUN_00415a80 -> FUN_00450460 into record buffer
12. Read another count via vtable+0x50 + 1, write via FUN_01900fc0
```

**Record buffer layout (at context+0x2038):**

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   2     Reserved (=0)            Hardcoded to 0 during Read
+0x02   2     ChildCount               Iterator vtable+0x50 result + 1
```

Followed by **variable-length serialized class data**, stored through the generic COM persistence framework (FUN_016ba9b0). The actual class data is opaque from the section's perspective -- the COM persistence framework handles encoding/decoding based on the Delphi class type of the object.

**Write method (FUN_018fead0) flow:**

1. Write section index from context+0x100 (FUN_01900f80)
2. Get record buffer from context+0x2038
3. Create class object via FUN_018515d0(_UNK_018492c8, 1, 0) -- creates appropriate class instance
4. Write ChildCount: read u16 at buffer+2 (FUN_01900f80)
5. Load class data: call vtable+0x28 on iterator (context+0x2040) with record buffer data, OleCheck
6. Send object to output stream via vtable+0x48
7. If object is non-null:
   a. Call vtable+0x148 on object (set board reference from context+8)
   b. Write object back to iterator via FUN_01851920

**Key insight:** The Classes V3 section is a **thin wrapper** around the generic COM persistence framework. The record buffer contains only minimal metadata (2 reserved bytes + 2-byte count). The actual class properties (name, member list, etc.) are serialized/deserialized by the COM persistence dispatcher (FUN_016ba9b0/FUN_016ba990), which handles 100+ different Delphi class types through a massive type-checking if/else chain.

### Rules Binary Layout (V3)

**Section class:** TRulesSection (VMT 0x018f4178)
**Read:** FUN_018ffcf0 | **Write:** FUN_018ffec0

**The V3 Rules Read method is a NO-OP.** The decompiled function at FUN_018ffcf0 is:

```c
void FUN_018ffcf0(void) {
    return;
}
```

This means V3 Rules sections **cannot be read** -- they can only be written. This suggests that V3 Rules were write-only for export purposes, or that Rules reading was implemented at a higher level and the V3 section only handled writing.

**Write method (FUN_018ffec0) flow:**

```
1. Write section index from context+0x100 (FUN_01900f80)
2. Get record buffer from context+0x2038
3. Create rule object via FUN_015d4a00(buffer[0], 0)
   -> Factory function: lookup table at 0x1d46ab8
   -> Index by RuleType byte (buffer[0]), range 0x00-0x45 (70 rule types)
   -> If table[ruleType] is non-null: creates new instance via class VMT
   -> If null: returns 0 (no rule created)
4. Write ChildCount: read u16 at buffer+2 (FUN_01900f80)
5. Load rule data: call vtable+0x28 on iterator (context+0x2040) with record buffer, OleCheck
6. If rule object was created (plStack_20 != null):
   a. Send object to output stream via vtable+0x48
   b. Set board reference: call vtable+0x148 on rule (from context+8)
   c. Write rule back to iterator via FUN_01851920
   d. Generate section name: FUN_018ffd00 -> rule type name + counter suffix
   e. Set rule parameters via FUN_01480490 (COM vtable+0x4c8)
7. Cleanup
```

**Record buffer layout (at context+0x2038):**

```
Offset  Size  Field                    Source
------  ----  -----                    ------
+0x00   1     RuleType                 Used by FUN_015d4a00 to create rule object
                                       Range 0x00-0x45 (70 possible rule types)
+0x02   2     ChildCount               Read via FUN_01900f80
```

**Rule type factory (FUN_015d4a00):**

Uses a global dispatch table at address 0x1d46ab8, indexed by the RuleType byte:
- Valid range: 0x00 to 0x45 (70 entries)
- Each entry is an 8-byte pointer to a Delphi class VMT
- If the table entry is non-null, creates a new instance via `(*(lVar1 + 8))(lVar1, 0x1d46a01)`
- If null, returns 0 (no rule created)

**Rule section name generator (FUN_018ffd00):**

After creating and serializing the rule object, this function generates a unique section name string:
1. Reads the rule type from the object via vtable+0x1f0
2. Looks up a counter array at `context+0x60+0x30+ruleType*4`
3. If counter is 0: uses a base name from table at 0x1dbb328 (indexed by rule type, 8 bytes per entry)
4. If counter > 0: formats as `"{basename}_{counter}"` (FUN_00416d10 with 3 args: base name, suffix format, counter)
5. Increments the counter
6. Passes the name string to the rule object via FUN_01480490 -> COM vtable+0x4c8

This generates unique section names for rules like "Clearance", "Clearance_1", "Clearance_2", etc.

### Summary: V3 Section Type Categories

| Section | Header Type | Layout | Serialization Method |
|---------|------------|--------|---------------------|
| Tracks | 14-byte primitive | Fixed binary | Direct field read/write |
| Arcs | 14-byte primitive | Fixed binary | Direct field read/write |
| Pads | 14-byte primitive | Fixed binary | Direct field read/write |
| Vias | 14-byte primitive | Fixed binary | Direct field read/write |
| Texts | 14-byte primitive | Fixed binary | Direct field read/write |
| Fills | 14-byte primitive | Fixed binary | Direct field read/write |
| Connections | 14-byte primitive | Fixed binary | Direct field read/write |
| Nets | 23-byte group | Fixed binary | Direct field read/write |
| Components | 23-byte group | Fixed binary | Direct field read/write |
| Dimensions | 23-byte group | Fixed binary | Direct field read/write |
| Polygons | 23-byte group | Variable (vertex array) | Direct field read/write |
| **Embeddeds** | **Custom (no common hdr)** | **Fixed 262B + variant blob** | **COM vtable calls + ShortString** |
| **Classes** | **None (opaque)** | **4B metadata + COM blob** | **Generic COM persistence framework** |
| **Rules** | **None (read=no-op)** | **3B metadata + COM blob** | **Rule factory + COM persistence** |
| Regions | N/A (Section format only) | N/A | Iterator-based Section reader |
| Coordinates | N/A (no V3 section exists) | N/A | No V3 support |

## V4 Binary Format (BinaryLoader.dll)

### V4 Common Header (19 bytes)

Written by `FUN_0190d0b0`, read by `FUN_0190d500`. Larger than V3's 14-byte header:

```
Offset  Size  Field                    Notes
------  ----  -----                    ------
+0x00   1     IsSelected               bool
+0x01   1     IsLocked                 bool (from InBoard/DesignLocked)
+0x02   1     Layer (encoded)          Layer ID encoded via switch table
+0x03   1     IsKeepOut                bool
+0x04   2     OwnerIndex               u16, 0xFFFF = no owner
+0x06   1     IsTentingTop             bool
+0x07   1     IsTentingSolderMaskMode  bool
+0x08   1     IsTestPoint              bool
+0x09   1     Flags                    bit 0 = IsTearDrop, bit 1 = IsPour
+0x0A   1     UnionIndex               bool (from InUnion)
+0x0B   2     NetIndex                 u16, 0xFFFF = no net
+0x0D   2     PolygonIndex             u16, 0xFFFF = none
+0x0F   2     ComponentIndex           u16, 0xFFFF = none
+0x11   2     DimensionIndex           u16, 0xFFFF = none
```

### V4 Track Layout (41 bytes)

**Read:** FUN_0190ed30 | **Write:** FUN_0190ec90

```
Offset  Size  Field
------  ----  -----
+0x00   19    V4 Common Header
+0x13   4     X1
+0x17   4     Y1
+0x1B   4     X2
+0x1F   4     Y2
+0x23   4     Width
+0x27   2     SubnetID
```

### V4 Arc Layout (49 bytes)

**Read:** FUN_01911460 | **Write:** FUN_01911370

```
Offset  Size  Field
------  ----  -----
+0x00   19    V4 Common Header
+0x13   4     CenterX
+0x17   4     CenterY
+0x1B   4     Radius
+0x1F   6     StartAngle (Extended)
+0x25   6     EndAngle (Extended)
+0x2B   4     Width
+0x2F   2     SubnetID
```

Note: V4 arcs still use 6-byte Delphi Extended for angles (same as V3).

### V4 Via Layout (80 bytes)

**Read:** FUN_0190f0b0 | **Write:** FUN_0190ef00

```
Offset  Size  Field
------  ----  -----
+0x00   19    V4 Common Header
+0x13   4     X
+0x17   4     Y
+0x1B   4     HoleSize
+0x1F   4     Diameter
+0x23   1     StartLayer (encoded)
+0x24   1     EndLayer (encoded)
+0x25   1     SolderMaskExpansionMode
+0x29   4     ThermalReliefAirGapWidth
+0x2D   2     ThermalReliefConductors
+0x31   4     ThermalReliefSpokeWidth
+0x35   4     PowerPlaneConnectStyle
+0x39   4     PowerPlaneReliefExpansion
+0x3D   4     SolderMaskExpansion
+0x41   4     PasteMaskExpansion
+0x45   2     CopperPadDiameter
+0x47   1     SolderMaskExpansionOverride
+0x48   1     ThermalReliefConductorsOverride
+0x49   1     ThermalReliefSpokeWidthOverride
+0x4A   1     PowerPlaneConnectStyleOverride
+0x4B   1     PowerPlaneReliefExpansionOverride
+0x4C   1     SolderMaskExpansionOverrideB
+0x4D   1     PasteMaskExpansionOverride
+0x4E   1     PowerPlaneClearanceOverride
+0x4F   1     CopperPadDiameterOverride
```

V4 vias carry extensive thermal/solder mask properties inline (vs V3's minimal layout).

### V4 Pad Layout (131 bytes)

**Read:** FUN_01910970 | **Write:** FUN_01910730

```
Offset  Size  Field
------  ----  -----
+0x00   19    V4 Common Header
+0x13   4     X
+0x17   4     Y
+0x1B   4     XSize (top)
+0x1F   4     YSize (top)
+0x23   4     MidXSize
+0x27   4     MidYSize
+0x2B   4     BotXSize
+0x2F   4     BotYSize
+0x33   4     HoleSize
+0x37   1     TopShape
+0x38   1     MidShape
+0x39   1     BotShape
+0x3A   20    Name (fixed string)
+0x4F   6     Rotation (Extended)
+0x55   4     PlatedFlag
+0x56   1     SolderPasteOverride
+0x57   44    ViaProperties (thermal/solder mask block from sub-object)
```

### V3 vs V4 Comparison

| Aspect | V3 | V4 |
|--------|----|----|
| Common header | 14 bytes | 19 bytes |
| Layer encoding | Direct enum | Switch table encoded |
| Additional flags | None | IsTentingTop, IsTearDrop, IsPour, UnionIndex |
| Track record | ~30 bytes | 41 bytes |
| Arc angles | 6-byte Extended | 6-byte Extended (same) |
| Via record | ~33 bytes | 80 bytes (extensive thermal/mask data) |
| Pad record | ~106 bytes | 131 bytes (multi-layer shapes + via sub-properties) |

### V4 VMT Architecture

V4 section classes share `FUN_0190d030` as read dispatcher and have per-record handlers:

| VMT Offset | Purpose |
|-----------|---------|
| +0x78 | V4 Write dispatcher (iterates objects, calls per-record write) |
| +0x80 | V4 Read dispatcher (`FUN_0190d030`, iterates records) |
| +0x88 | Per-record Write handler (type-specific) |
| +0x90 | Per-record Read handler (type-specific) |

## Section Format (Modern PcbDoc, "Tracks6/Data" etc.)

### Architecture

The Section format is the **latest format** used by modern PcbDoc files (Altium Designer 6+). All section types inherit from `TPrimitivesSection`.

### OLE Compound File Structure

Each section occupies a **sub-storage** within the PcbDoc OLE compound document:

```
PcbDoc OLE Root
├── FileHeader          (version info)
├── FileVersionInfo     (timestamps)
├── Board6/
│   ├── Header          (4-byte record count, LE u32)
│   └── Data            (serialized records)
├── Tracks6/
│   ├── Header
│   └── Data
├── Arcs6/
│   ├── Header
│   └── Data
├── Pads6/
│   ├── Header
│   └── Data
├── Vias6/
│   ├── Header
│   └── Data
├── Texts6/
│   ├── Header
│   └── Data
├── Fills6/
│   ├── Header
│   └── Data
├── Nets6/
│   ├── Header
│   └── Data
├── Components6/
│   ├── Header
│   └── Data
├── Polygons6/
│   ├── Header
│   └── Data
├── Dimensions6/
│   ├── Header
│   └── Data
├── Coordinates6/
│   ├── Header
│   └── Data
├── Classes6/
│   ├── Header
│   └── Data
├── Rules6/
│   ├── Header
│   └── Data
├── Connections6/
│   ├── Header
│   └── Data
├── FromTos6/
│   ├── Header
│   └── Data
├── DifferentialPairs6/
│   ├── Header
│   └── Data
├── Embeddeds6/
│   ├── Header
│   └── Data
├── EmbeddedBoards6/
│   ├── Header
│   └── Data
├── ShapeBasedRegions6/
│   ├── Header
│   └── Data
├── Regions6/
│   ├── Header
│   └── Data
├── ShapeBasedComponentBodies6/
│   ├── Header
│   └── Data
├── ComponentBodies6/
│   ├── Header
│   └── Data
├── WideStrings6/       (externalized string data)
├── EmbeddedFonts6/
└── Advanced Placer Options6/
```

### TPrimitivesSection Class Hierarchy (26 Section Types)

All sections share a common `TPrimitivesSection` base (instance size 0xB0 = 176 bytes):

```
TPrimitivesSection (base, 0xB0)
  ├── TTracksSection, TArcsSection, TFillsSection, TFromTosSection,
  │   TDifferentialPairsSection, TConnectionsSection, TNetsSection,
  │   TComponentsSection, TTextsSection, TCoordinatesSection,
  │   TEmbeddedBoardsSection, TClassesSection, TEmbeddedsSection,
  │   TSmartUnionsSection, TSignalClassesSection, TxNetClassesSection,
  │   TViasSection                                           (all 0xB0)
  ├── TDimensionsSection, TPolygonsSection                   (0xC0, +16)
  ├── TPadsSection, TShapeBasedComponentBodySection,
  │   TComponentBodySection, TMechanicalPrimitivesSection     (0xB8, +8)
  ├── TRegionsSection, TRulesSection                         (0xC8, +24)
  └── TAbstractViolationSection                              (0xD0, +32)
```

### Shared Read Method (FUN_018825f0)

```c
void TPrimitivesSection_ReadFromStorage(TPrimitivesSection* self) {
    // Phase 1: Open "Header" and "Data" streams from OLE storage
    InitializeStreams(self);                       // FUN_018a0320

    self->currentRecordIndex = 0;

    // Phase 2: Iterate primary records via iterator at self+0xA0
    record = self->recordIterator->First();        // vtable+0x28
    while (record != NULL) {
        ProcessSingleRecord(self, record);         // FUN_01882500
        record = self->recordIterator->Next();     // vtable+0x30
    }

    // Phase 3: Iterate secondary record list at self+0x50
    if (self->secondaryList != NULL) {
        iter = CreateEnumerator(self->secondaryList);
        while (iter->MoveNext()) {
            record = iter->Current();
            ProcessSingleRecord(self, record);
        }
    }

    // Phase 4: Finalize
    FinishRead(self);                              // FUN_018a0c40
}
```

### Large Data Externalization (32,000 byte threshold)

Records with data streams exceeding 32,000 bytes are externalized into separate OLE sub-streams. Five data stream types per record:

| Type Code | Description |
|-----------|-------------|
| 8 | Main binary data |
| 9 | Additional binary data |
| 10 | Parameter string data |
| 13 | Extra data stream |
| 14 | Polygon/region outline data |

After initial reading, `ProcessRecords` (FUN_018a11a0) restores externalized data from these separate streams back into the records.

### Section Format Method Overrides

Most section types use all inherited/shared methods. Notable overrides:

| Section Type | Override |
|-------------|----------|
| TPadsSection | Sets flag at +0xB0 before/after reading |
| TViasSection | Additional storage validation before write |
| TPolygonsSection | Two-pass read (type 0x0A then type 0x16 records) |
| TRegionsSection | Manages two extra object lists at +0xA8 and +0xB0 |
| TRulesSection | Manages two extra object lists |
| TAbstractViolationSection | Completely custom read/write |
| TMechanicalPrimitivesSection | Conditional read with feature check |

## OLE Stream Loading Architecture (BinaryLoader.dll)

### File Opening

BinaryLoader supports two file-open strategies:

1. **Direct OLE**: `StgOpenStorage` on filename (standard PcbDoc files)
2. **Custom ILockBytes**: `CreateFileW` → custom `ILockBytes` wrapper → `StgOpenStorageOnILockBytes` (for files needing custom I/O)

```
FUN_0185c150 (OpenForRead) / FUN_0185c0e0 (OpenForWrite)
    → FUN_01859740 (creates TStorageManager wrapper)
        → StgOpenStorage (direct) OR
        → CreateFileW + StgOpenStorageOnILockBytes (custom)
```

### Stream Name Tables

Three initialization functions populate a global stream name table (BSS at `0x01e91b98`):

| Function | Purpose | Stream Names |
|----------|---------|--------------|
| `FUN_0186b060` | Base/V3 init | "Board", "Tracks", "Arcs", "Pads", ... (85+ entries) |
| `FUN_0186a210` | V6/PcbDoc override | "Board6", "Tracks6", "Arcs6", "Pads6", ... (34 entries) |
| `FUN_0186a9f0` | PcbLib variant | "Track", "Arc", "Fill", "Region", ... (29 entries, singular) |

Base init runs first; V6 or PcbLib override selectively replaces entries based on file format.

### V6/PcbDoc Stream Name Table

| Index | Stream Name |
|-------|-------------|
| 0 | Board6 |
| 5 | Classes6 |
| 6 | Nets6 |
| 7 | Components6 |
| 8 | Polygons6 |
| 9 | Dimensions6 |
| 10 | Coordinates6 |
| 11 | EmbeddedBoards6 |
| 12 | Connections6 |
| 13 | Rules6 |
| 15 | FromTos6 |
| 16 | DifferentialPairs6 |
| 17 | Embeddeds6 |
| 18 | Arcs6 |
| 19 | Pads6 |
| 20 | Vias6 |
| 21 | Tracks6 |
| 22 | Texts6 |
| 23 | Fills6 |
| 24 | ShapeBasedRegions6 |
| 25 | Regions6 |
| 26 | ShapeBasedComponentBodies6 |
| 27 | ComponentBodies6 |
| 29 | WideStrings6 |
| 30 | EmbeddedFonts6 |

### Format Version Detection

```
1. Open root OLE storage (StgOpenStorage)
2. Open "FileHeader" stream
3. Read version identifier (4-byte length + ShortString)
4. Compare against expected version via virtual method
5. Select V3, V6/PcbDoc, or PcbLib stream name table accordingly
```

### Overall Loading Sequence

```
1. GetStorageManager (export 0x01b774f0)
   → Creates singleton storage manager, returns COM interface

2. Create PCB Document (FUN_01923210)
   → Document object layout:
     +0x18..0x31: 26 boolean section-loaded flags
     +0x40: filename (UnicodeString)
     +0x48: current OLE storage
     +0x228: section list/array

3. Read File (FUN_01923680)
   a. Open OLE compound file → store root IStorage at +0x48
   b. Open "FileHeader" stream → read/validate version
   c. Read "FileVersionInfo" stream → timestamps

4. Read All Sections (FUN_01923b10)
   For each section i = 0..N:
     a. Get section object from storage manager
     b. Call section's virtual Read method
     c. Section opens "{Name6}/Header" → reads record count (u32)
     d. Section opens "{Name6}/Data" → reads record data
     e. Per-record processing via iterator or binary reader

5. Find Section by Name (FUN_01923c50)
   → Iterates all sections, compares names via UStrCompare
```

### Key Function Reference

| Address | Function | Purpose |
|---------|----------|---------|
| `0x01b774f0` | GetStorageManager | DLL export, singleton storage manager |
| `0x0185c150` | OpenStorageForRead | Opens OLE compound file |
| `0x0185a220` | OpenStreamForRead | Opens named stream within storage |
| `0x0185b3a0` | ParseStreamPath | Splits "Section\\SubStream" and navigates sub-storages |
| `0x01923210` | PCBDoc.Constructor | Creates main PCB document object |
| `0x01923680` | PCBDoc.ReadFileHeader | Opens file, validates version |
| `0x01923b10` | PCBDoc.ReadAllSections | Iterates sections, calls Read on each |
| `0x01923c50` | PCBDoc.FindSectionByName | Looks up section by stream name |
| `0x0190d030` | V4SectionBase.Read | Shared V4 binary read dispatcher |
| `0x018825f0` | TPrimitivesSection.Read | Shared Section format read |
| `0x0189fb10` | TPrimitivesSection.SetStorage | Shared Section format write init |
| `0x0186a210` | InitV6Names | V6/PcbDoc stream name table |
| `0x0186b060` | InitV3Names | V3 base stream name table |
| `0x0186a9f0` | InitAltNames | PcbLib alternative stream name table |
