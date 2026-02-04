# Altium AD26 Reverse Engineering Notes

## Problem
"I/O Error 32" when opening a SchLib written by altium-cli (../altium-cli) in Altium Designer AD26.

I/O Error 32 in Delphi = `EInOutError` with code 32 = Windows `ERROR_SHARING_VIOLATION`.

## Installation Layout

- Main EXE: `C:\Program Files\Altium\AD26\X2.EXE` (41MB, Delphi)
- System DLLs: `C:\Program Files\Altium\AD26\System\`
- 65 DLLs in root, 259 files total in root
- Key DLLs in System directory

## Altium Technology Stack

Altium Designer originated as Protel in 1985 (Turbo Pascal), became a Delphi app from Delphi 3 onwards, and grew to ~15M lines of code. The DXP platform (Protel DXP, 2003) is a modular plugin architecture where every module is an isolated DLL exposing interfaces. Extensions can be built in Delphi, C#, or C++.

In AD26, Altium has **progressively rewritten modules from Delphi to C#/.NET 8**. The entire schematic subsystem (`Altium.Sch.*`) is now .NET 8, while the PCB subsystem and old-school engines remain native Delphi. The .NET 8 DLLs use **ReadyToRun (R2R)** AOT compilation, which produces real x86-64 machine code — this is why Ghidra can decompile them and the output looks like native code (it IS native x86, just compiled from C# not Delphi).

### How to tell them apart

| Marker | Delphi | .NET 8 (R2R) | .NET Framework |
|--------|--------|--------------|----------------|
| `strings \| grep "Embarcadero Delphi"` | Yes | No | No |
| `strings \| grep ".NETCoreApp"` | No | Yes | No |
| `strings \| grep "_CorDllMain"` | No | No | Yes |
| Ghidra decompilation | Real Delphi vtables | R2R stubs with GC barriers | "CLR Managed Code" / halt_baddata |
| Best tool | Ghidra | ILSpy (preferred) or Ghidra (R2R) | ILSpy |

### Key Binaries for SchLib Analysis

| Binary | Size | Type | Role |
|--------|------|------|------|
| `X2.EXE` | 41MB | **Delphi** (native x64) | Main executable, contains `EInOutError` |
| `System/AdvSch.dll` | 42MB | **Delphi** (native x64) | Main schematic editor, ISchLib interface, SchAPI exports |
| `System/Altium.Sch.DataModel.dll` | 6MB | **.NET 8 (R2R)** | Full serialization format, file I/O — **use ILSpy** |
| `System/Altium.Sch.Core.dll` | 2.5MB | **.NET 8 (R2R)** | Schematic core logic — **use ILSpy** |
| `System/Altium.Sch.Base.dll` | 559KB | **.NET 8 (R2R)** | `ISchDocument`, `IsSchLibraryFile` — **use ILSpy** |
| `System/Altium.Sch.Layer2Base.dll` | 611KB | **.NET 8 (R2R)** | `GetISchLibrary`, `ImportDocument` — **use ILSpy** |
| `System/LibraryMigrator.Engine.dll` | 1.2MB | **Delphi** (native x64) | `CreateAndLoadPCBLibraryFromFile`, `CreateSchLibraryWrapped` |
| `System/EDPSDK.dll` | 24MB | **Delphi** (native x64) | `Rt_BinaryFileLoader`, `SDK_SchLibrary`, OLE/CFB support |
| `System/IntegratedLibrary.dll` | 24MB | **Delphi** (native x64) | Library path resolution, `FSchLib`, `FPCBLib` |
| `System/Altium.PCB.DataModel.dll` | 14MB | **Delphi** (native x64) | PCB data model (Delphi compiler v30.0) |
| `System/Altium.PCB.BinaryLoader.dll` | 54MB | **Delphi** (native x64) | PCB binary file loader (Delphi compiler v30.0) |

## Delphi Class Names Found in AdvSch.dll

### SchLib-Related Classes
- `TSCHCommonDocument` - Common document base
- `TSCHSheetDocument` - Sheet document type
- `TSCHSheetDocumentNativeImport` - Native import handler
- `TSCHLIBExplorerView` / `TSCHLIBExplorerView2` - Library explorer UI
- `TSCHLibHighlighter` - Library syntax highlighting
- `TSCHLibraryGraphicalView` - Library graphical view
- `TSCHLibraryPanelView` - Library panel
- `TSCHLoader` / `TSCHLoaders` - File loading system
- `TSCHProcessControl` / `TSCHProcessControl2` - Process control
- `TSCHPasteObject` - Paste operations
- `TSCHGraphicalView` - Graphical view
- `TSCHHighlighter` / `TSCHHighlighterWrapper` - Highlighting
- `TSCHFeaturesAdapter` - Features adapter
- `TSCHExpressionBuilder` - Expression builder

### Error Classes in AdvSch.dll
- `EInOutError` (Delphi I/O error - this is the one that produces "I/O Error 32")
- `EListError`
- `ERangeError`
- `EMathError`
- `EConvertError`
- `EVariantError`
- `EAbstractError`
- `EIntfCastError`
- `EOSError`
- `EEncodingError`
- `EPNGInvalidFileHeader`

### Error Classes in X2.EXE
- `EInOutError` (also present here)
- `AllowRepositorySharing` string found

## SchAPI Functions (AdvSch.dll Exports)

### Library Component Functions
- `SchAPI_LoadComponentFromLibrary` - **KEY: loads component from library**
- `SchAPI_GetFirstLibraryComponent`
- `SchAPI_GetNextLibraryComponent`
- `SchAPI_GetCurrentLibraryComponent`
- `SchAPI_GetCurrentLibraryHandle`
- `SchAPI_GetLibraryComponentHandle`
- `SchAPI_GetLibraryPartContainer`
- `SchAPI_GetLibraryPartCount`
- `SchAPI_GetLibraryComponentForPart`
- `SchAPI_IsCurrentLibraryComponent`
- `SchAPI_DestroyLibraryComponentObject`

### Library Alias/Group Functions
- `SchAPI_AddLibraryComponentAliasName`
- `SchAPI_ClearLibraryComponentAliasNames`
- `SchAPI_GetLibraryComponentAliasCount`
- `SchAPI_GetLibraryComponentAliasNameAt`
- `SchAPI_AddLibraryComponentGroupName`
- `SchAPI_GetLibraryComponentGroupNameAt`
- `SchAPI_GetLibraryComponentGroupNameCount`
- `SchAPI_PartAvailableInLibrary`

### Document Functions
- `SchAPI_GetCurrentDocumentHandle`
- `SchAPI_GetCurrentDocumentName`
- `SchAPI_GetDocumentCountInProject`
- `SchAPI_GetDocumentHandleByIndex`
- `SchAPI_GetDocumentHandleFromFileName`
- `SchAPI_GetOpenedDocumentDetails`
- `SchAPI_GetOpenedDocumentsCount`
- `SchAPI_QueryDocumentName`
- `SchAPI_QueryDocumentOptions`

### Object/Iterator Functions
- `SchAPI_CreateIterator`
- `SchAPI_CreateGroupIterator`
- `SchAPI_CreateGroupIteratorIncludeAll`
- `SchAPI_CreateSimpleGroupIterator`
- `SchAPI_CreateSimpleIterator`
- `SchAPI_CreateSpatialIterator`
- `SchAPI_DestroyGroupIterator`
- `SchAPI_DestroyIterator`
- `SchAPI_DestroySpatialIterator`
- `SchAPI_CreateObject`
- `SchAPI_CreateObjectEx`
- `SchAPI_DestroyObject`
- `SchAPI_DestroyPartItem`
- `SchAPI_AddObjectToContainer`
- `SchAPI_CreatePainter`
- `SchAPI_DrawComponent`
- `SchAPI_DrawComponentByHandle`

### Import/Location Functions
- `SchAPI_ImportFromUser`
- `SchAPI_ImportFromUser_SystemOptions`
- `SchAPI_ChooseLocation`
- `SchAPI_ChooseRectangleByCorners`
- `SchAPI_GetBoundingRect`
- `SchAPI_GetBoundingRectangleForOrcad`
- `SchAPI_DefaultGroundPowerObjectName`

### Storage-Related Strings
- `StorageHandlerFlag`
- `StorageHandlerName`
- `FStorage`
- `AStorage`
- `fdoAllNonStorageItems`
- `StorageSupport`
- `TNotifierStorage`
- `FileHeader` (top-level CFB stream name)
- `TFileHeader.`

### File Format Identifiers
- `eProtelDOSBinarySchematic`
- `eProtelAsciiSchematicFile_v40`
- `eProtelAsciiSchematicFile_v50`
- `eProtelBinarySchematicFile_v40`
- `eProtelBinarySchematicFile_v50`
- `eProtelAsciiSchematicLibraryFile`
- `eProtelBinarySchematicLibraryFile_v40`
- `eProtelBinarySchematicLibraryFile_v50`
- `eCircuitStudioBinarySchematicFile_v50`
- `eSchLib` (enumeration value for SchLib type)

### Generic Interface Types
- `ISchLib` - Main SchLib interface
- `IEnumerable<SCHInterfaces.ISchLib>`
- `TList<SCHInterfaces.ISchLib>`
- `TArray<SCHInterfaces.ISchLib>`

## .NET Assemblies (System/ directory)

### Altium.Sch.* DLLs
| DLL | Description |
|-----|-------------|
| `Altium.Sch.Annotation.dll` | Annotation handling |
| `Altium.Sch.Base.dll` | Base types |
| `Altium.Sch.Compilation.dll` | Compilation |
| `Altium.Sch.Core.dll` | Core logic |
| `Altium.Sch.Data.Project.dll` | Project data |
| `Altium.Sch.DataModel.dll` | Data model |
| `Altium.Sch.Editor.dll` | Editor |
| `Altium.Sch.HTMLReports.dll` | HTML reports |
| `Altium.Sch.Interfaces.dll` | Interface definitions |
| `Altium.Sch.Layer2Base.dll` | Layer base |
| `Altium.Sch.Painter.dll` | Rendering |
| `Altium.Sch.SchematicDialogs.Plugin.dll` | Dialog plugins |
| `Altium.Sch.SchematicDialogs.UI.dll` | Dialog UI |
| `Altium.Sch.SignalCreator.dll` | Signal creation |
| `Altium.Sch.Validation.dll` | Validation |

## CFB File Structure Comparison

### Original (Synthiam.SchLib) vs New (Synthiam-new.SchLib)

Both files are CFB v3.62 with 512-byte sectors and 64-byte mini sectors.

| Property | Original | New |
|----------|----------|-----|
| File size | 566,784 | 558,080 |
| CFB Version | 3.62 | 3.62 |
| Sector size | 512 | 512 |
| Mini sector size | 64 | 64 |
| FAT sectors | 9 | 9 |
| Mini FAT sectors | 39 | 38 |
| Total CFB entries | 348 | 349 |
| FileHeader size | 9,493 bytes | 9,493 bytes |
| Storage stream size | 25 bytes | 25 bytes |
| Has SectionKeys | No | Yes (112 bytes) |

### Key Differences
1. **SectionKeys stream**: New file has it, original doesn't (for `ATtiny45/85` -> `ATtiny45_85` mapping)
2. **Parameter ordering**: Same parameters, different order in FileHeader
3. **Component Data sizes**: Slightly different per component (record encoding differences)
4. **New file has 349 entries** vs 348 (extra SectionKeys stream)

### FileHeader Parameters (both files identical)
- 466 total parameters
- HEADER = "Protel for Windows - Schematic Library Editor Binary File Version 5.0"
- WEIGHT = 5377
- COMPCOUNT = 172
- FONTIDCOUNT = 4
- MINORVERSION = 1
- Plus LIBREF0..171, PARTCOUNT0..171, COMPDESCR*, ALIASCOUNT*, COMP*ALIAS* entries

## altium-cli SchLib Writer (altium-format crate)

### Writer Architecture
- Uses `cfb` Rust crate for OLE Compound Binary Format
- Creates CFB v3 files (`CompoundFile::create_with_version(cfb::Version::V3, ...)`)
- Writes streams: `/Storage`, `/FileHeader`, `/SectionKeys` (if needed), `/ComponentName/Data`

### Write Order
1. `/Storage` - Icon storage header: `|HEADER=Icon storage\0`
2. `/FileHeader` - Parameters block with all component metadata
3. `/SectionKeys` - Only for components needing name escaping (/ or >31 chars)
4. Component storages - `/ComponentName/Data` with records
5. Alias redirections - `/AliasName/Redirection` streams

### Record Format
- Parameter records: 4-byte length + pipe-delimited parameters + null terminator
- Binary pin records: 4-byte length with flag bit 0x01 set, followed by binary fields

## .NET Decompilation Findings (ILSpy)

Decompiled `Altium.Sch.DataModel.dll` (6MB) with ILSpy - contains the full serialization format.

### Key Decompiled Files

| File | Contents |
|------|----------|
| `SchDataSerializerParam.cs` | OLE/CFB compound file serializer (uses OpenMcdf library) |
| `SchDataSerializerBinary.cs` | Raw binary file serializer |
| `SchDataSerializer.cs` | Base serializer with all Import/Export methods |
| `FileFormatV5.cs` | V5 format export/import for all record types |
| `SchDataImporterLibraryV5.cs` | SchLib file loading sequence |
| `SchDataExporterLibraryV5.cs` | SchLib file saving sequence |
| `SchDataUtils.cs` | Coordinate conversion utilities |

### Coordinate System (CRITICAL)

Altium uses 100,000 units per mil (DXP2004 SP2 format). Binary format divides by 100,000.

```csharp
// SchDataUtils.cs
public static void GetWholeAndFractionalPart_DXP2004SP2_To_DXP2004SP1(int coord, out int whole, out int fraction)
{
    whole = coord / 100000;        // Integer mil part
    fraction = coord - 100000 * whole;  // Sub-mil fraction (0-99999)
}

public static int GetCoord_DXP2004SP1_To_DXP2004SP2(int whole, int fraction)
{
    return whole * 100000 + fraction;  // Back to full precision
}
```

**altium-cli uses 10,000 units per mil** - this is a DIFFERENT coordinate system.

### Binary Pin Record Format (mode=1, flag 0x01000000)

When `mode=1` (binary), the parametric serializer reads:
- Coordinates as `short` (2 bytes), multiplied by 100,000 on import
- Strings as length-prefixed (Pascal strings)
- Colors as 4-byte int

Pin fields in order (from FileFormatV5.ExportPin):
1. `OwnerIndex` (int32) - via Export_LongInt
2. `OwnerPartId` (int16) - via Export_ShortInt
3. `OwnerPartDisplayMode` (byte)
4. `SymBol_InnerEdge` (byte)
5. `SymBol_OuterEdge` (byte)
6. `SymBol_Inner` (byte)
7. `SymBol_Outer` (byte)
8. `Description` (dynamic string)
9. `FormalType` (byte)
10. `Electrical` (pin electrical type)
11. `PinConglomerate` (byte) - orientation | hidden | showName | showDesignator | accessible | locked | additionalList
12. `PinLength` (coord - short in binary)
13. `Location.X` (coord - short in binary)
14. `Location.Y` (coord - short in binary)
15. `Color` (uint32)
16. `Name` (dynamic string)
17. `Designator` (dynamic string)
18. `SwapIdPin` (string)
19. `SwapIDPart` (dynamic string)
20. `DefaultValue` (dynamic string)

**IMPORTANT**: altium-cli writes pin field #1 as record_type=2 (int32), but Altium expects `OwnerIndex` (int32).

### Parametric Record Format (mode=0, no flag bit)

4-byte length (low 24 bits = size, high 8 bits = mode 0x00)
Followed by pipe-delimited params: `|KEY=VALUE|KEY=VALUE\0`

### Binary Record Format (mode=1, flag bit 0x01000000)

4-byte length (low 24 bits = size, high 8 bits = mode 0x01)
Followed by sequential binary fields (no names, position-dependent)

### SchLib Loading Sequence (SchDataImporterLibraryV5.Run)

1. `ImportBaseWarehouse()`:
   a. `ImportLibrary()` - reads `/FileHeader` stream
   b. `componentSectionKeyList.Load()` - reads `/SectionKeys` (optional)
   c. `FindFirstStream("Data")` - iterates all `/<component>/Data` streams
   d. For each Data stream: read RECORD byte, create object, import fields
   e. Pins use BINARY mode (mode=1), other records use RECORD mode (mode=0)
2. `ImportExtendedWarehouse()`:
   a. Reads `/Storage` stream
   b. Reads extended pin data: PinFrac, PinDesc, PinMiscData, PinTextData, PinWideText, etc.
3. `ImportAdditionalWarehouse()`:
   a. Reads `/LibAdditional` stream (if present)

### ImportLibrary (FileHeader Parsing)

```csharp
Serializer.StartStream("", "FileHeader");
Import_Instruction(ref argN, "RECORD");     // reads the first record byte
Import_String(ref argN2, "HEADER");         // reads HEADER parameter
Import_LongInt(ref weight, "Weight");       // reads WEIGHT parameter
Import_LongInt(ref argN3, "MinorVersion");  // reads MinorVersion parameter
Import_String(ref argN4, "UniqueID");       // reads UniqueID (optional)
FileFormat.ImportFromFile(Serializer, library);  // reads remaining library params
Serializer.EndStream();
```

### Component Name Escaping (FixName)

```csharp
private string FixName(string name)
{
    // Replace invalid chars: / \ : * ? " < > | !
    // Truncate to 30 chars (NOT 31!)
    if (name.Length <= 31) return name;
    return name.Substring(0, 30);
}
```

**NOTE**: altium-cli truncates to 31 chars, but Altium truncates to 30!

### Library Exporter Save Sequence (SchDataExporterLibraryV5)

1. Write FileHeader (library metadata + component index)
2. Write SectionKeys (for components needing name escaping)
3. Write BaseWarehouse (component Data streams):
   - Component record: `RECORD` mode (byte 0x01 = component type)
   - Pin records: `BINARY` mode (byte 0x02 = pin type)
   - Other records: `RECORD` mode
   - Terminate with `RECORD 0x00`
4. Write pin extended data (PinFrac, PinDesc, PinMiscData, etc.)
5. Write Storage stream
6. Write LibAdditional (if applicable)

### Pin Extended Data Streams

For each component that has pins:
- `/<component>/PinFrac` - Fractional coordinate parts (int32 x3: locX_frac, locY_frac, length_frac)
- `/<component>/PinDesc` - Long descriptions (>254 chars)
- `/<component>/PinMiscData` - SwapIdPair
- `/<component>/PinTextData` - Custom pin text display settings
- `/<component>/PinWideText` - Unicode pin text (name, designator, etc.)
- `/<component>/PinSymbolLineWidth` - Symbol line widths
- `/<component>/PinPackageLength` - Package lengths
- `/<component>/PinPropagationDelay` - Propagation delays
- `/<component>/PinFunctionData` - Pin function definitions

### OpenMcdf Usage

Altium's .NET code uses **OpenMcdf** library for CFB access:
```csharp
compound = writing
    ? new CompoundFile(CFSVersion.Ver_3, CFSConfiguration.EraseFreeSectors)
    : new CompoundFile(file, CFSUpdateMode.ReadOnly, CFSConfiguration.Default);
```

### File Open Modes
- Read: `FileMode.Open, FileAccess.Read, FileShare.Read`
- Write: `FileMode.Create, FileAccess.Write`
- Open mode 0x0000 = Read, 0xFF00 = Write

## SchLib Alias Implementation (Reverse Engineered)

Aliases allow a single SchLib component to be referenced by multiple names.

### Data Model (from ILSpy decompilation of Altium.Sch.DataModel.dll)

**`SchDataAliasList`** - A `SortedList` (sorted, no duplicates) storing alias names as keys.

```csharp
public class SchDataAliasList : ISchDataAliasList {
    private SortedList list;  // Keys = alias names (sorted), Values = null
    public void Add(string argValue);      // Add alias (no duplicates)
    public void Remove(string argValue);   // Remove by name
    public void Delete(int argIndex);      // Remove by index
    public void Clear();                   // Remove all
    public int GetCount();                 // Number of aliases
    public string GetValue(int argIndex);  // Get alias at index
    public string GetCommaText();          // "alias1,alias2,alias3"
    public void SetCommaText(string);      // Parse comma-separated
}
```

Each `ISchDataComponent` has a private `aliasList` field and `GetAliasList()` accessor.

### File Format: Export (SchDataExporterLibraryV5)

#### FileHeader Stream (component index)
The `/FileHeader` stream stores a component index with alias info:
```
|ALIASCOUNT{i}={count+1}|LIBREF{i}={primary_name}|ALIAS{i+1}={alias1}|...
```
- `AliasCount{i}` = number of aliases + 1 (includes primary LibReference)
- First "alias" is the primary `LibReference`
- Remaining are actual aliases: `Alias1`, `Alias2`, etc.

#### Component Header (per-component Data stream)
Each `/<component>/Data` stream starts with:
```
|ALIASCOUNT={count+1}|LIBREFERENCE={primary}|ALIAS1={alias1}|ALIAS2={alias2}|...
```
Same format: AliasCount includes the primary reference.

#### Redirection Streams
For each alias, a redirection stream is created:
```
/<alias_name>/Redirection → |RECORD=0|SECTIONNAME={primary_component_name}
```
This allows looking up an alias name and finding which primary component it maps to.

#### SectionKeys
Both the primary name and all alias names are added to the `ComponentSectionKeyList` with code 31.

### File Format: Import (SchDataImporterLibraryV5)

#### ImportComponentHeaderV10
```csharp
Import_ShortInt(ref argN, "AliasCount");
for (int i = 1; i <= argN; i++) {
    Import_String(ref argN2, "Alias" + i);
    if (component.GetLibReference() == "*")
        component.SetLibReference(argN2);  // First = primary name
    else
        component.GetAliasList().Add(argN2);  // Rest = aliases
}
```

#### GetLibraryReferenceByAliasName (alias lookup)
Resolution order:
1. Check if `/<section>/Redirection` stream exists → read `SectionName` parameter (= primary name)
2. Check if `/<section>/Data` stream exists → return aliasName directly (it IS the primary)
3. Fall back to scanning `/FileHeader`: iterate all components, check `LibRef{i}` and `Comp{i}Alias{j}` for match

### Native SchAPI Layer (AdvSch.dll)

Thin wrappers around vtable calls on component objects:
| Function | Vtable Offset | Signature |
|----------|--------------|-----------|
| `SchAPI_GetLibraryComponentAliasCount` | +0x838 | `(component*, out count) → HRESULT` |
| `SchAPI_GetLibraryComponentAliasNameAt` | +0x838,+0x840 | `(component*, index, buf, bufsize) → HRESULT` |
| `SchAPI_AddLibraryComponentAliasName` | +0x8e8 | `(component*, name) → HRESULT` |
| `SchAPI_RemoveLibraryComponentAliasName` | +0x8f0 | `(component*, name) → HRESULT` |
| `SchAPI_ClearLibraryComponentAliasNames` | +0x900 | `(component*) → HRESULT` |

The actual implementation is in the .NET layer (Altium.Sch.DataModel.dll), accessed via COM interop from native Delphi.

### Summary for altium-cli Implementation

To correctly implement aliases in a SchLib writer:
1. **FileHeader**: Write `ALIASCOUNT{i}={N+1}` where N = number of aliases. First entry is `LIBREF{i}`, followed by `ALIAS1`..`ALIASN` for aliases. Also write `COMP{i}ALIAS{j}` entries.
2. **Component Data header**: Write `ALIASCOUNT={N+1}`, `LIBREFERENCE={primary}`, `ALIAS1={alias1}`, ...
3. **Redirection streams**: For each alias, create `/<alias_escaped>/Redirection` with `|RECORD=0|SECTIONNAME={primary_escaped}`.
4. **SectionKeys**: Add both primary name and all alias names to the SectionKeys map (code 31).
5. **AliasCount always includes the primary reference** (count = aliases + 1).

## Hypotheses for I/O Error 32

### Most Likely: Binary Pin Format Mismatch
altium-cli writes pin records with a **different field order** than Altium expects:
- altium-cli writes: `record_type(4) | unknown(1) | ownerPartId(2) | displayMode(1) | symbols(4) | desc | unknown(1) | electrical(1) | conglomerate(1) | length(2) | locX(2) | locY(2) | color(4) | name | designator | swapIdGroup | swapIdPart | defaultValue`
- Altium expects: `ownerIndex(4) | ownerPartId(2) | displayMode(1) | symbols(4) | desc | formalType(1) | electrical(1) | conglomerate(1) | length(2) | locX(2) | locY(2) | color(4) | name | designator | swapIdPin | swapIdPart | defaultValue`

Key differences:
1. **Field 1**: altium-cli writes `record_type=2` (4 bytes), Altium expects `OwnerIndex` (4 bytes)
2. **Field between symbols and electrical**: altium-cli writes `unknown byte 0`, Altium expects `FormalType` (byte)
3. **Swap ID fields**: altium-cli writes `SwapIdGroup`, Altium expects `SwapIdPin`

### Possible: SectionKeys Stream
Original file lacks SectionKeys; new file has it. Could cause parsing confusion.

### Possible: CFB Library Difference
altium-cli uses Rust `cfb` crate; Altium uses `OpenMcdf` (.NET). May produce slightly different CFB structures.

### Less Likely: File Locking
True sharing violation unlikely since it's a new file.

## Ghidra Analysis Plan

1. Analyze `AdvSch.dll` (42MB Delphi) - primary target for SchLib loading code (Delphi side)
2. Focus on `TSCHLoader` and `SchAPI_LoadComponentFromLibrary`
3. Trace the Delphi-side file opening that calls into .NET
4. Look for CFB/OLE structured storage opening code
5. Trace `EInOutError` exception sources

## Ghidra Project: ghidra-altium

Project location: `/c/Users/dev/git/ghidra-altium.gpr`
Project data: `/c/Users/dev/git/ghidra-altium.rep/` (~9.5G)

### Quick Start for Future Agents

**37 native Delphi DLLs are fully imported and analyzed.** You do NOT need to re-import or re-analyze.

To query any binary using ghidra-cli:
```bash
# Start daemon (auto-starts on first command)
ghidra daemon start --project ghidra-altium

# Switch to a specific program
ghidra program open AdvSch.dll --project ghidra-altium

# Query functions, decompile, search strings, etc.
ghidra function list --project ghidra-altium --limit 20
ghidra decompile <function_name_or_address> --project ghidra-altium
ghidra find string "SchLib" --project ghidra-altium
ghidra stats --project ghidra-altium
ghidra program list --project ghidra-altium

# Switch between programs without JVM restart (<200ms)
ghidra program open LibraryMigrator.Engine.dll --project ghidra-altium
```

**Key programs to start with for SchLib RE:**
1. `AdvSch.dll` — Main schematic engine, has all SchAPI exports and TSCHLoader
2. `LibraryMigrator.Engine.dll` — Small, focused library creation/loading engine
3. `EDPSDK.dll` — `Rt_BinaryFileLoader`, SDK wrappers for SchLib/PcbLib
4. `Altium.Sch.Base.dll` — Core interfaces (`ISchDocument`, `IsSchLibraryFile`)
5. `Altium.Sch.Layer2Base.dll` — `GetISchLibrary`, `ImportDocument`, `ExportDocument`

**For .NET DLLs** (file format serialization code), use ILSpy — see decompilation findings below.

### All Analyzed Binaries

Binaries below are in the `ghidra-altium` Ghidra project. **Native Delphi** binaries decompile properly in Ghidra. **.NET 8 (R2R)** binaries have AOT-compiled x86 that Ghidra can partially decompile, but **ILSpy gives much better results** for these. **.NET Framework** binaries show "CLR Managed Code" in Ghidra — use ILSpy only.

#### Core Engines
| Binary | Size | Analysis Time | Role |
|--------|------|---------------|------|
| `X2.EXE` | ~30M | 621s | **Main Altium Designer executable**, contains `EInOutError`, Delphi RTL |
| `AdvSch.dll` | 41M | 637s | **Main schematic engine** — SchAPI exports, TSCHLoader, file I/O |
| `Advpcb.dll` | 112M | 1992s | **Main PCB engine** — largest binary, core PCB editor |

#### PCB DLLs
| Binary | Size | Analysis Time | Role |
|--------|------|---------------|------|
| `Altium.PCB.BinaryLoader.dll` | ~54M | 518s | **PCB binary file loader** — file format parsing |
| `Altium.PCB.DataModel.dll` | 14M | 212s | PCB data model (native, not .NET despite name) |
| `AdvPcbTools.dll` | 21M | 328s | PCB tools (DRC, routing helpers) |

#### Schematic DLLs (ALL .NET 8 — use ILSpy, not Ghidra)
| Binary | Size | Type | Role |
|--------|------|------|------|
| `Altium.Sch.Base.dll` | 547K | .NET 8 (R2R) | Core schematic interfaces: `ISchDocument`, `IsSchLibraryFile` |
| `Altium.Sch.Core.dll` | 2.5M | .NET 8 (R2R) | Schematic core logic |
| `Altium.Sch.DataModel.dll` | 5.8M | .NET 8 (R2R) | **Schematic serialization/file format** — key for RE |
| `Altium.Sch.Layer2Base.dll` | 611K | .NET 8 (R2R) | `GetISchLibrary`, `ImportDocument`, `ExportDocument` |
| `Altium.Sch.Compilation.dll` | 2.3M | .NET 8 (R2R) | `GetSchLibHashedObjects`, library comparison |
| `Altium.Sch.Annotation.dll` | ~1M | .NET 8 (R2R) | Annotation handling |
| `Altium.Sch.Editor.dll` | ~2M | .NET 8 (R2R) | Editor logic |
| `Altium.Sch.Interfaces.dll` | ~500K | .NET 8 (IL only) | Interface definitions (pure IL, no R2R) |
| `Altium.Sch.Painter.dll` | ~2M | .NET 8 (R2R) | Rendering |

#### SDK & Framework DLLs
| Binary | Size | Type | Role |
|--------|------|------|------|
| `EDPSDK.dll` | 24M | **Delphi** | `Rt_BinaryFileLoader`, `SDK_PcbLibrary`, `SDK_SchLibrary`, OLE support |
| `DXPSDK.dll` | 13M | **Delphi** | DXP SDK framework |
| `Altium.SDK.dll` | 491K | .NET 8 (R2R) | SDK core |
| `Altium.SDK.Interfaces.dll` | 4.2M | .NET 8 (R2R) | SDK interface definitions |
| `Altium.Sdk.Ids.Contracts.dll` | 25K | .NET Framework | SDK ID contracts |
| `Altium.Sdk.Ids.dll` | 49K | .NET Framework | SDK IDs |
| `Altium.Dxp.Interfaces.dll` | 1.2M | .NET 8 (R2R) | `cDocKind_PcbLib`, `cDocKind_Schlib`, document type constants |

#### Library Management DLLs
| Binary | Size | Type | Role |
|--------|------|------|------|
| `LibraryMigrator.Engine.dll` | 1.2M | **Delphi** | `CreateAndLoadPCBLibraryFromFile`, `CreateSchLibraryWrapped`, `DestroyPCBLibrary` |
| `Altium.WorkspaceManager.Comparators.dll` | 490K | .NET 8 (R2R) | `ReadPCBLib`, `ReadSchLib`, `ReadSchLibHashedObjectsToList` |
| `IntegratedLibrary.dll` | 24M | **Delphi** | `FSchLib`, `FPCBLib`, library path resolution |

### Not Yet Imported (candidates for future analysis)
| Binary | Size | Role | Priority |
|--------|------|------|----------|
| `EditScript.dll` | 39M | `eIdReaderPCBLIB`, `eIdReaderSCHLIB`, `Rt_BinaryFileLoader`, `IMBA_DataModelFactory` | High |
| `ScriptingSystem.dll` | 36M | Same reader IDs as EditScript, scripting API surface | Medium |
| `UnifiedComponent.dll` | 28M | `PcbLibValidationImplements`, `SchLibValidationImplements` | Medium |
| `EDesignData.dll` | 30M | `CreatePcbLib`, `CreateSchDoc`, `CreateSchLib` | Medium |
| `Orcad7ld.dll` | 21M | OrCAD importer with `TSCHBaseLibraryComponentImporter` | Low |
| `PCAD16ld.dll` | 20M | P-CAD importer with `CreateSCHLibrary` | Low |

### Also in Project (from earlier imports, not high priority)
- `Altium.PCB.CollaborateMerge.Module.dll`
- `Altium.PCB.DataModel.X.dll`
- `Altium.Sch.Data.Project.dll`
- `Altium.Sch.HTMLReports.dll`
- `Altium.Sch.SchematicDialogs.Plugin.dll`
- `Altium.Sch.SchematicDialogs.UI.dll`
- `Altium.Sch.SignalCreator.dll`
- `Altium.Sch.Validation.dll`
- `Altium.Data.Dimensional.dll`
- `Altium.Dxp.Classes.dll`

### .NET DLLs — Use ILSpy (ilspycmd)

`ilspycmd` is installed globally: `dotnet tool list -g` shows `ilspycmd 9.1.0.7988`.

```bash
# Decompile entire assembly to stdout
ilspycmd "/c/Program Files/Altium/AD26/System/Altium.Sch.DataModel.dll"

# Search for specific code
ilspycmd "/c/Program Files/Altium/AD26/System/Altium.Sch.DataModel.dll" | grep -n "AliasCount"
```

| DLL | Role |
|-----|------|
| `Altium.Sch.DataModel.dll` | Full serialization format (SchDataSerializerParam.cs, FileFormatV5.cs) — **key for SchLib RE** |
| `Altium.Sch.Core.dll` | Schematic core, `GetDelphiTypeName` interop |
| `Altium.Sch.Base.dll` | Base types, `ISchDocument`, file extension constants |
| `OpenMCDF.dll` (202KB) | **CFB/OLE Compound File** implementation used by Altium. Source: github.com/ironfede/openmcdf |
| `DXPServerSDK.Contracts.dll` / `DXPServerSDK.dll` | Server SDK, minimal value |

### Identifying DLL Type

**WARNING**: The old method of checking for `_CorDllMain` only detects .NET Framework DLLs. .NET 8 (R2R) DLLs do NOT have `_CorDllMain` and look like native PE files to simple tools. Use these markers instead:

```bash
# .NET 8 / .NET Core
strings "$dll" | grep ".NETCoreApp"

# .NET Framework (older)
strings "$dll" | grep "_CorDllMain"

# Native Delphi
strings "$dll" | grep "Embarcadero Delphi"
```

### DLL Selection Method

605 DLLs scanned in `/c/Program Files/Altium/AD26/System/` using `strings` searching for:
- SchLib, PcbLib, SchDoc, PcbDoc, SchAPI, PcbAPI
- ISchLib, IPcbLib, CFB, CompoundFile, OLE, Storage
- EInOutError, TSCHLoader, FileHeader, Rt_BinaryFileLoader
- Binary.*Loader, DataModel, eIdReader

### bridge.py Analysis Fix

`handle_analyze()` was fixed to poll `auto_mgr.isAnalyzing()` in a loop and save the program after analysis completes. The `AutoAnalysisManager.startAnalysis(monitor)` API is non-blocking and returns immediately.

## ILSpy Decompilation Results (2025-02-01)

### Decompiled DLLs

All output saved to `/c/Users/dev/git/ghidra-cli/ilspy-cli/decompiled/`.

| DLL | Files | Content |
|-----|-------|---------|
| `Altium.Sch.DataModel` | 77 | **Complete SchLib/SchDoc serialization** — record formats, binary/parametric encoding, import/export |
| `Altium.Sch.Base` | 210 | Base types, TObjectId enum, interfaces |
| `Altium.Sch.Core` | 507 | Core schematic logic |
| `Altium.Sch.Layer2Base` | 99 | GetISchLibrary, ImportDocument, ExportDocument |
| `Altium.Sch.Compilation` | 328 | Library comparison, hashing |
| `Altium.Sch.Editor` | 17 | Editor logic (partial) |
| `Altium.Sch.Interfaces` | 17 | Interface definitions (IL-only, limited output) |
| `Altium.SDK.Interfaces` | 3795 | **All COM interfaces** — IPCB_Pad, IPCB_Track, IPCB_Via, IPCB_Library, ISch_Lib |
| `Altium.Dxp.Interfaces` | 1597 | Document type constants, DXP framework |
| `Altium.WorkspaceManager.Comparators` | 115 | ReadPCBLib, ReadSchLib readers |
| `InteractiveProperties.Providers.PCB.DataModel` | 282 | PCB property definitions for all object types |
| `LibraryMigrator.Engine` | 165 | Library creation/loading |
| `Altium.PCB.CollaborateMerge.Module` | 49 | PCB collaboration/merge |
| `Altium.Edp.Interfaces` | 17 | EDP interfaces |

### SchLib Binary Record Codes (BinaryFileCode.cs)

Complete mapping from `Altium.Sch.DataModel/FileFormats/BinaryFileCode.cs`:

| Code | Name | Description |
|------|------|-------------|
| 1 | CPart | Component/Symbol |
| 2 | CPin | Pin (uses BINARY mode) |
| 3 | CSymbol | Symbol |
| 4 | CLabel | Label |
| 5 | CBezier | Bezier curve |
| 6 | CPolyline | Polyline |
| 7 | CPolygon | Polygon |
| 8 | CEllipse | Ellipse |
| 9 | CPie | Pie |
| 10 | CRoundRectangle | Round rectangle |
| 11 | CEllipticalArc | Elliptical arc |
| 12 | CArc | Arc |
| 13 | CLine | Line |
| 14 | CRectangle | Rectangle |
| 15 | CSheetSymbol | Sheet symbol |
| 16 | CSheetEntry | Sheet entry |
| 17 | CPowerObject | Power object |
| 18 | CPort | Port |
| 19 | CSimProbe | Simulation probe |
| 20 | CSimVector | Simulation vector |
| 21 | CSimStimulus | Simulation stimulus |
| 22 | CNoERC | No ERC marker |
| 23 | CErrorMarker | Error marker |
| 24 | CLayoutDirective | Layout directive |
| 25 | CNetLabel | Net label |
| 26 | CBus | Bus |
| 27 | CWire | Wire |
| 28 | CTextFrame | Text frame |
| 29 | CJunction | Junction |
| 30 | CImage | Image |
| 31 | CSheet | Sheet |
| 32 | CSheetName | Sheet name |
| 33 | CSheetFileName | Sheet file name |
| 34 | CDesignator | Designator |
| 35 | CPartType | Part type |
| 36 | CPartDescription | Part description |
| 37 | CBusEntry | Bus entry |
| 38 | CSheetPartFileName | Sheet part file name |
| 39 | CTemplate | Template |
| 40 | CTaskHolder | Task holder |
| 41 | CParameter | Parameter |
| 42 | CSchComponent | Schematic component |
| 43 | CParameterSet | Parameter set |
| 44 | CImplementationsList | Implementations list |
| 45 | CImplementation | Implementation |
| 46 | CImplementationMap | Implementation map |
| 47 | CMapDefiner | Map definer |
| 48 | CParameterList | Parameter list |
| 200 | CLibrary | Library container |
| 208 | CEmbeddedStream | Embedded stream |
| 209 | CNote | Note |
| 210 | CProbe | Probe |
| 254 | CExtraObjectIndex | Extended index (followed by RECORDEX int32) |
| 255 | CEndInstruction | Record terminator |

### SchLib Serialization Modes

**Mode 0 (Parametric/ASCII)**: Used for all records except pins.
- 4-byte header: `length | (mode << 24)` where mode=0x00
- Body: pipe-delimited `|KEY=VALUE|KEY=VALUE\0`

**Mode 1 (Binary)**: Used for pin records only.
- 4-byte header: `length | 0x01000000`
- Body: sequential binary fields (position-dependent, no names)
- Coordinates stored as `int16` (divided by 100,000 from internal)
- Strings stored as Pascal strings (1-byte length + data, max 254 chars)

### SchLib Pin Record Field Order (Binary Mode)

From `FileFormatV5.ExportPin()` — exact binary field order:

1. `OwnerIndex` — int32 (Export_LongInt)
2. `OwnerPartId` — int16 (Export_ShortInt)
3. `OwnerPartDisplayMode` — byte
4. `SymBol_InnerEdge` — byte
5. `SymBol_OuterEdge` — byte
6. `SymBol_Inner` — byte
7. `SymBol_Outer` — byte
8. `Description` — dynamic string (Pascal, max 254 chars)
9. `FormalType` — byte
10. `Electrical` — pin electrical type (byte)
11. `PinConglomerate` — byte (orientation bits 0-1 | hidden bit 2 | showName bit 3 | showDesignator bit 4 | notAccessible bit 5 | locked bit 6 | additionalList bit 7)
12. `PinLength` — coord (int16 in binary = mils)
13. `Location.X` — coord (int16 in binary = mils)
14. `Location.Y` — coord (int16 in binary = mils)
15. `Color` — uint32
16. `Name` — dynamic string
17. `Designator` — dynamic string
18. `SwapIdPin` — string (fixed Pascal)
19. `SwapIDPart` — dynamic string
20. `DefaultValue` — dynamic string

Additional ASCII-only fields (NOT in binary mode):
- `SwapIdPair` — string
- `PinName_PositionConglomerate` — byte
- `Name_CustomPosition_Margin` — coord (conditional)
- `Name_CustomFontID` — fontID (conditional)
- `Name_CustomColor` — color (conditional)
- `PinDesignator_PositionConglomerate` — byte
- `Designator_CustomPosition_Margin` — coord (conditional)
- `Designator_CustomFontID` — fontID (conditional)
- `Designator_CustomColor` — color (conditional)
- `SymBol_LineWidth` — byte
- `PinPackageLength` — coord
- `PinPropagationDelay` — double
- `HidePinNameAsFunction` — boolean
- `PinSelectedFunctionsCount`/`PinSelectedFunction{N}` — function list
- `PinDefinedFunctionsCount`/`PinDefinedFunction{N}` — function list
- `PinSymbolicName` — string
- `ShowPinSymbolicNameAsFunction` — boolean

### SchLib Component Record Field Order (Parametric Mode, Code 42)

From `FileFormatV5.ExportComponent()`:

1. `LibReference` — dynamic string (default "*")
2. `ComponentDescription` — string
3. `PartCount` — int16 (default 1)
4. `DisplayModeCount` — byte (default 1)
5. *[GraphicalObject fields via ExportGraphicalObject]*
6. `Location.X` — coord
7. `Location.Y` — coord
8. `DisplayMode` — byte
9. `IsMirrored` — boolean
10. `Orientation` — rotation (0/1/2/3 = 0°/90°/180°/270°)
11. `CurrentPartId` — int16
12. `ShowHiddenFields` — boolean
13. `ShowHiddenPins` — boolean
14. `LibraryPath` — dynamic string
15. `SourceLibraryName` — dynamic string
16. `DatabaseTableName` — dynamic string
17. `SheetPartFileName` — dynamic string
18. `TargetFileName` — dynamic string
19. `UniqueID` — string (GUID)
20. `AreaColor` — uint32
21. `Color` — uint32
22. `PinColor` — uint32
23. `OverideColors` — boolean
24. `DisplayFieldNames` — boolean
25. `DesignatorLocked` — boolean
26. `PartIDLocked` — boolean (with default)
27. `PinsMoveable` — boolean
28. `AliasList` — dynamic string (comma-separated)
29. `NotUseLibraryName` — boolean (inverted)
30. `NotUseDBTableName` — boolean (inverted)
31. `DesignItemId` — dynamic string
32. `VaultGUID` — dynamic string
33. `ItemGUID` — dynamic string
34. `RevisionGUID` — dynamic string
35. `SymbolVaultGUID` — dynamic string
36. `SymbolItemGUID` — dynamic string
37. `SymbolRevisionGUID` — dynamic string
38. `GenericComponentTemplateGUID` — dynamic string
39. `HasOnlyCurrentPartInfo` — boolean
40. `AllPinCount` — int16
41. `KeyComponentUniqueId` — dynamic string
42. `ComponentKind` — byte (version-aware encoding)
43. `ComponentKindVersion2` — byte (conditional)
44. `ComponentKindVersion3` — byte (conditional, value 6 = Jumper)
45. `CustomDisplayModeName{N}` — dynamic string (repeated for each display mode)

### GraphicalObject Base Fields (via ExportGraphicalObject → ExportDataObject)

DataObject fields:
- `OwnerIndex` — int32
- `IsNotAccesible` — boolean (inverted)
- `OwnerIndexAdditionalList` — boolean
- `IndexInSheet` — int32

GraphicalObject fields:
- `OwnerPartId` — int16
- `OwnerPartDisplayMode` — byte
- `SelectionMemory` — byte
- `UnionIndex` — int32
- `GraphicallyLocked` — boolean

### SchLib Library Header Fields (FileHeader Stream)

From `FileFormatV5.ExportLibrary()`:

- Font table (FontIdCount, FontName{N}, Size{N}, etc.)
- `UseMBCS` — boolean (always true)
- `IsBOC` — boolean
- `Description` — dynamic string
- `DocumentBorderStyle` — enum
- `SheetStyle` — byte
- `WorkspaceOrientation` — enum
- `BorderOn` — boolean
- `TitleBlockOn` — boolean
- `SheetNumberSpaceSize` — int32
- `Color` — uint32
- `AreaColor` — uint32
- `SnapGridOn` — boolean
- `SnapGridSize` — coord
- `VisibleGridOn` — boolean
- `VisibleGridSize` — coord
- `CustomX` — coord
- `CustomY` — coord
- `UseCustomSheet` — boolean
- `ShowHiddenPins` — boolean
- `ReferenceZonesOn` — boolean (inverted)
- `Display_Unit` — enum
- `AlwaysShowCD` — boolean
- `ReleaseVaultGUID` — dynamic string
- `FolderGUID` — dynamic string
- `LifeCycleDefinitionGUID` — dynamic string
- `RevisionNamingSchemeGUID` — dynamic string

### SchLib Save Sequence (SchDataExporterLibraryV5)

1. `FixDuplicatedLibRefs()` — deduplicate component names
2. `WriteBaseWarehouseHeader()` — write `/FileHeader` with component index:
   - RECORD=0, HEADER, Weight, MinorVersion(9), UniqueID
   - Library properties (font table, grid settings, etc.)
   - CompCount, then for each component: LibRef{i}, CompDescr{i}, PartCount{i}, AliasCount{i}, Comp{i}Alias{j}
3. `WriteBaseWarehouseData()` — for each component:
   - Write alias `/<alias>/Redirection` streams (RECORD=0, SectionName=primary)
   - Write `/<component>/Data` stream:
     - Component record (RECORD mode, code 42)
     - Child records (pins in BINARY mode code 2, others in RECORD mode)
     - Terminate with RECORD=0
   - After all components: write `/SectionKeys`
4. `WriteExtendedWarehouse()` — write `/Storage` stream (embedded images)
5. `PrepareAndWritePinsExtendedData()` — per component:
   - `/<component>/PinFrac` — fractional coordinates (3x int32: locX_frac, locY_frac, length_frac)
   - `/<component>/PinDesc` — long descriptions (>254 chars, ASCII)
   - `/<component>/PinMiscData` — PairSwapID (Unicode parameters)
   - `/<component>/PinTextData` — custom pin text display settings (binary)
   - `/<component>/PinWideText` — Unicode pin text (Desc, Name, Desig, SwapId, SwapIDPart, DefValue)
   - `/<component>/PinSymbolLineWidth` — symbol line widths (Unicode parameters)
   - `/<component>/PinPackageLength` — package lengths (Unicode parameters)
   - `/<component>/PinPropagationDelay` — propagation delays (Unicode parameters)
   - `/<component>/PinFunctionData` — pin function definitions (Unicode parameters)
6. `WriteAdditionalWarehouse()` — write `/LibAdditional` and `/<component>/Additional` if applicable

### SchLib Load Sequence (SchDataImporterLibraryV5)

1. `ImportBaseWarehouse()`:
   - `ImportLibrary()` — read `/FileHeader` (RECORD, HEADER, Weight, MinorVersion, UniqueID, library properties)
   - `componentSectionKeyList.Load()` — read `/SectionKeys` (optional)
   - `FindFirstStream("Data")` — iterate all `/<component>/Data` streams
   - Outer loop: read component record (code 42), ImportFromFile, AddComponent
   - Inner loop: read child records until code 0, ImportFromFile, UpdateOwner
2. `ImportExtendedWarehouse()`:
   - Read `/Storage` stream (embedded images, code 208)
   - `ReadAndProcessPinsExtendedData()` — read all 9 pin extended streams per component
3. `ImportAdditionalWarehouse()`:
   - Read `/LibAdditional` header, then `/<component>/Additional` streams

### PCB Record Types (from Altium.SDK.Interfaces)

| Code | Name | Description |
|------|------|-------------|
| 0 | eNoObject | No object |
| 1 | eArcObject | PCB arc |
| 2 | ePadObject | PCB pad |
| 3 | eViaObject | PCB via |
| 4 | eTrackObject | PCB track/trace |
| 5 | eTextObject | PCB text |
| 6 | eFillObject | PCB fill |
| 7 | eConnectionObject | Ratsnest connection |
| 8 | eNetObject | Net |
| 9 | eComponentObject | PCB component/footprint |
| 10 | ePolyObject | PCB polygon pour |
| 11 | eRegionObject | PCB region |
| 12 | eComponentBodyObject | 3D body |
| 13 | eDimensionObject | Dimension |
| 14 | eCoordinateObject | Coordinate |
| 15 | eClassObject | Object class |
| 16 | eRuleObject | Design rule |
| 17 | eFromToObject | From-to |
| 18 | eDifferentialPairObject | Differential pair |
| 19 | eViolationObject | DRC violation |
| 20 | eEmbeddedObject | Embedded object |
| 21 | eEmbeddedBoardObject | Embedded board |
| 22 | eSplitPlaneObject | Split plane |
| 23 | eTraceObject | Trace |
| 24 | eSpareViaObject | Spare via |
| 25 | eBoardObject | Board |
| 26 | eBoardOutlineObject | Board outline |

### PCB Object Properties (from SDK.Interfaces and InteractiveProperties)

**IPCB_Pad**: XLocation, YLocation, TopXSize/TopYSize/MidXSize/MidYSize/BotXSize/BotYSize, XStackSizeOnLayer/YStackSizeOnLayer, TopShape/MidShape/BotShape/StackShapeOnLayer, HoleSize, Rotation(double), Plated(bool), Mode(TPadMode), PinDescriptorString, SwapID_Pad, SwapID_Part, OwnerPart_ID, IsConnectedToPlane(per-layer)

**IPCB_Track**: X1, Y1, X2, Y2, Width, Length(calculated)

**IPCB_Via**: XLocation, YLocation, Size, SizeOnLayer, StackSizeOnLayer, HoleSize, LowLayer, HighLayer, StartLayer, StopLayer, Mode(TPadMode), IsConnectedToPlane(per-layer)

**IPCB_Arc**: CenterX, CenterY, Radius, StartAngle(double), EndAngle(double), Width

**IPCB_Fill**: LocationX, LocationY, Width, Length, Rotation(double)

**IPCB_Region**: EdgeCount, RegionKind(TRegionKind), CavityHeight, ArcApproximation, ShapeSegmentCount

**IPCB_LibComponent**: Pattern(name), Description, Height, ItemGUID, ItemRevisionGUID

### PCB Binary File Structure (from Ghidra decompilation of Altium.PCB.BinaryLoader.dll)

The PCB binary format is a **CFB (OLE Compound Document)** with multiple streams, each containing one section type. Unlike SchLib which uses parametric `|KEY=VALUE|` encoding for most records, **PCB records use BOTH parametric AND fixed binary layouts** depending on the section.

#### PCB CFB Streams (from `FUN_01847b90` init function)

```
Board6                    - Board settings (parametric)
Advanced Placer Options6  - Placer options
Advanced Router Options6  - Router options
Design Rule Checker Options6 - DRC options
Pin Swap Options6         - Pin swap options
Classes6                  - Object classes
Nets6                     - Net definitions
Components6               - Component/footprint instances
Polygons6                 - Polygon pours
Dimensions6               - Dimensions
Coordinates6              - Coordinate objects
EmbeddedBoards6           - Embedded boards
Connections6              - Ratsnest connections
Rules6                    - Design rules
NewRules6                 - New rules format
FromTos6                  - From-to connections
DifferentialPairs6        - Differential pairs
Embeddeds6                - Embedded objects
Arcs6                     - Arc primitives
Pads6                     - Pad primitives
Vias6                     - Via primitives
Tracks6                   - Track primitives
Texts6                    - Text primitives
Fills6                    - Fill primitives
ShapeBasedRegions6        - Shape-based regions
Regions6                  - Regions
ShapeBasedComponentBodies6 - 3D component bodies (shape-based)
ComponentBodies6          - 3D component bodies
WideStrings6              - Wide (Unicode) strings
EmbeddedFonts6            - Embedded fonts
SplitPlaneRegions6        - Split plane regions
UnionNames                - Union names
UnionRelations            - Union relations
SmartUnions               - Smart unions
```

Each section also has a **Header** and **Data** sub-stream in the CFB (e.g., `HeaderTrack`/`DataTrack` for tracks).

#### PCB Record Dispatch (from `FUN_018b8ee0`)

The parametric `Record` field maps to object types:

| Record Value | Object Type | Delphi Class Pointer |
|-------------|-------------|---------------------|
| `Board` | Board settings | `PTR_PTR_017a0028` (boards share with tracks?) |
| `Class` | Object class | `PTR_PTR_018261f8` |
| `ClassCluster` | Class cluster | `PTR_PTR_01827720` |
| `Component` | Component | `PTR_PTR_0179bdc8` |
| `Polygon` | Polygon pour | `PTR_PTR_017ad740` |
| `Dimension` | Dimension | `PTR_PTR_01573e98` (DimensionKind=8) |
| `Coordinate` | Coordinate | `PTR_PTR_0179dda8` |
| `Track` | Track/trace | `PTR_PTR_017a0028` |
| `Connection` | Ratsnest | `PTR_PTR_0179f828` |
| `Text` | Text | `PTR_PTR_017a1218` |
| `Fill` | Fill | `PTR_PTR_017a25e8` |
| `Embedded` | Embedded obj | `PTR_PTR_017b3618` |
| `EmbeddedBoard` | Embedded board | `PTR_PTR_0179e978` |
| `Fromto` | From-to | `PTR_PTR_0179d428` |
| `Region` | Region | `PTR_PTR_017a2d38` |
| `ComponentBody` | 3D body | `PTR_PTR_017a5778` |
| `DXPRule` / `Rule` | Design rule | `PTR_PTR_01582068` |

After creating the object, the dispatch calls virtual method at vtable offset `0x1c0` to load parametric data.

#### PCB Section Delphi Classes (RTTI strings)

| Class Name | Version | Description |
|-----------|---------|-------------|
| `TPCBBinaryFileV6` | V6 | Main PCB binary file class |
| `TPCBBinaryFile` | Legacy | Legacy PCB binary file |
| `TPCB3BinaryFile` | V3 | Protel 99 binary file |
| `TPCBLibraryBinaryFileV6` | V6 | PcbLib binary file |
| `TPCBLibraryBinaryFile` | Legacy | Legacy PcbLib |
| `TPCBLoader` | - | Main loader class |
| `TBoardSection` | - | Board section |
| `TTracksSectionK/L` | - | Tracks section |
| `TArcsSectionK/L` | - | Arcs section |
| `TPadsSectionK/L` | - | Pads section |
| `TViasSectionK/L` | - | Vias section |
| `TTextsSectionK/L` | - | Texts section |
| `TFillsSectionK/L` | - | Fills section |
| `TComponentsSectionK/L` | - | Components section |
| `TNetsSection` | - | Nets section |
| `TClassesSection` | - | Classes section |
| `TRulesSection` | - | Rules section |
| `TPolygonsSection` | - | Polygons section |
| `TDimensionsSection` | - | Dimensions section |
| `TRegionsSection` | - | Regions section |

Key methods found:
- `TPCBLibraryBinaryFileV6.ReadComponentParamsTOC` (at ~`0x01b28481`)
- `TPCBLibraryBinaryFileV6.ReadLayerKindMapping` (at ~`0x01b291c9`)

#### Board Record Parametric Fields (from `FUN_015f0630`)

The Board6 record uses parametric `|KEY=VALUE|` format:

```
Record=Board
FileName=<string>
Kind=Protel_Advanced_PCB
Version=3.00
Date=<date string>
Time=<time string>
OriginX=<coord>
OriginY=<coord>
BigVisibleGridSize=<double>
VisibleGridSize=<double>
ElectricalGridRange=<coord>
ElectricalGridEnabled=<bool>
SnapGridSize=<double>
SnapGridSizeX=<double>
SnapGridSizeY=<double>
TrackGridSize=<double>
ViaGridSize=<double>
ComponentGridSize=<double>
ComponentGridSizeX=<double>
ComponentGridSizeY=<double>
DotGrid=<bool>
DisplayUnit=<byte>
Plane1NetName=<string>
Plane2NetName=<string>
Plane3NetName=<string>
Plane4NetName=<string>
```

#### PCB Object Interface Properties (from IPCB_ interfaces)

**IPCB_Primitive** (base for all objects):
- `ObjectID` (int) - object type code
- `Layer` (int) - layer ID (V7_Layer)
- `Selected`, `IsPreRoute`, `Enabled`, `Used`, `DRCError` (bool flags)
- `IsKeepout`, `PolygonOutline`, `InBoard`, `InPolygon`, `InComponent`, `InNet` (bool)
- `UserRouted`, `TearDrop`, `IsTenting`, `IsTenting_Top/Bottom` (bool)
- `IsTestPoint_Top/Bottom`, `IsAssyTestPoint_Top/Bottom` (bool)
- `Index` (ushort), `UnionIndex` (int)
- `PowerPlaneConnectStyle`, `ReliefConductorWidth`, `ReliefEntries`, `ReliefAirGap` (int)
- `PasteMaskExpansion`, `SolderMaskExpansion`, `PowerPlaneClearance`, `PowerPlaneReliefExpansion` (int)
- `UniqueId` (string), `Handle` (string)
- `Export_ToParameters(ref string)` - serializes to parametric format

**IPCB_Track**:
- `X1`, `Y1`, `X2`, `Y2` (int - coordinates)
- `Width` (int)

**IPCB_Arc**:
- `CenterX`, `CenterY` (int - coordinates)
- `Radius` (int), `LineWidth` (int)
- `StartAngle`, `EndAngle` (double - degrees)
- `StartX`, `StartY`, `EndX`, `EndY` (int - calculated)

**IPCB_Pad**:
- `XLocation`, `YLocation` (int)
- `TopXSize`, `TopYSize`, `MidXSize`, `MidYSize`, `BotXSize`, `BotYSize` (int)
- `TopShape`, `MidShape`, `BotShape` (int - pad shape enum)
- `XStackSizeOnLayer`, `YStackSizeOnLayer`, `StackShapeOnLayer` (per-layer)
- `HoleSize`, `HoleWidth` (int), `HoleRotation` (double)
- `Rotation` (double), `Plated` (bool)
- `Mode` (int - TPadMode), `DrillType` (int), `HoleType` (int)
- `Name` (string), `SwapID_Pad`, `SwapID_Part`, `SwappedPadName` (string)
- `OwnerPart_ID` (int), `JumperID` (int)
- `SolderMaskExpansionFromHoleEdge` (bool)
- `HolePositiveTolerance`, `HoleNegativeTolerance` (int)
- `XPadOffsetOnLayer`, `YPadOffsetOnLayer` (per-layer)

**IPCB_Via**:
- `XLocation`, `YLocation` (int)
- `Size` (int), `SizeOnLayer` (per-layer), `StackSizeOnLayer` (per-layer)
- `HoleSize` (int), `Height` (int)
- `LowLayer`, `HighLayer` (IV7_Layer), `StartLayer`, `StopLayer` (object)
- `Mode` (int), `ShapeOnLayer` (per-layer)
- `SolderMaskExpansionFromHoleEdge` (bool)
- `HolePositiveTolerance`, `HoleNegativeTolerance` (int)

**IPCB_Text** (extends IPCB_RectangularPrimitive):
- `Size` (int), `Width` (int)
- `FontID` (short), `FontName` (string), `CharSet` (byte)
- `Text` (string), `UnderlyingString` (string), `ConvertedString` (string)
- `Mirror` (bool), `UseTTFonts` (bool), `Bold` (bool), `Italic` (bool)
- `Inverted` (bool), `InvertedTTTextBorder` (int)
- `TTFTextWidth`, `TTFTextHeight` (int)
- `InvRectWidth`, `InvRectHeight` (int), `UseInvertedRectangle` (bool)
- `TTFInvertedTextJustify` (int), `TTFOffsetFromInvertedRect` (int)
- `BarCode*` properties (BarCodeKind, BarCodeRenderMode, BarCodeMinWidth, etc.)
- `Multiline` (bool), `WordWrap` (bool), `MultilineTextWidth/Height` (int)
- `TextKind` (int), `BorderSpaceType` (int)

**IPCB_Fill** (extends IPCB_RectangularPrimitive):
- `LocationX`, `LocationY` (int)
- `Width` (int), `Length` (int)

**IPCB_Component** (extends IPCB_Group):
- `ChannelOffset` (int), `ComponentKind` (int)
- `Pattern` (string - footprint name)
- `NameOn` (bool), `CommentOn` (bool), `LockStrings` (bool)
- `GroupNum` (int), `Rotation` (double), `Height` (int)
- `NameAutoPos` (int), `CommentAutoPos` (int)
- `SourceDesignator`, `SourceUniqueId`, `SourceHierarchicalPath` (string)
- `SourceFootprintLibrary`, `SourceComponentLibrary`, `SourceLibReference` (string)
- `SourceDescription`, `FootprintDescription` (string)
- `DefaultPCB3DModel` (string), `IsBGA` (bool)
- `EnablePinSwapping`, `EnablePartSwapping` (bool)
- `SourceCompDesignItemID` (string)
- `FlippedOnLayer` (bool), `JumpersVisible` (bool)
- `VaultGUID`, `ItemGUID`, `ItemRevisionGUID` (string)
- `FootprintConfiguratorName`, `FootprintConfigurableParameters_Encoded` (string)

**IPCB_Region**:
- `EdgeCount` (int)
- `RegionKind` (TRegionKind)
- `CavityHeight` (int)
- `ArcApproximation`, `ShapeSegmentCount` (int)

### PCB Format Status

The PCB binary serialization format (how records are encoded on disk) is in **native Delphi DLLs**:
- `Altium.PCB.BinaryLoader.dll` (54MB) — PCB binary file loader/parser, **108,701 functions**, analyzed in Ghidra
- `Altium.PCB.DataModel.dll` (14MB) — PCB data model

The .NET interfaces reveal what properties exist. The Ghidra analysis of BinaryLoader reveals:
- CFB stream structure (34 section streams)
- Record dispatch function mapping parametric Record values to Delphi classes
- Board record parametric field order
- Section class hierarchy (TXxxSectionK/L variants)

#### PCB Record Parametric Fields (from Ghidra decompilation of writer functions)

**Track** (from `FUN_015f7520`):
```
X1=<coord>             (likely, from DAT_015f761c)
Y1=<coord>             (likely, from DAT_015f7630)
X2=<coord>             (likely, from DAT_015f7644)
Y2=<coord>             (likely, from DAT_015f7658)
Width=<coord>
SubPolyIndex=<int>
UserRouted=<bool>
TearDrop=<bool>
```

**Arc** (from `FUN_015f0470` / `FUN_015f7be0`):
```
Location.X=<coord>     (via vtable 0x98 = GetState_CenterX)
Location.Y=<coord>     (via vtable 0xa0 = GetState_CenterY)
Radius=<coord>
StartAngle=<double>
EndAngle=<double>
Width=<coord>
SubPolyIndex=<int>
```

**Pad** (from `FUN_015f22c0`):
```
Name=<string>          (pad name, first 4 chars)
Location.X=<coord>     (via vtable 0x98)
Location.Y=<coord>     (via vtable 0xa0)
XSize=<coord>          (if all layers same) OR TopXSize/MidXSize/BotXSize
YSize=<coord>          (if all layers same) OR TopYSize/MidYSize/BotYSize
Shape=<string>         (if all layers same) OR TopShape/MidShape/BotShape
HoleSize=<coord>
Rotation=<double>
Plated=<bool>
DaisyChain=<int>
(per-layer pad cache data follows)
```

**Via** (from `FUN_015f6a20`):
```
HoleWidth=<coord>
Width=<coord>
ViaStyle=<string>      (from enum table at PTR_PTR_01d82e30)
```

**Text** (from `FUN_015f7310`):
```
Location.X=<coord>     (via vtable 0x98)
Location.Y=<coord>     (via vtable 0xa0)
Height=<coord>
Font=<int>
Rotation=<double>
Mirror=<bool>
Text=<string>
Width=<coord>
```

**Board** (from `FUN_015f0630`):
```
Record=Board
FileName=<string>
Kind=Protel_Advanced_PCB
Version=3.00
Date=<date>
Time=<time>
OriginX=<coord>
OriginY=<coord>
BigVisibleGridSize=<double>
VisibleGridSize=<double>
ElectricalGridRange=<coord>
ElectricalGridEnabled=<bool>
SnapGridSize/X/Y=<double>
TrackGridSize=<double>
ViaGridSize=<double>
ComponentGridSize/X/Y=<double>
DotGrid=<bool>
DisplayUnit=<byte>
Plane1-4NetName=<string>
```

**Polygon** (from `FUN_015f2c60`):
```
PolygonType=<string>   (from enum table at PTR_PTR_01d84918)
PourOver=<bool>
RemoveDead=<bool>
GridSize=<coord>
TrackWidth=<coord>
HatchStyle=<int>
UseOctagons=<bool>
MinPrimLength=<coord>
(variable-length vertex list follows)
```

**Component** (from `FUN_015f86b0`, reader):
```
Pattern=<string>               (footprint name)
Location.X=<coord>             (via DAT_015f8ffc)
Location.Y=<coord>             (via DAT_015f900c)
NameOn=<bool>
CommentOn=<bool>
GroupNum=<int>
Count=<int>
Rotation=<double>
FootPrint=<string>             (legacy field)
FileName=<string>              (legacy library path)
IntegratedLibraryName=<string>
Location.x=<coord>             (lowercase, legacy V3/V4)
Location.y=<coord>             (lowercase, legacy V3/V4)
Designator.Text=<string>
Designator.Location.X=<coord>
Designator.Location.Y=<coord>
Designator.Height=<coord>
Designator.Font=<short>
Designator.Rotation=<double>
Designator.Mirror=<bool>
Designator.Width=<coord>
Designator.Visible=<bool>
Designator.Layer=<layer>
Comment.Text=<string>
Comment.Location.X=<coord>
Comment.Location.Y=<coord>
Comment.Height=<coord>
Comment.Font=<short>
Comment.Rotation=<double>
Comment.Mirror=<bool>
Comment.Width=<coord>
Comment.Visible=<bool>
Comment.Layer=<layer>
```

**Net** (from `FUN_015f9ea0`):
```
Name=<string>
RECORD=<string>        (removed after reading)
(remaining params written as key-value block)
```

### PCB Binary Section Format

Each primitive section (Tracks6, Arcs6, Pads6, etc.) in the CFB has a **Header** and **Data** sub-stream:
- `HeaderTrack` / `DataTrack` — Track header + binary data
- (Similar for other sections)

The header stream contains the parametric records (Record=Track with |KEY=VALUE|).
The data stream contains the binary records (fixed-size binary fields).

**Record framing** in Data streams (verified from real PcbDoc files):
- `u8` — record type (1=Arc, 2=Pad, 3=Via, 4=Track, 5=Text, 6=Fill)
- `u32` — record data length (LE)
- `N bytes` — binary record data (layout depends on record type)

**Exception**: Pad records (type=2) use multi-block framing — see Pad section below.

**Note**: The `Tracks6/Header` stream is just `u32 record_count` (4 bytes). The parametric `|KEY=VALUE|` data is in the `PrimitiveParameters` stream, NOT interleaved with binary data.

The column indices for Tracks are `[0, 1, 6, 7, 8]` (v1, from `FUN_01a04e20`) or `[0, 1, 8, 9, 10]` (v2, from `FUN_01a0a3b0`).

**Two format versions**:
- v1 (`FUN_01a05e50`): Records are 0x28 bytes (40 bytes), read by `FUN_01a077c0`
- v2 (`FUN_01a0b300`): Records are 0x50 bytes (80 bytes), read by `FUN_01a0d5f0`

Both versions read: HeaderPrim → DataPrim → HeaderTrack → DataTrack → HeaderRegion → DataRegion → BinPaths.

### PCB Coordinate System

All PCB coordinates are **signed int32** in internal units, little-endian.
- **1 mil = 10,000 internal units** (PCB system)
- Note: SchLib system uses 100,000 units/mil — different scale!

### PCB Primitive Header (13 bytes)

Every binary record starts with this header (from Ghidra decompilation of FUN_01849fd0):

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 1 | u8 | **Layer** (converted from internal layer enum via FUN_00dd7550) |
| 1 | 2 | u16 | **Flags** (bit field, built from boolean getters) |
| 3 | 2 | u16 | **Net ID** (0xFFFF = none) |
| 5 | 2 | u16 | **Polygon Index** (0xFFFF = none) |
| 7 | 2 | u16 | **Component Index** (0xFFFF = none) |
| 9 | 2 | u16 | **Unknown Ref 4** (0xFFFF = none) |
| 11 | 2 | u16 | **Unknown Ref 5** (0xFFFF = none) |

**Flags bit field** (offset 1-2, u16):
| Bit | Mask | Meaning |
|-----|------|---------|
| 0 | 0x0001 | FUN_0182a6c0 (unknown) |
| 1 | 0x0002 | FUN_0182afc0 (polygon outline?) |
| 2 | 0x0004 | VMT+0x60 (locked) |
| 3 | 0x0008 | FUN_0182c020 (unknown) |
| 4 | 0x0010 | VMT+0x100 (unknown) |
| 5 | 0x0020 | FUN_0182bef0 (tent_top?) |
| 6 | 0x0040 | FUN_0182bec0 (tent_bottom?) |
| 7 | 0x0080 | FUN_0182bf80 (test_fab_top?) |
| 8 | 0x0100 | FUN_0182bf50 (test_fab_bottom?) |
| 9 | 0x0200 | FUN_0182be90 (unknown) |

Note: KiCad splits bytes 1-2 as separate flags1/flags2 bytes; Altium internally treats them as a single u16.

### Record Type IDs

| ID | Type |
|----|------|
| 1 | Arc |
| 2 | Pad |
| 3 | Via |
| 4 | Track |
| 5 | Text |
| 6 | Fill |
| 11 | Region |

### Common Primitive Header (13 bytes)

All simple binary records (Track, Arc, Fill) share this header (from Ghidra FUN_01849fd0):
```
Offset  Size  Type   Field
  0     u8    layer          # Altium layer ID (converted from internal enum via FUN_00dd7550)
  1     u8    flags1         # bit 0x04 = NOT locked, 0x02 = polygon_outline, 0x20 = tent_top,
                             #   0x40 = tent_bottom, 0x80 = test_fab_top
  2     u8    flags2         # 0x02 = keepout, 0x01 = test_fab_bottom, 0x10 = teardrop (regions)
  3     u16   net            # Net index (0xFFFF = none)
  5     u16   polygon        # Parent polygon index (0xFFFF = none)
  7     u16   component      # Component index (0xFFFF = none)
  9     u16   ref4           # Unknown reference (0xFFFF = none, KiCad skips)
 11     u16   ref5           # Unknown reference (0xFFFF = none, KiCad skips)
```

Note: Via header is slightly different — byte 0 is layer (KiCad skips it), flags1/flags2 at bytes 1-2.
Note: Pad header has same flag bits but different semantics at bytes 1-2 (see Pad section).

### Track Binary Record (49 bytes in AD26, minimum 36)

From Ghidra decompilation (FUN_01856d20 + FUN_0185db80):

| Offset | Size | Type | Field | Ghidra function |
|--------|------|------|-------|-----------------|
| 0 | 13 | - | Common Header (layer, flags, net, polygon, component) | FUN_01849fd0 |
| 13 | 4 | i32 | **Start X** | FUN_017befe0 |
| 17 | 4 | i32 | **Start Y** | FUN_017bf040 |
| 21 | 4 | i32 | **End X** | FUN_017bf0a0 |
| 25 | 4 | i32 | **End Y** | FUN_017bf100 |
| 29 | 4 | i32 | **Width** | FUN_017bef70 |
| 33 | 2 | u16 | **SubPolyIndex** | FUN_017bef40 |
| 35 | 1 | u8 | **UserRouted** (bool) | FUN_0182a700 |
| 36 | 4 | i32 | **UnionIndex** | FUN_0182a810 |
| 40 | 1 | u8 | **Track-specific bool** | FUN_017bef10→FUN_0185e1c0 |
| 41 | 4 | i32 | **Layer enum index** | FUN_01829f00→FUN_00dd7410 |
| 45 | 4 | i32 | **KeepoutRestrictions** | FUN_017ca450→FUN_017ca510 |

Total: 49 bytes (0x31). Note: Track has 1 extra byte (offset 40) compared to Arc/Fill trailing pattern.
altium2kicad reads `UNIONINDEX` at offset 36 (as u8) and `USERROUTED` at offset 44 (wrong — that's inside layer_enum).

### Arc Binary Record (60 bytes in AD26, minimum 45)

From Ghidra decompilation (FUN_01857610 + FUN_0185dda0):

| Offset | Size | Type | Field | Ghidra function |
|--------|------|------|-------|-----------------|
| 0 | 13 | - | Common Header (layer, flags, net, polygon, component) | FUN_01849fd0 |
| 13 | 4 | i32 | **Center X** | VMT+0x98 |
| 17 | 4 | i32 | **Center Y** | VMT+0xa0 |
| 21 | 4 | i32 | **Radius** | FUN_017bf3b0 |
| 25 | 8 | f64 | **Start Angle** (degrees, IEEE 754) | FUN_017bf440 |
| 33 | 8 | f64 | **End Angle** (degrees, IEEE 754) | FUN_017bf4b0 |
| 41 | 4 | i32 | **Width** (line width) | FUN_017bf520 |
| 45 | 2 | u16 | **SubPolyIndex** | FUN_017bf3e0 |
| 47 | 1 | u8 | **UserRouted** (bool) | FUN_0182a700 |
| 48 | 4 | i32 | **UnionIndex** | FUN_0182a810 |
| 52 | 4 | i32 | **Layer enum index** | FUN_01829f00→FUN_00dd7410 |
| 56 | 4 | i32 | **KeepoutRestrictions** | FUN_017ca450→FUN_017ca510 |

Total: 60 bytes (0x3c).

### Via Binary Record (330 bytes in AD26, minimum ~30)

Header differs from Track/Arc/Fill — byte 0 is skipped (layer), flags at bytes 1-2.
Uses single subrecord framing: `u8 type(3) + u32 len + data`.

**Subrecord 1 fields** (from KiCad AVIA6 + Ghidra):
```
Offset  Size  Type   Field                  KiCad Name
  0     1     u8    (skip/layer)            -
  1     1     u8    flags1                  is_test_fab_top(0x80), is_tent_bottom(0x40),
                                             is_tent_top(0x20), is_locked(0x04 inverted)
  2     1     u8    flags2                  is_test_fab_bottom(0x01)
  3     2     u16   net                     net
  5     8     -     (skip)                  -
 13     4     i32   position_x              position.x
 17     4     i32   position_y              position.y
 21     4     i32   diameter                diameter
 25     4     i32   hole_size               holesize
 29     1     u8    layer_start             layer_start
 30     1     u8    layer_end               layer_end
```

**Extended fields** (if subrecord1 > 74 bytes, from KiCad AVIA6):
| Offset | Size | Type | KiCad Name |
|--------|------|------|------------|
| 31 | 1 | u8 | Unknown |
| 32 | 4 | i32 | `thermal_relief_airgap` |
| 36 | 1 | u8 | `thermal_relief_conductorcount` |
| 37 | 1 | - | Skip |
| 38 | 4 | i32 | `thermal_relief_conductorwidth` |
| 42 | 4 | i32 | Unknown (20mil?) |
| 46 | 4 | i32 | Unknown (20mil?) |
| 50 | 4 | - | Skip |
| 54 | 4 | i32 | `soldermask_expansion_front` |
| 58 | 8 | - | Skip |
| 66 | 1 | u8 | `soldermask_expansion_manual` (bit 0x02) |
| 67 | 7 | - | Skip |
| 74 | 1 | u8 | `viamode` (0=simple, 1=pad-stack) |
| 75 | 128 | 32×i32 | `diameter_by_layer[32]` |

**Additional extended fields** (if subrecord1 ≥ 246 bytes):
| Offset | Size | Type | KiCad Name |
|--------|------|------|------------|
| 203 | 38 | - | Skip |
| 241 | 1 | u8 | `soldermask_expansion_linked` (bit 0x01) |
| 242 | 4 | i32 | `soldermask_expansion_back` |

**Premium extended fields** (if subrecord1 ≥ 307 bytes, AD26):
| Offset | Size | Type | KiCad Name |
|--------|------|------|------------|
| 246 | 45 | - | Skip |
| 291 | 4 | i32 | `pos_tolerance` |
| 295 | 4 | i32 | `neg_tolerance` |

**Via writer architecture** (from Ghidra FUN_0187fa70):
The Via binary data in the stream consists of multiple sections:
1. **Core via data**: 0xF6 (246) bytes — serialized by FUN_0185b5a0
   - Includes common header, position, diameter, hole, layers, per-layer diameters
   - At offset 0xCB (203): layer enum index
   - At offset 0xCF (207): start layer
   - At offset 0xD0 (208): end layer
   - Per-layer diameters written via large switch (60+ layers)
2. **Extended entries**: N × 9 bytes (with u32 count + u32 stride=9 headers)
3. **Additional section**: 0x2A (42) bytes — serialized by FUN_0185d0a0
4. **Pad layer entries**: M × 0x1E (30) bytes (with u32 count + u32 stride=30 headers)
5. **Trailing data**: 9 bytes — serialized by FUN_0185d900

Total length formula: `300 + N*9 + M*30 + 21` (where N=extended entries, M=pad layers)
For a standard via with no extras: 246 + 0 + 42 + 0 + 9 = 297 bytes (+ length headers)

### Fill Binary Record (50 bytes in AD26, minimum 37)

From Ghidra decompilation (FUN_018574c0 + FUN_0185dcd0):

| Offset | Size | Type | Field | Ghidra function |
|--------|------|------|-------|-----------------|
| 0 | 13 | - | Common Header (layer, flags, net, polygon, component) | FUN_01849fd0 |
| 13 | 4 | i32 | **Corner1 X** | FUN_017c96f0 |
| 17 | 4 | i32 | **Corner1 Y** | FUN_017c9750 |
| 21 | 4 | i32 | **Corner2 X** | FUN_017c9720 |
| 25 | 4 | i32 | **Corner2 Y** | FUN_017c9780 |
| 29 | 8 | f64 | **Rotation** (degrees) | FUN_017c96b0 |
| 37 | 1 | u8 | **UserRouted** (bool) | FUN_0182a700 |
| 38 | 4 | i32 | **UnionIndex** | FUN_0182a810 |
| 42 | 4 | i32 | **Layer enum index** | FUN_01829f00→FUN_00dd7410 |
| 46 | 4 | i32 | **KeepoutRestrictions** | FUN_017ca450→FUN_017ca510 |

Total: 50 bytes (0x32).

### Common Trailing Fields (Track/Arc/Fill Pattern)

All simple primitives (Track, Arc, Fill) share the same trailing field pattern after their type-specific data. KiCad skips these as a single block and only reads keepoutrestrictions at the end:

| Offset | Size | Type | Field | Getter | KiCad | altium2kicad |
|--------|------|------|-------|--------|-------|-------------|
| N+0 | 1 | u8 | **UserRouted** (bool) | FUN_0182a700 | skip | `USERROUTED` |
| N+1 | 4 | i32 | **UnionIndex** | FUN_0182a810 | skip | `UNIONINDEX` (reads u8) |
| N+5 | 4 | i32 | **Layer enum index** | FUN_01829f00→FUN_00dd7410 | skip | |
| N+9 | 4 | i32 | **KeepoutRestrictions** | FUN_017ca450→FUN_017ca510 | `keepoutrestrictions` (reads u8) | |

Where N is the offset after type-specific fields:
- Track: N=35, Arc: N=47, Fill: N=37

**Track exception**: Track inserts 1 extra byte at N+5 (FUN_017bef10→FUN_0185e1c0, a Track-specific boolean), shifting LayerEnum to N+6 and Keepout to N+10 (total 14 trailing bytes vs 13 for Arc/Fill).

**Identification**: `UserRouted` confirmed via Altium SDK `IPCB_Primitive.GetState_UserRouted()` and altium2kicad cross-reference. `UnionIndex` confirmed via altium2kicad `UNIONINDEX` field at same offset. Both are shared base class properties on all PCB primitives.

### Pad Binary Record (Multi-Block, Most Complex)

**Framing**: `u8 type(2)` + 6 subrecords, each with `u32 length` prefix (NO type byte per subrecord).

Total size: ~912 bytes per pad.

Subrecords (from Ghidra FUN_0187eb60 + KiCad APAD6):
1. **Pad Name** (WxString: content varies, e.g. "1", "A1")
2. **Unknown string** (often empty)
3. **Unknown string** (often `|&|0`)
4. **Unknown string** (often empty)
5. **Main Pad Data** (172 bytes in AD26, minimum 110 per KiCad)
6. **Per-Layer Stack Data** (596/628/651 bytes)

#### Subrecord 5: Main Pad Data (Ghidra FUN_0184ad40 + FUN_01858be0, 0xAC=172 bytes)

| Offset | Size | Type | Field | KiCad Name | Ghidra |
|--------|------|------|-------|------------|--------|
| 0 | 1 | u8 | **Layer** (74=multi-layer) | `layer` | FUN_01849fd0 |
| 1 | 1 | u8 | **Flags1** (0x80=test_fab_top, 0x40=tent_bottom, 0x20=tent_top, 0x04=locked) | `flags1` | |
| 2 | 1 | u8 | **Flags2** (0x01=test_fab_bottom) | `flags2` | |
| 3 | 2 | u16 | **Net** | `net` | |
| 5 | 2 | u16 | **Polygon** (KiCad skips) | - | |
| 7 | 2 | u16 | **Component** | `component` | |
| 9 | 4 | - | Skip (ref4+ref5, 0xFFFF init) | - | |
| 13 | 4 | i32 | **Position X** | `position.x` | VMT+0x98 |
| 17 | 4 | i32 | **Position Y** | `position.y` | VMT+0xA0 |
| 21 | 4 | i32 | **Top Size X** | `topsize.x` | FUN_017c4140(TopLayer) |
| 25 | 4 | i32 | **Top Size Y** | `topsize.y` | FUN_017c43f0(TopLayer) |
| 29 | 4 | i32 | **Mid Size X** | `midsize.x` | FUN_017c4140(BotLayer) |
| 33 | 4 | i32 | **Mid Size Y** | `midsize.y` | FUN_017c43f0(BotLayer) |
| 37 | 4 | i32 | **Bot Size X** | `botsize.x` | FUN_017c4140(MidLayer) |
| 41 | 4 | i32 | **Bot Size Y** | `botsize.y` | FUN_017c43f0(MidLayer) |
| 45 | 4 | i32 | **Hole Size** | `holesize` | FUN_017c3ae0 |
| 49 | 1 | u8 | **Top Shape** | `topshape` | FUN_013cc890(TopLayer) |
| 50 | 1 | u8 | **Mid Shape** | `midshape` | FUN_013cc890(BotLayer) |
| 51 | 1 | u8 | **Bot Shape** | `botshape` | FUN_013cc890(MidLayer) |
| 52 | 8 | f64 | **Rotation** (degrees) | `direction` | FUN_017c4be0 |
| 60 | 1 | u8 | **Is Plated** | `plated` | FUN_0185e240 |
| 61 | 1 | u8 | Unknown | - | FUN_017c4af0 |
| 62 | 1 | u8 | **Pad Mode** (0=simple,1=top-mid-bot,2=full-stack) | `padmode` | FUN_017c3eb0 |
| 63 | 4 | i32 | Unknown | - | FUN_017c5330 |
| 67 | 1 | u8 | **Thermal connect mode** | - | FUN_017cae40 |
| 68 | 4 | i32 | **Thermal relief air gap** | - | FUN_017cb500 |
| 72 | 2 | u16 | **Thermal relief spoke count** | - | FUN_017cb660 |
| 74 | 4 | i32 | **Thermal relief spoke width** | - | FUN_017cb3a0 |
| 78 | 4 | i32 | Unknown | - | FUN_017cb240 |
| 82 | 4 | i32 | Unknown | - | FUN_017cb0e0 |
| 86 | 4 | i32 | **Paste mask expansion** | `pastemaskexpansionmanual` | FUN_017cace0 |
| 90 | 4 | i32 | **Solder mask expansion** | `soldermaskexpansionmanual` | FUN_017cb870 |
| 94 | 2 | u16 | **Pad layer bitmask** (16 bits, layers 0x27-0x37) | - | loop 0x27..0x37 |
| 96 | 1 | u8 | Unknown | - | FUN_017caef0 |
| 97 | 1 | u8 | Unknown | - | FUN_017cb5b0 |
| 98 | 1 | u8 | Unknown | - | FUN_017cb710 |
| 99 | 1 | u8 | Unknown | - | FUN_017cb450 |
| 100 | 1 | u8 | **Paste mask expansion mode** (0=none,1=rule,2=manual) | `pastemaskexpansionmode` | FUN_017cb2f0 |
| 101 | 1 | u8 | **Solder mask expansion mode** | `soldermaskexpansionmode` | FUN_017cad90 |
| 102 | 1 | u8 | Unknown | - | FUN_017cb920 |
| 103 | 1 | u8 | Unknown | - | FUN_017cb190 |
| 104 | 1 | u8 | Unknown | - | FUN_017cb030 |
| 105 | 1 | u8 | **UserRouted** (bool, shared base class) | - | FUN_0182a700 |
| 106 | 4 | i32 | **UnionIndex** (shared base class) | - | FUN_0182a810 |
| 110 | 4 | i32 | Unknown | - | FUN_017c3fd0 |
| 114 | 4 | i32 | **Layer enum** | - | FUN_01829f00→FUN_00dd7410 |
| 118 | 1+1 | u8+u8 | Hole shape related | - | FUN_0185e2b0+FUN_0185e2a0 |
| 120 | 1 | u8 | Unknown | - | |
| 121 | 4 | i32 | Unknown | - | FUN_017cb7c0 |
| 125 | 1 | u8 | Unknown | - | FUN_0185e2c0 |
| 126 | 16 | GUID | **Jumper ID GUID 1** | - | FUN_0044cb60 |
| 142 | 16 | GUID | **Jumper ID GUID 2** | - | FUN_0044cb60 |
| 158 | 4 | i32 | Unknown | - | FUN_017c40a0 |
| 162 | 4 | i32 | Unknown | - | FUN_017c8600 |
| 166 | 4 | i32 | Unknown | - | FUN_017c85d0 |
| 170 | 1 | u8 | Unknown | - | FUN_017c8360 |
| 171 | 1 | u8 | = 1 (constant) | - | FUN_0185e2d0 |

Total: 0xAC = 172 bytes.

**Extended fields** (if subrecord5 ≥ 114, from KiCad):
| Offset | Size | Type | Field |
|--------|------|------|-------|
| 106 | 8 | f64 | **Hole Rotation** (if size > 110) |
| 114 | 1 | u8 | **To Layer** (if size ≥ 120) |
| 117 | 1 | u8 | **From Layer** |
| 160 | 4 | i32 | **Pad to Die Length** (if size ≥ 202) |
| 196 | 8 | f64 | **Pad to Die Delay** |

Note: KiCad offsets for extended fields assume sequential reading from 106+; in AD26 the core is always 172 bytes so these extended offsets only apply to older/shorter formats.

#### Subrecord 6: Per-Layer Stack Data (596/628/651 bytes, from KiCad APAD6_SIZE_AND_SHAPE)

| Offset | Size | Type | Field | Ghidra |
|--------|------|------|-------|--------|
| 0 | 116 | 29×i32 | **Inner Size X per Layer** | FUN_017c4140 loop |
| 116 | 116 | 29×i32 | **Inner Size Y per Layer** | FUN_017c43f0 loop |
| 232 | 29 | 29×u8 | **Inner Shape per Layer** | FUN_017c4ab0 loop |
| 261 | 1 | u8 | Skip |
| 262 | 1 | u8 | **Hole Shape** (0=Round, 1=Square, 2=Slot) | `holeshape` |
| 263 | 4 | i32 | **Slot Size** | `slotsize` |
| 267 | 8 | f64 | **Slot Rotation** | `slotrotation` |
| 275 | 128 | 32×i32 | **Hole Offset X per Layer** | `holeoffset[].x` |
| 403 | 128 | 32×i32 | **Hole Offset Y per Layer** | `holeoffset[].y` |
| 531 | 1 | u8 | Skip |
| 532 | 32 | 32×u8 | **Alt Shape per Layer** | `alt_shape` |
| 564 | 32 | 32×u8 | **Corner Radius % per Layer** | `cornerradius` |

Total: 596 bytes (KiCad minimum). Sizes 628 and 651 include additional data after offset 596.

**Pad shape data block** (0x274=628 bytes, from Ghidra FUN_0184b380 + FUN_01859790):
This is an extended variant that includes per-layer boolean presence flags (at offsets 0x253+layer_index) and additional fields:
| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0x105 (261) | 1 | u8 | FUN_017c4b70 |
| 0x106 (262) | 1 | u8 | FUN_017c3b10 |
| 0x107 (263) | 4 | i32 | FUN_017c3b50 |
| 0x10B (267) | 8 | f64 | FUN_017c3aa0 |

### Enum Values (validated against Altium SDK Delphi types)

**TLayer** (Layer IDs):
```
0=eNoLayer
1=eTopLayer, 2..31=eMidLayer1..eMidLayer30, 32=eBottomLayer
33=eTopOverlay, 34=eBottomOverlay
35=eTopPaste, 36=eBottomPaste
37=eTopSolder, 38=eBottomSolder
39..54=eInternalPlane1..eInternalPlane16
55=eDrillGuide, 56=eKeepOutLayer
57..72=eMechanical1..eMechanical16
73=eDrillDrawing, 74=eMultiLayer
75=eConnectLayer, 76=eBackGroundLayer       (display-only, not in binary files)
77=eDRCErrorLayer, 78=eHighlightLayer       (display-only)
79=eGridColor1, 80=eGridColor10             (display-only)
81=ePadHoleLayer, 82=eViaHoleLayer          (display-only)
```
Note: The TLayer enum in altium-types.md only lists Mechanical1-16, but cLayerStrings and
cDefaultLayerDrawingOrder in altium-constants.md show **Mechanical17-32** exist (eMechanical17..eMechanical32).
The constants file's layer string array places them between Mechanical16 and DrillDrawing, suggesting
layer IDs 73+ are shifted in newer Altium versions — or more likely these are in a separate extended
layer range. The cLayerStrings array order is: ...Mechanical16, Mechanical17..32, DrillDrawing, MultiLayer...
which implies Mechanical17=73, Mechanical18=74... but this conflicts with DrillDrawing=73 in the TLayer enum.
**This needs Ghidra verification** — AD26 likely uses an extended layer mapping.

**TShape** (Pad/Via shape):
```
0=eNoShape, 1=eRounded, 2=eRectangular, 3=eOctagonal
4=eCircleShape, 5=eArcShape, 6=eTerminator
7=eRoundRectShape, 8=eRotatedRectShape, 9=eRoundedRectangular
```
Note: KiCad only handles 1,2,3,9. Values 4-8 are less common.

**TExtendedHoleType** (Hole shape): 0=eRoundHole, 1=eSquareHole, 2=eSlotHole

**TExtendedDrillType** (Drill method): 0=eDrilledHole, 1=ePunchedHole, 2=eLaserDrilledHole, 3=ePlasmaDrilledHole

**TPadMode**: 0=ePadMode_Simple, 1=ePadMode_LocalStack, 2=ePadMode_ExternalStack

**TCacheState** (expansion mode — used for paste/solder mask expansion validity):
```
0=eCacheInvalid (None), 1=eCacheValid (Rule), 2=eCacheManual (Manual)
```
This is what KiCad calls "ALTIUM_MODE" — maps to PASTEMASKEXPANSIONMODE/SOLDERMASKEXPANSIONMODE values.

**TPadCache** (Pad thermal/mask cache record — field order in SDK):
```
PlaneConnectionStyle : TPlaneConnectionStyle
ReliefConductorWidth : TCoord
ReliefEntries        : SmallInt (i16)
ReliefAirGap         : TCoord
PowerPlaneReliefExpansion : TCoord
PowerPlaneClearance  : TCoord
PasteMaskExpansion   : TCoord
SolderMaskExpansion  : TCoord
Planes               : Word (u16)
--- followed by 9 TCacheState validity flags (one per field above) ---
PlaneConnectionStyleValid, ReliefConductorWidthValid, ReliefEntriesValid,
ReliefAirGapValid, PowerPlaneReliefExpansionValid, PasteMaskExpansionValid,
SolderMaskExpansionValid, PowerPlaneClearanceValid, PlanesValid
```

**TPlaneConnectionStyle**: 0=ePlaneNoConnect, 1=ePlaneReliefConnect, 2=ePlaneDirectConnect
**TPlaneConnectStyle** (alternate enum, DIFFERENT ordering): 0=eReliefConnectToPlane, 1=eDirectConnectToPlane, 2=eNoConnect
Note: Binary files appear to use TPlaneConnectStyle ordering (Relief=0, Direct=1, NoConnect=2) based on KiCad parser.

**TObjectId** (record type byte in binary framing):
```
0=eNoObject, 1=eArcObject, 2=ePadObject, 3=eViaObject
4=eTrackObject, 5=eTextObject, 6=eFillObject, 7=eConnectionObject
8=eNetObject, 9=eComponentObject, 10=ePolyObject, 11=eRegionObject
12=eComponentBodyObject, 13=eDimensionObject, 14=eCoordinateObject
15=eClassObject, 16=eRuleObject, 17=eFromToObject
18=eDifferentialPairObject, 19=eViolationObject
20=eEmbeddedObject, 21=eEmbeddedBoardObject
22=eTraceObject (internal), 23=eSpareViaObject (internal)
24=eBoardObject, 25=eBoardOutlineObject
```

**Text Type**: 0=Stroke, 1=TrueType, 2=Barcode

**TTextAutoposition**:
```
0=eAutoPos_Manual, 1=eAutoPos_TopLeft, 2=eAutoPos_CenterLeft
3=eAutoPos_BottomLeft, 4=eAutoPos_TopCenter, 5=eAutoPos_CenterCenter
6=eAutoPos_BottomCenter, 7=eAutoPos_TopRight, 8=eAutoPos_CenterRight
9=eAutoPos_BottomRight
```

**TPolyRegionKind**: 0=ePolyRegionKind_Copper, 1=ePolyRegionKind_Cutout, 2=ePolyRegionKind_NamedRegion
Note: KiCad extends beyond SDK: 2=Dashed Outline(?), 3=Unknown, 4=Cavity Definition. SDK only defines 0-2.

**TPolygonType**: 0=eSignalLayerPolygon, 1=eSplitPlanePolygon

**TPolyHatchStyle** (polygon fill style):
```
0=ePolyHatch90, 1=ePolyHatch45, 2=ePolyVHatch
3=ePolyHHatch, 4=ePolyNoHatch, 5=ePolySolid
```

**TConnectionMode**: 0=eRatsNestConnection, 1=eBrokenNetMarker

**TDimensionKind**:
```
0=eNoDimension, 1=eLinearDimension, 2=eAngularDimension
3=eRadialDimension, 4=eLeaderDimension, 5=eDatumDimension
6=eBaselineDimension, 7=eCenterDimension, 8=eOriginalDimension
9=eLinearDiameterDimension, 10=eRadialDiameterDimension
```

**TDimensionUnit**: 0=eMils, 1=eInches, 2=eMillimeters, 3=eCentimeters, 4=eDegrees, 5=eRadians, 6=eAutomaticUnit

**TDielectricType** (layer stack): 0=eNoDielectric, 1=eCore, 2=ePrePreg, 3=eSurfaceMaterial

**TLayerStackStyle**: 0=eLayerStack_Pairs, 1=eLayerStacks_InsidePairs, 2=eLayerStackBuildup

**TRuleKind** (52 rule types, 0-indexed — string IDs from cRuleIdStrings):
```
 0=Clearance               "Clearance"
 1=ParallelSegment         "ParallelSegment"
 2=MaxMinWidth             "Width"
 3=MaxMinLength            "Length"
 4=MatchedLengths          "MatchedLengths"
 5=DaisyChainStubLength    "StubLength"
 6=PowerPlaneConnectStyle  "PlaneConnect"
 7=RoutingTopology         "RoutingTopology"
 8=RoutingPriority         "RoutingPriority"
 9=RoutingLayers           "RoutingLayers"
10=RoutingCornerStyle      "RoutingCorners"
11=RoutingViaStyle         "RoutingVias"
12=PowerPlaneClearance     "PlaneClearance"
13=SolderMaskExpansion     "SolderMaskExpansion"
14=PasteMaskExpansion      "PasteMaskExpansion"
15=ShortCircuit            "ShortCircuit"
16=BrokenNets              "UnRoutedNet"
17=ViasUnderSMD            "ViasUnderSMD"
18=MaximumViaCount         "MaximumViaCount"
19=MinimumAnnularRing      "MinimumAnnularRing"
20=PolygonConnectStyle     "PolygonConnect"
21=AcuteAngle              "AcuteAngle"
22=ConfinementConstraint   "RoomDefinition"
23=SMDToCorner             "SMDToCorner"
24=ComponentClearance      "ComponentClearance"
25=ComponentRotations      "ComponentOrientations"
26=PermittedLayers         "PermittedLayers"
27=NetsToIgnore            "NetsToIgnore"
28=SignalStimulus           "SignalStimulus"
29=Overshoot_FallingEdge   "OvershootFalling"
30=Overshoot_RisingEdge    "OvershootRising"
31=Undershoot_FallingEdge  "UndershootFalling"
32=Undershoot_RisingEdge   "UndershootRising"
33=MaxMinImpedance         "MaxMinImpedance"
34=SignalTopValue           "SignalTopValue"
35=SignalBaseValue          "SignalBaseValue"
36=FlightTime_RisingEdge   "FlightTimeRising"
37=FlightTime_FallingEdge  "FlightTimeFalling"
38=LayerStack              "LayerStack"
39=MaxSlope_RisingEdge     "SlopeRising"
40=MaxSlope_FallingEdge    "SlopeFalling"
41=SupplyNets              "SupplyNets"
42=MaxMinHoleSize          "HoleSize"
43=TestPointStyle           "Testpoint"
44=TestPointUsage           "TestPointUsage"
45=UnconnectedPin           "UnConnectedPin"
46=SMDToPlane               "SMDToPlane"
47=SMDNeckDown              "SMDNeckDown"
48=LayerPair                "LayerPairs"
49=FanoutControl            "FanoutControl"
50=MaxMinHeight             "Height"
51=DifferentialPairsRouting "DiffPairsRouting"
```
Note: String IDs (right column) are what appear in Rules6 parametric `RULEKIND=` fields.

**TUnit**: 0=eMetric, 1=eImperial

**TBoardSide**: 0=eBoardSide_Top, 1=eBoardSide_Bottom

### Constants (from Altium SDK)

**Coordinate system** (confirmed by SDK constants AND KiCad altium_props_utils.cpp):
- `InternalUnits = 10000` — **1 mil = 10,000 internal units**
- 1 internal unit = 0.1 microinch = 0.0001 mil = **2.54 nanometers**
- KiCad comment: "Altium's internal precision is 0.1uinch. KiCad's is 1nm."
- KiCad conversion: `value_nm = altium_internal_units * 2.54` (rounded to 10nm)
- `k1Mil = 10000`, `k1Inch = 10,000,000` (1 inch = 1000 mils)
- `kMaxCoord = 999,990,000` (99999 mils = ~2540mm), `kMinCoord = 0`
- TCoord = Integer (i32), all coordinates stored as internal units
- Binary i32 coordinate fields (X, Y, Width, etc.) are ALL in this unit system
- Parametric string values like `X=1000mil` → multiply by 10000 to get internal units

**Object ranges**:
- `FirstObjectId = eArcObject` (1), `LastObjectId = eEmbeddedBoardObject` (21)
- `AllPrimitives` = {Arc, Via, Track, Text, Fill, Pad, Component, Net, Poly, Dimension, Coordinate, Embedded, EmbeddedBoard, FromTo, Connection, **PolyRegion**, ComponentBody}
- Note: `ePolyRegionObject` in AllPrimitives is likely alias for `eRegionObject` (11)
- `WideStringObjects` = {Text, Dimension, Coordinate, Component} — these use WideString encoding

**Layer sets**:
- `SignalLayers = [eTopLayer..eBottomLayer]` (1-32)
- `cMidLayers = [eMidLayer1..eMidLayer30]` (2-31)
- `InternalPlanes = [eInternalPlane1..eInternalPlane16]` (39-54)
- `MechanicalLayers = [eMechanical1..eMechanical32]` — **32 mechanical layers** in constants, not 16
- `MinLayer = eTopLayer` (1), `MaxBoardLayer = eMultiLayer` (74), `MaxLayer = eViaHoleLayer` (82)

**Mechanical layer count discrepancy**: TLayer enum (altium-types.md) defines only eMechanical1-16,
but cLayerStrings and MechanicalLayers constant (altium-constants.md) show 32 mechanical layers.
The cLayerStrings array inserts Mechanical17-32 between Mechanical16 and DrillDrawing, implying
the TLayer enum was extended in newer SDK versions. This affects layer ID numbering above 72.

### Key References (Community Parsers)

- **altium-parser.cpp** (local copy) — KiCad altium_parser_pcb.cpp, downloaded for offline reference. Contains complete binary parsers for APAD6, AVIA6, ATRACK6, AARC6, ATEXT6, AFILL6, AREGION6, ABOARD6, ARULE6, ADIMENSION6, AMODEL, ANET6, APOLYGON6, ACLASS6, ACOMPONENT6, ACOMPONENTBODY6, AEXTENDED_PRIMITIVE_INFORMATION.
- [KiCad altium_parser_pcb.cpp](https://gitlab.com/kicad/code/kicad/-/blob/master/pcbnew/pcb_io/altium/altium_parser_pcb.cpp) — Most complete binary parser, used as primary reference
- [pyAltiumLib](https://pyaltiumlib.readthedocs.io/latest/fileformat/) — Detailed public format docs
- [AltiumSharp](https://github.com/issus/AltiumSharp) — C# .NET parser
- [python-altium](https://github.com/vadmium/python-altium) — Python format documentation
- [altium2kicad](https://github.com/thesourcerer8/altium2kicad) — Perl converter

### Ghidra Function Cross-Reference (Binary Readers/Writers)

| Function | Purpose |
|----------|---------|
| `FUN_01a05e50` | v1 section reader (reads HeaderPrim/DataPrim/HeaderTrack/DataTrack/HeaderRegion/DataRegion/BinPaths) |
| `FUN_01a0b300` | v2 section reader (same streams, larger record size) |
| `FUN_01a077c0` | v1 column-based record reader (0x28-byte records, 10 × i32) |
| `FUN_01a0d5f0` | v2 column-based record reader (0x50-byte records, 20 × i32) |
| `FUN_01a04e20` | v1 Track section writer (columns [0, 1, 6, 7, 8]) |
| `FUN_01a0a3b0` | v2 Track section writer (columns [0, 1, 8, 9, 10]) |
| `FUN_01a07590` | v1 header column name reader |
| `FUN_01a0d3b0` | v2 header column name reader |
| `FUN_01a07460` | v1 column name → index mapper (enum at PTR_DAT_019fbe28, 15 columns) |
| `FUN_01a0d260` | v2 column name → index mapper (enum at PTR_DAT_019fc600, 17 columns) |
| `FUN_0187da60` | Read int32 from binary stream |
| `FUN_01884620` | Read int64/double from binary stream |
| `FUN_0187e5e0` | Read byte from binary stream |
| `FUN_0187dae0` | Read 3-byte value from binary stream |
| `FUN_018845a0` | Read float32 from binary stream |
| `FUN_0187e680` | Read string from binary stream |

### Text Binary Record (ID=5, multi-block)

**Framing**: `u8 type(5)` + 2 subrecords with `u32 len` prefix each.

Subrecords:
1. **Main text data** (252 bytes in AD26, minimum 40 for basic, 123 for extended)
2. **Text string** (variable length, null-terminated ASCII)

**Writer chain** (Ghidra): FUN_01880680 → FUN_0185e100 → FUN_0185df50 → FUN_0185dc50 → FUN_01856e60 → FUN_01849fd0

**Subrecord 1 — Basic fields** (always present, first 40 bytes, from KiCad ATEXT6 + Ghidra FUN_01856e60):
```
Offset  Size  Type   Field                   Ghidra
  0     u8    layer                          FUN_01849fd0 (13-byte header)
  1     u8    flags1                         (tent_top=0x20, tent_bottom=0x40 etc.)
  2     u8    flags2
  3     u16   net                            (0xFFFF = none)
  5     u16   polygon                        (0xFFFF = none)
  7     u16   component                      (0xFFFF = none)
  9     u16   ref4                           (0xFFFF = none)
 11     u16   ref5                           (0xFFFF = none)
 13     i32   position_x                     VMT+0x98
 17     i32   position_y                     VMT+0xA0
 21     i32   height                         FUN_017c7b10
 25     u16   stroke_font_type               FUN_017d5b40
 27     f64   rotation                       FUN_017c96b0
 35     u8    is_mirrored                    FUN_0185e460 helper
 36     i32   stroke_width                   FUN_017c7ce0
```
If subrecord1 < 123 bytes, `fonttype = STROKE` and parsing stops here.

**Extended fields** (if subrecord1 ≥ 123 bytes, offset 40+, from KiCad + Ghidra):
```
 40     u8    is_comment                     FUN_0185e430 helper
 41     u8    is_designator                  FUN_0185e440 helper
 42     u8    user_routed (shared)            FUN_0182a700
 43     u8    font_type (0=stroke,1=TT,2=barcode)  FUN_0185e480 helper
 44     u8    is_bold                        FUN_0185e410 helper
 45     u8    is_italic                      FUN_0185e450 helper
 46     64    wchar[] font_name (UTF-16LE)   FUN_017d5b80 → lstrcpyW
110     u8    is_inverted                    FUN_0185e420 helper
111     i32   margin_border_width            FUN_017d5c70
115     u32   widestring_index               FUN_017d6090
119     i32   union_index (shared)            FUN_0182a810
123     u8    is_inverted_rect               FUN_0185e470 helper
124     i32   textbox_rect_width             FUN_017d5cd0
128     i32   textbox_rect_height            FUN_017d5ca0
132     u8    textbox_rect_justification     FUN_017d5fa0
133     i32   text_offset_width              FUN_017d5fe0
```

**Barcode/frame section** (if remaining ≥ 103, offset 137+, from KiCad):
```
137     i32   unk_vec_x
141     i32   unk_vec_y
145     i32   barcode_margin_x
149     i32   barcode_margin_y
153     i32   unk32
157     u8    barcode_type
158     1     (skip)
159     u8    barcode_inverted
160     u8    barcode_font_type
161     64    wchar[] barcode_fontname (UTF-16LE)
225     5     (skip)
230     u8    is_frame
231     u8    is_offset_border
232     8     (skip)
```

**Final section** (if remaining ≥ 115):
```
240     u8    is_justification_valid
```

**Writer trailing fields** (Ghidra FUN_0185e100 + FUN_0185e090, offsets 226-251):
```
226     i32   layer_enum_index               FUN_01829f00→FUN_00dd7410
232     i32   = 0x80000000 (sentinel)
236     i32   = 0x80000000 (sentinel)
240     i32   unknown (0 or 1)               FUN_017d5780
244     i32   unknown                        FUN_017c7ae0
248     i32   unknown                        FUN_017c7ae0 (2nd call)
```

Note: The "unknown_flag" (offset 42) and "unknown_value" (offset 119) use the same shared getter functions FUN_0182a700 and FUN_0182a810 as Track/Arc/Fill — confirming these are inherited from a common base class.

**Subrecord 2 — Text string**: Raw ASCII text (e.g. "HS1", "A10", "40"). If `widestring_index > 0`, the actual display text comes from the WideStrings6 table instead. KiCad normalizes `\r\n` → `\n`.

**Verified**: 7,260 text records from RFSoC_AMC.PcbDoc. Subrecord 1 = 252 bytes, subrecord 2 = 2–30+ bytes.

### Component Record (ID=9, parametric only)

Components6/Data uses pure **parametric format** (`u32 len + |KEY=VALUE|` ASCII text).
No binary record framing — the entire record is a pipe-delimited parameter string.

Key fields: `|LAYER=|X=|Y=|PATTERN=|NAMEON=|COMMENTON=|ROTATION=|HEIGHT=|DESIGNATOR=|COMMENT=|`

### Connection Binary Record (ID=7)

**Framing**: `u32 len (always 43)` + 43 bytes data (no type byte prefix)

Fixed 47-byte total per record (u32 len + data).

From Ghidra decompilation (FUN_01857730 + FUN_0185de70):

```
Offset  Size  Type   Field                      Ghidra
  0     13    -      Common Header               FUN_01849fd0
 13     i32   from_x                             FUN_017c7810
 17     i32   from_y                             FUN_017c7870
 21     i32   to_x                               FUN_017c7840
 25     i32   to_y                               FUN_017c78a0
 29     u8    from_layer                          FUN_017c63b0→FUN_00dd7560
 30     u8    to_layer                            FUN_017c6dc0→FUN_00dd7560
 31     i32   connection_layer_enum               FUN_01829f00→FUN_00dd7410
 35     i32   from_layer_enum                     FUN_017c63b0→FUN_00dd7410
 39     i32   to_layer_enum                       FUN_017c6dc0→FUN_00dd7410
```

Total: 43 bytes (0x2B). Writer: FUN_019259a0 → FUN_0185de70 → FUN_01857730.

**Key finding**: The "tail bytes" (offsets 29-42) are NOT pad references — they are layer information. The `from_layer` and `to_layer` are the Altium layer IDs for the start and end pads. The three `*_layer_enum` fields are extended layer enum representations of the same layers.

SDK confirms: `IPCB_Connection` has `Layer1` (from), `Layer2` (to), and `Mode` properties. No pad index/reference properties exist on the Connection interface — connections identify endpoints by coordinates and layers only.

**Note**: Connections6 uses `u32 len + data` framing (NOT `u8 type + u32 len + data`). This is the only primitive data stream that uses parametric-style framing with a u32 length prefix instead of the standard type+length prefix.

**Verified**: 117 connection records from RFSoC_AMC.PcbDoc, all 43 bytes.

### Polygon Record (ID=10, parametric only)

Polygons6/Data uses pure **parametric format** with inline vertex data.
Key fields: `|POLYGONTYPE=|GRIDSIZE=|TRACKWIDTH=|HATCHSTYLE=|KIND0=|VX0=|VY0=|CX0=|CY0=|SA0=|EA0=|R0=|...`

### Region Binary Record (ID=11, hybrid binary+parametric)

**Framing**: `u8 type(11)` + `u32 total_len` + data

**Binary header** (18 bytes):
```
Offset  Size  Field
  0     u8    layer           # Altium layer ID
  1     u8    flags1          # bit 0x04 = NOT locked, bit 0x10 = teardrop
  2     u8    flags2          # 2 = keepout
  3     u16   net             # Net index (0xFFFF = none)
  5     u16   polygon         # Parent polygon index (0xFFFF = none)
  7     u16   component       # Component index (0xFFFF = none)
  9     5     (skip)          # Padding/reserved (often 0xFF 0xFF 0xFF 0xFF 0x00)
 14     u16   holecount       # Number of holes (cutouts) in the region
 16     2     (skip)          # Padding
```

**Parametric properties** (variable length):
```
 18     u32   prop_len        # Length of property text (including null terminator)
 22     str   properties      # Null-terminated pipe-delimited ASCII
```

Key properties: `V7_LAYER=<name>|NAME=<n>|KIND=<k>|SUBPOLYINDEX=<i>|UNIONINDEX=<u>|ARCRESOLUTION=<r>|ISSHAPEBASED=<b>|CAVITYHEIGHT=<h>`

KIND values: 0=copper region, 1=board cutout, 2=polygon cutout, 3=dashed outline, 4=cavity definition

**Outline vertices** (immediately after properties):
```
         u32   num_outline_vertices
         [num_outline_vertices × vertex]:
           f64   x             # Altium internal units (÷10000 = mils)
           f64   y             # Altium internal units (note: KiCad negates Y)
```

**Hole vertices** (holecount × hole):
```
         u32   num_hole_vertices
         [num_hole_vertices × vertex]:
           f64   x
           f64   y
```

Each vertex is 16 bytes (f64 x + f64 y). Total vertex data = Σ(4 + nv×16) for outline + each hole.

**Verified**: 46,673 regions from RFSoC_AMC.PcbDoc, record sizes 195–220,632 bytes. Multiple outlines per region (1 outer boundary + 0–317 holes). All records parse with 0 leftover bytes.

**Extended vertex format** (used by some newer versions):
When `aExtendedVertices` is true, vertices use a richer format:
```
  u8    isRound          # 0 = line segment, nonzero = arc
  i32   x, y             # KiCad units (not f64)
  i32   cx, cy           # Arc center
  i32   radius           # Arc radius
  f64   angle1, angle2   # Arc start/end angles
```
This extended format adds 1 extra vertex (num_outline_vertices++) and is primarily for shape-based regions.

### ComponentBody Binary Record (ID=12, hybrid binary+parametric)

**Identical framing and header to Region** — same 18-byte binary header, same parametric properties block, same outline/hole vertex format.

**Framing**: `u8 type(12)` + `u32 total_len` + data

Binary header: same as Region (layer, flags1, flags2, net, polygon, component, skip5, holecount, skip2).

**Additional parametric properties** (3D-specific):
- `STANDOFFHEIGHT=<mils>` — Height above board
- `OVERALLHEIGHT=<mils>` — Total body height
- `BODYPROJECTION=<0|1>` — Projection type
- `BODYCOLOR3D=<int>` — RGB color as integer
- `BODYOPACITY3D=<float>` — Opacity (0.0–1.0)
- `IDENTIFIER=<comma-sep-codepoints>` — Unicode identifier (WideStrings encoding)
- `MODELID=<guid>` — Reference to 3D model in Models stream
- `MODELTYPE=<int>` — 3D model type

Vertex data: same as Region (outline + holes with f64 x,y pairs).

**Verified**: 2,299 ComponentBody records from RFSoC_AMC.PcbDoc, sizes 806–941 bytes.

### ShapeBased Variants (ShapeBasedRegions6, ShapeBasedComponentBodies6)

These streams contain the **same records** as their non-ShapeBased counterparts but with extended vertex format for the **outline only**:

**Extended vertex format** (37 bytes, outline only):
```
  u8    isRound          # 0 = line segment, nonzero = arc
  i32   x                # Altium internal units (÷10000 = mils)
  i32   y
  i32   cx               # Arc center X (0 for line segments)
  i32   cy               # Arc center Y
  i32   radius           # Arc radius
  f64   angle1           # Arc start angle (degrees)
  f64   angle2           # Arc end angle (degrees)
```

Key differences from standard Region/ComponentBody:
- Outline vertex count field = N, but **N+1 vertices** are stored (closing vertex)
- Outline uses 37-byte extended format; **holes still use standard 16-byte f64 format**
- Same record count as non-ShapeBased counterpart (2,299 CB, 46,673 Region in test file)
- ShapeBased records are larger due to extended vertex data

### Primitive Object ID Values and Framing Summary

| ID | Type | Stream | Framing | Size (AD26) | Status |
|----|------|--------|---------|-------------|--------|
| 1 | Arc | Arcs6 | `u8 type + u32 len + data` | 60 bytes | ✓ Complete |
| 2 | Pad | Pads6 | `u8 type + 6×(u32 len + data)` | ~912 bytes | ✓ Complete |
| 3 | Via | Vias6 | `u8 type + u32 len + data` | 330 bytes | ✓ Complete |
| 4 | Track | Tracks6 | `u8 type + u32 len + data` | 49 bytes | ✓ Complete |
| 5 | Text | Texts6 | `u8 type + 2×(u32 len + data)` | 252+N bytes | ✓ Complete |
| 6 | Fill | Fills6 | `u8 type + u32 len + data` | 50 bytes | ✓ Complete |
| 7 | Connection | Connections6 | `u32 len + data` (no type) | 47 bytes | ✓ Complete |
| 8 | Net | Nets6 | `u32 len + text` (parametric) | variable | ✓ Parametric |
| 9 | Component | Components6 | `u32 len + text` (parametric) | variable | ✓ Parametric |
| 10 | Polygon | Polygons6 | `u32 len + text` (parametric) | variable | ✓ Parametric |
| 11 | Region | Regions6 | `u8 type + u32 len + data` | variable | ✓ Complete |
| 12 | ComponentBody | ComponentBodies6 | `u8 type + u32 len + data` | variable | ✓ Complete |
| 11* | ShapeBasedRegion | ShapeBasedRegions6 | `u8 type + u32 len + data` | variable | ✓ Complete |
| 12* | ShapeBasedCB | ShapeBasedCB6 | `u8 type + u32 len + data` | variable | ✓ Complete |

### PcbLib File Format (Library-Specific)

PcbLib files use a different CFB layout than PcbDoc. Primitives are organized **per footprint** rather than by type.

```
<file.pcblib>/
├── FileHeader                    # Version string, unique ID
├── FileVersionInfo/Header+Data   # Version parameters
├── Library/
│   ├── Header + Data             # Library TOC (footprint name list)
│   ├── EmbeddedFonts             # Optional
│   ├── ComponentParamsTOC/       # Per-component metadata
│   ├── LayerKindMapping/         # Layer mapping data
│   ├── Models/                   # Embedded 3D models (zlib STEP)
│   ├── ModelsNoEmbed/            # External model references
│   ├── PadViaLibrary/            # Shared pad/via definitions
│   └── Textures/
├── <Footprint1>/
│   ├── Header                    # u32: primitive count
│   ├── Data                      # Binary primitive records
│   ├── Parameters                # Pipe-delimited parameters
│   ├── WideStrings               # UTF-16 encoded strings
│   ├── PrimitiveGuids            # GUID mapping
│   └── UniqueIDPrimitiveInformation
├── <Footprint2>/
│   └── ...
└── SectionKeys                   # Name → storage key mapping
```

**Library/Data TOC format**:
1. ParameterBlock: Library header (`|HEADER=...` metadata)
2. `u32`: Number of footprints
3. Array of PCB String Blocks (each: `u32` length + ASCII string)

**SectionKeys**: Maps component reference names to OLE storage keys (needed because OLE names are limited to 31 chars).

**ComponentParamsTOC/Data**: Per-component metadata with `|NAME=|DESCRIPTION=|HEIGHT=|PADCOUNT=|` etc.

**3D Models** (Library/Models/): Indexed in Data stream with `ID` parameter; actual STEP files in numbered sub-streams ("0", "1", ...) as zlib-compressed ASCII.

### PcbLib vs PcbDoc Differences

| Aspect | PcbLib | PcbDoc |
|--------|--------|--------|
| Purpose | Library of footprints | Single board design |
| `Library/` storage | Yes (TOC, Models) | No |
| Primitive location | Per-footprint `Data` stream | By-type sections (`Tracks6/`, `Arcs6/` etc.) |
| Net/Connection data | Not present | `Nets6/`, `Connections6/`, `Classes6/` |
| Board data | Not present | `Board6/Data` |
| Rules | Not present | `Rules6/Data` |

### PcbDoc Stream Listing (verified from RFSoC_AMC.PcbDoc)

Each stream has `Header` (u32 record count) + `Data` sub-streams:

**Primitive data streams** (binary records with u8 type + u32 len framing):
- `Arcs6` (8364 records, 60 bytes each)
- `ComponentBodies6`
- `Components6`
- `Connections6`
- `Fills6` (357 records, 50 bytes each)
- `Pads6` (10823 records, ~912 bytes each multi-block)
- `Regions6`
- `ShapeBasedComponentBodies6`
- `ShapeBasedRegions6`
- `Texts6`
- `Tracks6` (106874 records, 49 bytes each)
- `Vias6` (7875 records, 330 bytes each)

**Metadata streams** (parametric `|KEY=VALUE|` format):
- `Board6` — Board parameters (origin, grid, layer count)
- `BoardRegions`
- `Classes6` — Net classes
- `DifferentialPairs6`
- `Nets6` — Net definitions
- `Polygons6` — Polygon pour definitions
- `Rules6` — Design rules
- `SignalClasses`
- `SmartUnions`
- `UnionNames`

**Configuration streams**:
- `FileHeader` / `FileHeaderSix` — Version string
- `FileVersionInfo` — Version parameters
- `Advanced Placer Options6`
- `Design Rule Checker Options6`
- `Pin Swap Options6`
- `PinPairsSection`
- `ConstraintManager`
- `LayerKindMapping`

**Auxiliary streams** (cross-referencing system):
- `PrimitiveParameters` — Component parameter storage (see below)
- `ExtendedPrimitiveInformation` — Per-primitive override parameters (mask expansions etc.)
- `UniqueIDPrimitiveInformation` — Maps PRIMITIVEINDEX+PRIMITIVEOBJECTID → UNIQUEID
- `WideStrings6` — Indexed UTF-16 string table for Text records
- `EmbeddedFonts6` — Embedded font data
- `Models` — 3D models (Header/Data + numbered streams 0,1,2...)
- `ModelsNoEmbed` — External model references
- `PadViaLibrary` / `PadViaLibraryCache` / `PadViaLibraryLinks`
- `FromTos6` — From-To connections
- `Textures`
- `WaivedViolations`
- `Texts` (old-format text, separate from `Texts6`)

### Cross-Referencing System

PcbDoc uses several auxiliary streams to cross-reference primitives:

**UniqueIDPrimitiveInformation/Data** (parametric records):
Maps each primitive to a unique 8-character ID.
```
|PRIMITIVEINDEX=0|PRIMITIVEOBJECTID=Pad|UNIQUEID=IHDRDDWW
|PRIMITIVEINDEX=1|PRIMITIVEOBJECTID=Pad|UNIQUEID=DPEOHXYE
```
- `PRIMITIVEINDEX`: Zero-based index within the primitive's type stream (Pads6, Tracks6, etc.)
- `PRIMITIVEOBJECTID`: Type name (Pad, Track, Arc, Via, Fill, Text, Component, Region, etc.)
- `UNIQUEID`: 8-character unique identifier

**PrimitiveParameters/Data** (parametric records):
Stores component parameters (BOM properties). Structured as groups:
1. Header record: `|PRIMITIVEID=<uid>|ID=Component#N|APPURTENANCE=System|VARIANTGUID=System|COUNT=0`
2. Count record: `|PRIMITIVEID=<uid>|ID=Component#N|VARIANTGUID=|COUNT=21`
3. N parameter records: `|NAME=<key>|VALUE=<val>|ISIMPORTED=TRUE`

The `PRIMITIVEID` matches the component's `UNIQUEID` field in Components6.

**ExtendedPrimitiveInformation/Data** (parametric records):
Stores per-primitive property overrides (sparse — only primitives with non-default settings):
```
|PRIMITIVEINDEX=90|PRIMITIVEOBJECTID=Fill|TYPE=Mask|SOLDERMASKEXPANSIONMODE=Rule|PASTEMASKEXPANSIONMODE=None
```

**WideStrings6/Data** (indexed string table):
UTF-16 encoded strings referenced by Text records' `widestring_index` field. Record N in this stream corresponds to `widestring_index=N`. Format: `u32 len + data` (data is UTF-16LE encoded text).

### WideStrings Encoding

The `WideStrings` stream stores Unicode text as comma-separated UTF-16 code points:
```
|TEXT=72,101,108,108,111    => "Hello"
```

### Next Steps for PCB Format

**Completed**:
- Binary record layouts verified against KiCad parser and real PcbDoc files:
  - Track (49 bytes): full field map with common header, coordinates, width, subpolyindex, keepout
  - Arc (60 bytes): full field map with center, radius, angles, width
  - Via (330 bytes): complete field map including extended thermal, mask, tolerance fields
  - Fill (50 bytes): full field map with corners, rotation, keepout
  - Pad (~912 bytes, 6 subrecords): multi-block framing documented
  - Text (252+N bytes, 2 subrecords): complete field map including font, barcode, frame fields
  - Region (variable, hybrid): complete format — 18-byte header + parametric properties + outline/hole vertices (f64 x,y pairs)
  - ComponentBody (variable, hybrid): same as Region with 3D-specific parametric properties
  - ShapeBasedRegions6/ShapeBasedComponentBodies6: extended 37-byte vertex format for outlines
  - Component (parametric only): pure pipe-delimited ASCII
  - Polygon (parametric only): pure pipe-delimited ASCII with inline vertex data
- Common 13-byte primitive header documented (layer, flags, net, polygon, component)
- PcbDoc stream structure verified from real files (46,673 regions, 106,874 tracks, etc.)
- PcbLib format documented (per-footprint storage with Library TOC)
- Record framing confirmed: `u8 type + u32 len + data` (Pad/Text use multi-block)
- Confirmed PCB coordinate system: 10,000 internal units = 1 mil
- Region vertex format: f64 x,y pairs (16 bytes/vertex), multiple outlines per region
- Cross-referenced all formats against KiCad altium_parser_pcb.cpp source

### Board6 Layer Stack (from KiCad ABOARD6)

Board6/Data is parametric (`|KEY=VALUE|` format). Key fields:
```
SHEETX, SHEETY              → Board sheet position
SHEETWIDTH, SHEETHEIGHT     → Board sheet size
LAYERSETSCOUNT              → Number of layers - 1

Per layer (i = 1, 2, 3, ...):
LAYER{i}NAME                → Layer name (e.g. "Top Layer", "Mid-Layer 1")
LAYER{i}NEXT                → Next layer ID in stack
LAYER{i}PREV                → Previous layer ID in stack
LAYER{i}COPTHICK            → Copper thickness (mil units, e.g. "1.4mil")
LAYER{i}DIELCONST           → Dielectric constant
LAYER{i}DIELHEIGHT          → Dielectric thickness (e.g. "60mil")
LAYER{i}DIELMATERIAL        → Dielectric material name (e.g. "FR-4")
```

Followed by board outline vertices: `KIND0`, `VX0`, `VY0`, `CX0`, `CY0`, `SA0`, `EA0`, `R0`, ...

### Rules6 (from KiCad ARULE6)

Rules6/Data is parametric with 2-byte skip prefix. Key fields:
```
NAME, PRIORITY, SCOPE1EXPRESSION, SCOPE2EXPRESSION, RULEKIND
```

Rule kinds and their specific fields:
| RULEKIND | Fields |
|----------|--------|
| `Clearance` | GAP |
| `Width` | MINLIMIT, MAXLIMIT, PREFERREDWIDTH |
| `RoutingVias` | WIDTH, MINWIDTH, MAXWIDTH, HOLEWIDTH, MINHOLEWIDTH, MAXHOLEWIDTH |
| `HoleSize` | MINLIMIT, MAXLIMIT |
| `HoleToHoleClearance` | GAP |
| `SolderMaskExpansion` | EXPANSION |
| `PasteMaskExpansion` | EXPANSION |
| `PlaneClearance` | CLEARANCE |
| `PolygonConnect` | AIRGAPWIDTH, RELIEFCONDUCTORWIDTH, RELIEFENTRIES, CONNECTSTYLE (Direct/Relief/NoConnect) |
| `DiffPairsRouting` | (no specific fields parsed by KiCad) |
| `Height` | (no specific fields parsed by KiCad) |

### Dimensions6 (from KiCad ADIMENSION6)

Dimensions6/Data is parametric with 2-byte skip prefix. Key fields:
```
LAYER, DIMENSIONKIND, TEXTFORMAT, TEXTPREFIX, TEXTSUFFIX, HEIGHT, ANGLE,
LINEWIDTH, TEXTHEIGHT, TEXTLINEWIDTH, TEXTPRECISION, ITALIC, TEXTGAP,
ARROWSIZE, TEXTPOSITION, X1, Y1, TEXTDIMENSIONUNIT (Inches/Mils/Millimeters/Centimeters)
REFERENCES_COUNT, REFERENCE{i}POINTX, REFERENCE{i}POINTY
TEXT{i}X, TEXT{i}Y
```

### ExtendedPrimitiveInformation (from KiCad)

Parametric records that provide per-primitive overrides:
```
PRIMITIVEINDEX, PRIMITIVEOBJECTID (Arc/Pad/Via/Track/Text/Fill/Region/Model)
TYPE=Mask
PASTEMASKEXPANSIONMODE (None/Rule/Manual), PASTEMASKEXPANSION_MANUAL
SOLDERMASKEXPANSIONMODE (None/Rule/Manual), SOLDERMASKEXPANSION_MANUAL
```

### Models (from KiCad AMODEL)

Models/Data is parametric. Key fields:
```
NAME          → Model filename
ID            → Model GUID
EMBED         → Boolean, true if embedded
ROTX, ROTY, ROTZ → 3D rotation
DZ            → Z offset
CHECKSUM      → Model checksum
```

### ComponentBody6 Parsing (from KiCad ACOMPONENTBODY6)

Binary header: `u8 type(12)` + subrecord with 7 skip + u16 component + 9 skip, then parametric:
```
MODEL.NAME, MODELID, MODEL.EMBED
MODEL.2D.X, MODEL.2D.Y, MODEL.3D.DZ
MODEL.3D.ROTX, MODEL.3D.ROTY, MODEL.3D.ROTZ
MODEL.2D.ROTATION, BODYOPACITY3D, BODYPROJECTION
```

**Still needed**:
- Test with multiple PcbDoc files to find version-dependent field differences
- Begin Rust implementation of PCB parser

**Resolved unknowns** (this session):
- FUN_0182a700 = **UserRouted** (bool) — confirmed via SDK `GetState_UserRouted()` + altium2kicad `USERROUTED`
- FUN_0182a810 = **UnionIndex** (i32) — confirmed via altium2kicad `UNIONINDEX` at same offset
- Track has 1 extra byte at offset 40 (FUN_017bef10→FUN_0185e1c0) not present in Arc/Fill
- **Connection tail bytes decoded**: offsets 29-42 are layer data (from_layer u8, to_layer u8, 3×i32 layer_enums), NOT pad references. Connections identify endpoints by coordinates + layers only, with no pad index references.

### Ghidra Decompilation Findings (Altium.PCB.BinaryLoader.dll)

**Running Ghidra headless scripts**: The daemon bridge doesn't work, but `analyzeHeadless.bat` with Java scripts works reliably.
```bash
"C:/Users/dev/ghidra/support/analyzeHeadless.bat" "C:/Users/dev/git/ghidra-altium" ghidra-altium \
  -process "Altium.PCB.BinaryLoader.dll" -noanalysis \
  -scriptPath "C:/Users/dev/git/ghidra-altium" \
  -postScript DecompileFunc.java <address_hex>
```

**Stream initialization** (FUN_01847b90): Lists ALL PcbDoc CFB stream names registered as section handlers:
```
Board6, Advanced Placer Options6, Advanced Router Options6, Design Rule Checker Options6,
Pin Swap Options6, Classes6, Nets6, Components6, Polygons6, Dimensions6, Coordinates6,
EmbeddedBoards6, Connections6, Rules6, NewRules6, FromTos6, DifferentialPairs6, Embeddeds6,
Arcs6, Pads6, Vias6, Tracks6, Texts6, Fills6, ShapeBasedRegions6, Regions6,
ShapeBasedComponentBodies6, ComponentBodies6, WideStrings6, EmbeddedFonts6,
SplitPlaneRegions6, UnionNames, UnionRelations, SmartUnions
```

**Section class hierarchy**:
- `TPrimitivesSection` (base class at 0x0185ed90 area) — generic binary record reader/writer
  - `TTracksSection` — handles Tracks6 stream
  - Other per-type sections for each stream above
- Common reader: FUN_0185f500 — reads `u32 count` header, loops N times calling VMT+0xC0 per record
- Per-record handler (VMT+0xC0 → VMT+0xA0): dispatches to type-specific writers

**Binary record writer dispatch** (FUN_018843e0): Dispatches by primitive object type:
| Type | Function | Primitive |
|------|----------|-----------|
| 1 | FUN_01883b40 | **Arc** |
| 2 | FUN_0187eb60 | **Pad** (writes 0xac=172 bytes common + extended) |
| 3 | FUN_0187fa70 | **Via** |
| 4 | FUN_0187f9b0 | **Track** |
| 5 | FUN_01880680 | **Text** |
| 6 | FUN_01880e60 | **Fill** |

**Pad writer** (FUN_0187eb60): Architecture matches KiCad's 6-subrecord reading:
- 4 string subrecords (name, unknown, unknown, unknown) via FUN_0187e7e0
- Core pad serializer FUN_01858be0 → FUN_0184ad40 → FUN_01849fd0 (13-byte header)
- Subrecord 5: 0xAC (172) bytes minimum; if extended: 0xCA (202) or pad_count*0x1E + 0xCE
- Subrecord 6: shape/stack data — 0x274 (628) bytes base, plus optional entry_count × 0xF (15) bytes
- Conditional flags at 0x0F and 0x10 control which extended sections are present

**3D Routing columnar format** (NOT the main PCB format):
Two variant classes use a column-based internal format with HeaderPrim/DataPrim/HeaderTrack/DataTrack/HeaderRegion/DataRegion/BinPaths streams:
- `T3DRoutingUVFSection` — v1 format, 0x28-byte (40) records per index entry
- `T3DRoutingXYZSection` — v2 format, 0x50-byte (80) records per index entry

Column reader data types (shared by both):
| Function | Reads | Size |
|----------|-------|------|
| FUN_0187da60 | read_i32 | 4 bytes |
| FUN_01884620 | read_i64 | 8 bytes |
| FUN_018845a0 | read_i32 (variant) | 4 bytes |
| FUN_0187dae0 | read_u16 | 2 bytes |
| FUN_0187e5e0 | read_u8 | 1 byte |
| FUN_0187e680 | read_string | variable |

Column enum names (Delphi RTTI at 0x019fbe28): `epIndexForSave`, `epObjectId`, `epFaceU_i`, `epFaceV_i`, `epFaceRot_d`, `epFaceIdx_i` — these are 3D routing UV/face parameters, not main PCB fields.

**Key insight**: The HeaderPrim/DataPrim/etc. columnar streams are used ONLY by the 3D routing overlay section, not the main PCB primitive data. Main PCB data uses per-type binary streams (Tracks6/Data, Arcs6/Data, etc.) with the standard `u8 type + u32 len + data` framing.

### CFB Stream Names

From `FileFormatConsts.cs`:

| Stream | Purpose |
|--------|---------|
| `FileHeader` | Library/document metadata and component index |
| `Data` | Component record data (per component storage) |
| `Storage` | Embedded image/object storage |
| `SectionKeys` | Component name escaping map |
| `Redirection` | Alias → primary component mapping |
| `Additional` | Additional component data |
| `LibAdditional` | Library-level additional data |
| `PinFrac` | Pin fractional coordinates |
| `PinDesc` | Pin long descriptions (>254 chars) |
| `PinMiscData` | Pin swap ID pairs |
| `PinTextData` | Pin custom text display settings |
| `PinWideText` | Pin Unicode text |
| `PinSymbolLineWidth` | Pin symbol line widths |
| `PinPackageLength` | Pin package lengths |
| `PinPropagationDelay` | Pin propagation delays |
| `PinFunctionData` | Pin function definitions |
| `Files` | File streams |
| `ReuseBlocks` | Reuse block data |
| `ObjectDefinitions` | Object definitions |
| `HarnessConnectionPointConnector` | Harness connectors |
| `HarnessComponentCrimps` | Harness crimps |
| `HarnessAssociatedParts` | Harness associated parts |
| `ReuseBlockInfos` | Reuse block metadata |

### File Format Header Strings

| Format | Header String |
|--------|---------------|
| SchLib Binary V5 | `Protel for Windows - Schematic Library Editor Binary File Version 5.0` |
| SchLib ASCII V5 | `Protel for Windows - Schematic Library Editor Ascii File Version 5.0` |
| SchLib JSON V5 | `Altium Designer - Schematic Library Editor Json File Version 5.0` |
| SchDoc Binary V5 | `Protel for Windows - Schematic Capture Binary File Version 5.0` |
| SchDoc ASCII V5 | `Protel for Windows - Schematic Capture Ascii File Version 5.0` |
| SchLib Binary V4 | `Protel for Windows - Schematic Library Editor Binary File Version 1.2 - 2.0` |
| SchDoc Binary V4 | `Protel for Windows - Schematic Capture Binary File Version 1.2 - 2.0` |

### Coordinate System Detail

Internal coordinates: **100,000 units per mil** (DXP2004 SP2 format).

```csharp
// Binary mode: int16 → internal
Import_Coord: value = short_value * 100000
Export_Coord: short_value = internal / 100000

// PinFrac extends precision:
final_coord = (short_value * 100000) + fractional_part
// where fractional_part is int32 from PinFrac stream (0-99999 range)
```

**Note**: altium-cli uses 10,000 units/mil — this is WRONG. Must be 100,000.
