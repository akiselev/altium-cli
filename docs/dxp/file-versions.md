# Altium File Format Versions

Altium Designer uses two separate version systems: one for PCB-family files and one
for schematic-family files. These version numbers are stored in the file header and
determine which parser/serializer code path is used.

## PCB Family: `TAdvPCBFileFormatVersion`

**Source**: `Altium.Edp.Interfaces/RT_PCB/TAdvPCBFileFormatVersion.cs`

The PCB version enum encodes **three dimensions in a single byte**: the format
generation (V3-V6), the file type (document vs library vs ASCII), and the product
variant (AD vs CircuitStudio vs CircuitMaker vs PCBWorks).

| Value | C# Enum Name | File Type | Format | Product |
|-------|-------------------------------|-----------|--------|---------|
| 0 | `ePCBFileFormatNone` | — | — | — |
| 1 | `eAdvPCBFormat_Binary_V3` | PcbDoc | Binary | All |
| 2 | `eAdvPCBFormat_Library_V3` | PcbLib | Binary | All |
| 3 | `eAdvPCBFormat_ASCII_V3` | PcbDoc | ASCII | All |
| 4 | `eAdvPCBFormat_Binary_V4` | PcbDoc | Binary | All |
| 5 | `eAdvPCBFormat_Library_V4` | PcbLib | Binary | All |
| 6 | `eAdvPCBFormat_ASCII_V4` | PcbDoc | ASCII | All |
| 7 | `eAdvPCBFormat_Binary_V5` | PcbDoc | Binary | All |
| 8 | `eAdvPCBFormat_Library_V5` | PcbLib | Binary | All |
| 9 | `eAdvPCBFormat_ASCII_V5` | PcbDoc | ASCII | All |
| 10 | `eAdvPCBFormat_Binary_V6` | PcbDoc | Binary | Altium Designer |
| 11 | `eAdvPCBFormat_Library_V6` | PcbLib | Binary | All |
| 12 | `eAdvPCBFormat_ASCII_V6` | PcbDoc | ASCII | All |
| 13 | `eAdvPCBFormat_Binary_V6_CS` | PcbDoc | Binary | CircuitStudio |
| 14 | `eAdvPCBFormat_Binary_V6_CM` | PcbDoc | Binary | CircuitMaker |
| 15 | `eAdvPCBFormat_Binary_V6_PCBWorks` | PcbDoc | Binary | PCBWorks |
| 16 | `eAdvPCBFormat_PadViaLibrary_V6` | PvLib | Binary | All |

### Key observations

- **Binary** = `.PcbDoc` files (OLE compound document containing binary records)
- **Library** = `.PcbLib` files (OLE compound document with per-component substorages)
- **ASCII** = `.PcbDoc` exported as ASCII text (no ASCII library variant exists)
- **V6 product variants** (CS/CM/PCBWorks) are the same binary format with different
  header strings, allowing Altium to identify which product created the file
- **PadViaLibrary** = `.PvLib` files (dedicated pad/via template library, V6 only)
- There is no ASCII variant of PcbLib at any version

### Version groups

From `xPCBTypes/Consts.cs`:

```
cAdvPCBFormats_Version3 = [Binary_V3, Library_V3, ASCII_V3]        // 3 entries
cAdvPCBFormats_Version4 = [Binary_V4, Library_V4, ASCII_V4]        // 3 entries
cAdvPCBFormats_Version5 = [Binary_V5, Library_V5, ASCII_V5]        // 3 entries
cAdvPCBFormats_Version6 = [Binary_V6, Library_V6, ASCII_V6,        // 6 entries
                           Binary_V6_CS, Binary_V6_CM,
                           Binary_V6_PCBWorks]
```

Note: `PadViaLibrary_V6` is NOT included in `cAdvPCBFormats_Version6`.

### Current format constants

```csharp
cAdvPCBFormat_ASCII_Current = eAdvPCBFormat_ASCII_V5  // ASCII still uses V5!

kCurrentPCBFormat_AD       = "PCB 6.0 Binary File"
kCurrentPCBFormat_CS       = "CircuitStudio PCB 6.0 Binary File"
kCurrentPCBFormat_CM       = "CircuitMaker PCB 6.0 Binary File"
kCurrentPCBFormat_PCBWorks = "PCBWorks PCB 6.0 Binary File"
kCurrentPCBLibFormat       = "PCB 6.0 Library File"
```

These header strings are stored in the `FileHeader` stream of the OLE compound
document and are used to identify both the format version and the originating product.

### Our Rust mapping

Our `PcbFileFormatVersion` enum in `altium-format-types/src/pcb.rs` maps 1:1 to the
C# enum:

| Our Name | C# Name |
|----------------------|--------------------------------|
| `None` | `ePCBFileFormatNone` |
| `BinaryV3` | `eAdvPCBFormat_Binary_V3` |
| `LibraryV3` | `eAdvPCBFormat_Library_V3` |
| `AsciiV3` | `eAdvPCBFormat_ASCII_V3` |
| `BinaryV4` | `eAdvPCBFormat_Binary_V4` |
| `LibraryV4` | `eAdvPCBFormat_Library_V4` |
| `AsciiV4` | `eAdvPCBFormat_ASCII_V4` |
| `BinaryV5` | `eAdvPCBFormat_Binary_V5` |
| `LibraryV5` | `eAdvPCBFormat_Library_V5` |
| `AsciiV5` | `eAdvPCBFormat_ASCII_V5` |
| `BinaryV6` | `eAdvPCBFormat_Binary_V6` |
| `LibraryV6` | `eAdvPCBFormat_Library_V6` |
| `AsciiV6` | `eAdvPCBFormat_ASCII_V6` |
| `BinaryV6CS` | `eAdvPCBFormat_Binary_V6_CS` |
| `BinaryV6CM` | `eAdvPCBFormat_Binary_V6_CM` |
| `BinaryV6PCBWorks` | `eAdvPCBFormat_Binary_V6_PCBWorks` |
| `PadViaLibraryV6` | `eAdvPCBFormat_PadViaLibrary_V6` |


## Schematic Family: `TFileFormatVersion`

**Source**: `Altium.Edp.Interfaces/RT_SchDataModel/TFileFormatVersion.cs`

Schematic versioning is much simpler -- only two generations exist:

| Value | C# Enum Name | Description |
|-------|--------------|-------------|
| 0 | `ffv4` | Version 4.0 (legacy Protel) |
| 1 | `ffv5` | Version 5.0 (modern Altium Designer) |

Unlike the PCB enum, the schematic version does **not** encode file type or
serialization format. Those are determined separately via `TFileKind` and
`TSerializerType`.

### File kinds (`TFileKind`)

**Source**: `Altium.Edp.Interfaces/Rt_Schematic/TFileKind.cs`

| Enum Value | File Type | Serializer | Version |
|----------------------------------------------|-----------|------------|---------|
| `eProtelBinarySchematicFile_v40` | SchDoc | Binary | v4 |
| `eProtelBinarySchematicFile_v50` | SchDoc | Parametric | v5 |
| `eProtelAsciiSchematicFile_v40` | SchDoc | ASCII | v4 |
| `eProtelAsciiSchematicFile_v50` | SchDoc | ASCII | v5 |
| `eProtelBinarySchematicLibraryFile_v40` | SchLib | Binary | v4 |
| `eProtelBinarySchematicLibraryFile_v50` | SchLib | Parametric | v5 |
| `eCircuitStudioBinarySchematicFile_v50` | SchDoc | Parametric | v5 |
| `eAltiumJsonSchematicFile_v50` | SchDoc | JSON | v5 |
| `eAltiumJsonSchematicLibraryFile_v50` | SchLib | JSON | v5 |

### File header strings

**Source**: `Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs`

**SchDoc headers**:
| Header String | Version | Format |
|----------------------------------------------------------------------|---------|--------|
| `Protel for Windows - Schematic Capture Binary File Version 1.2 - 2.0` | v4 | Binary |
| `Protel for Windows - Schematic Capture Binary File Version 5.0` | v5 | Binary |
| `Protel for Windows - Schematic Capture Ascii File Version 5.0` | v5 | ASCII |
| `Altium Designer - Schematic Capture Json File Version 5.0` | v5 | JSON |

**SchLib headers**:
| Header String | Version | Format |
|----------------------------------------------------------------------------|---------|--------|
| `Protel for Windows - Schematic Library Editor Binary File Version 1.2 - 2.0` | v4 | Binary |
| `Protel for Windows - Schematic Library Editor Binary File Version 5.0` | v5 | Binary |
| `Protel for Windows - Schematic Library Editor Ascii File Version 1.2 - 2.0` | v4 | ASCII |
| `Protel for Windows - Schematic Library Editor Ascii File Version 5.0` | v5 | ASCII |
| `Altium Designer - Schematic Library Editor Json File Version 5.0` | v5 | JSON |

Note the quirky v4 header: `"Version 1.2 - 2.0"` is the v4 header despite the
enum being called `ffv4`. This is a legacy artifact from Protel 99 SE.


## IntLib (Integrated/Compiled Libraries)

`.IntLib` files are **compiled libraries** -- OLE compound documents that bundle a
SchLib and one or more PcbLib (or other model libraries) into a single container.
They do not have their own file format version enum. The embedded SchLib and PcbLib
sub-documents inside an IntLib use their respective version schemes above.

The file association in Altium is:
- Extension: `.IntLib`
- Internal name: `AltiumCompiledLibrary`
- Display name: `Altium Compiled Library`


## Harness Family (AD 24+)

**Source**: `FileFormatConsts.cs`

Newer Altium versions add harness design files. These all use version 1.0:

| File Type | Header String |
|---------------------|--------------------------------------------------------------------|
| Harness Wiring Diagram | `Altium Designer - Harness Wiring Diagram Binary File Version 1.0` |
| Harness Layout Drawing | `Altium Designer - Harness Layout Drawing Binary File Version 1.0` |
| Harness Library | `Altium Designer - Harness Library Binary File Version 1.0` |

Each also has ASCII and JSON variants with corresponding header strings.


## Summary: File Extension to Version Mapping

| Extension | Format Family | Current Version | Header Identifier |
|-----------|---------------|-----------------|-------------------------------------|
| `.PcbDoc` | PCB | V6 (binary) | `"PCB 6.0 Binary File"` |
| `.PcbLib` | PCB | V6 (binary) | `"PCB 6.0 Library File"` |
| `.PvLib` | PCB | V6 (binary) | (PadViaLibrary_V6) |
| `.SchDoc` | Schematic | V5 (parametric) | `"...Schematic Capture Binary File Version 5.0"` |
| `.SchLib` | Schematic | V5 (parametric) | `"...Schematic Library Editor Binary File Version 5.0"` |
| `.IntLib` | Container | N/A | Contains embedded SchLib + PcbLib |
