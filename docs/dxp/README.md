# Altium Designer File Format Documentation

Reverse-engineered documentation for the Altium Designer DXP binary file formats (`.SchDoc`, `.SchLib`, `.PcbDoc`, `.PcbLib`, `.PrjPcb`, `.IntLib`). Based on decompiled .NET assemblies from AD26, Ghidra analysis of Delphi DLLs, and the KiCad Altium importer.

## How to Read This

Start with **container-format.md** for the big picture, then dive into the domain you care about (schematic or PCB). The reference files at the bottom are raw data dumps useful for cross-checking specific values.

## File Index

### Core Format

| File | Description |
|------|-------------|
| [container-format.md](container-format.md) | **Start here.** OLE/CFB container structure, block encoding (size-prefixed with flags), stream layouts for each file type (SchLib, SchDoc, PcbDoc, PcbLib, PrjPcb, IntLib), parameter string encoding (pipe-delimited, Windows-1252), and end-to-end reading flow for both schematic and PCB. |
| [coordinates.md](coordinates.md) | Coordinate system used across all file types. Internal units (10,000 = 1 mil), DXP fractional encoding for schematic parameters (`X` + `X_FRAC`), indexed vertex coordinates, PCB binary coordinate layout, and Win32 COLORREF color encoding. |
| [serialization.md](serialization.md) | The `altium-format-derive` proc macros (`AltiumRecord`, `AltiumBase`, `AltiumEnum`) and the traits they generate (`FromParams`/`ToParams`, `FromBinary`/`ToBinary`, `FromParamValue`/`ToParamValue`). Field attribute syntax for mapping struct fields to Altium parameters. |

### Schematic Domain

| File | Description |
|------|-------------|
| [schematic-records.md](schematic-records.md) | All schematic record types (RECORD=1 through 209). Parameter-based format, dispatch enum, base types (`SchPrimitiveBase`, `SchGraphicalBase`), ownership model via `OWNERINDEX`, multi-part symbols, display modes. Detailed field layouts for SchComponent, SchPin, SchWire, SchNetLabel, SchPowerObject, SchRectangle, SchDesignator, SchParameter, and the implementation/footprint-assignment records (44-48). |
| [sch-files.md](sch-files.md) | Complete SchDoc and SchLib loading/saving pipelines reverse-engineered from `Altium.Sch.DataModel.dll`. Class hierarchy (importers, exporters, serializers), CFB document structure for both file types, record format within streams, three-warehouse architecture (Base, Extended, Additional), object hierarchy and OWNERINDEX linking, font table format, FileHeader format, alias/redirection system, export pipeline, embedded object container format, and binary-code-to-TObjectId mapping. |
| [sch-dotnet-model.md](sch-dotnet-model.md) | Decompiled .NET interface reference for the schematic data model. Two generations of interfaces (legacy COM `SCHInterfaces` and modern `Altium.Sch.Interfaces.Objects`), interface hierarchy, TObjectId enumeration, base/graphical/component/pin/document interfaces, connectivity objects, iterator patterns, parameter system, unique ID management, and cross-document references. |
| [sch-api-functions.md](sch-api-functions.md) | All 135 `SchAPI_*` exports from `AdvSch.dll` (Delphi). Document/window management, object creation/destruction, iterators, property getters/setters, library operations, and utility functions. |

### PCB Domain

| File | Description |
|------|-------------|
| [pcb-records.md](pcb-records.md) | All PCB record types (object IDs 1-14). Binary record format (u8 object ID + little-endian packed fields), dispatch enum, common header (`PcbPrimitiveCommon` with layer/flags/unique_id), layer value table, PcbFlags bitmask. Detailed field layouts for PcbPad (stack modes, pad shapes, mask expansion, per-layer arrays), PcbTrack, PcbVia (from/to layers, via types), PcbArc, PcbText, PcbFill, PcbPolygon, PcbRegion, PcbComponentBody, PcbDimension. PCB flat ownership model. |
| [pcb-files.md](pcb-files.md) | Complete PcbDoc and PcbLib binary file format guide from decompiled .NET and Ghidra analysis. Architecture overview, .NET interface hierarchy (`IPCB_StructuredStorage`, section types), TObjectId system (Pcbtypes vs RT_PCB variants), V6 and V7 layer systems, CFB stream layouts, section loading order, per-primitive binary format details. |
| [pcb-dotnet-model.md](pcb-dotnet-model.md) | Decompiled .NET interface reference for the PCB data model. TObjectId enumeration (26 types), V6 layer IDs (`TV6_Layer`), V7 layer system (`IV7_Layer` with genus/family/species), SDK and runtime interface definitions, pad/via/track/text/component interfaces, and layer stack management. |
| [pcb-api-functions.md](pcb-api-functions.md) | All ~290 `PcbApi_*` exports from `Advpcb.dll` (Delphi). Iterator/traversal, object factory, property query (primitives, board, layers, components, rules, dimensions), container management, export/painter, library reader, event/robot, and undo/redo functions. |

### Sidecar Streams & Extended Data

| File | Description |
|------|-------------|
| [sidecar-streams-deep-dive.md](sidecar-streams-deep-dive.md) | How supplementary data is stored in separate CFB streams alongside main records. Architectural overview (format-evolution artifacts that merge into runtime objects on load). Complete stream load orders for SchLib (15 steps including 9 pin sidecar streams), SchDoc (8 steps), PcbDoc, and PcbLib. Per-stream format details: WideStrings, UniqueIDs, ExtendedPrimitiveInfo, PinFrac, PinDesc, PinMiscData, PinWideText, PinSymbolLineWidth, PinPackageLength, PinPropagationDelay, PinFunctionData. |

### Altium Internals & Reverse Engineering

| File | Description |
|------|-------------|
| [dotnet-delphi.md](dotnet-delphi.md) | How .NET managed code interacts with native Delphi code in AD26. COM interop architecture (not P/Invoke), binary inventory (which DLLs are Delphi vs .NET 8 R2R), OLE compound file structure, record format (ASCII mode 0 vs binary mode 1), serializer class hierarchy, and the `GetNextLine()` method that reads records from streams. |
| [impedance-mismatch-analysis.md](impedance-mismatch-analysis.md) | Gap analysis between Altium's actual API surface and our `altium-format` Rust implementation. Executive summary of "we model serialization, Altium models the design domain." Comparison table of capabilities (iterators, layer system, design rules, pad stack, mask expansion, net/class management, undo). Lists all 26 PCB object types with implementation status and priority additions. |
| [altium-NOTES.md](altium-NOTES.md) | Working notes from reverse engineering AD26 to debug "I/O Error 32" when opening altium-cli-written SchLib files. Installation layout, technology stack history (Protel 1985 -> Delphi -> AD26 .NET 8 migration), how to distinguish Delphi vs .NET 8 vs .NET Framework binaries, key binary inventory for SchLib analysis, Delphi class names and error classes from AdvSch.dll and X2.EXE. |

### Pad Format Deep Dives

| File | Description |
|------|-------------|
| [altium-pad-field-analysis.md](altium-pad-field-analysis.md) | Byte-by-byte layout of the PCB pad binary record (subrecord 5). Known fields with offsets (layer, flags, net, component, position, sizes, shapes, direction, plated, pad_mode, paste/solder mask expansion). Tables of all unknown fields at specific offsets with their Ghidra function addresses and .NET property candidates. |
| [altium-pad-unknowns-hypothesis.md](altium-pad-unknowns-hypothesis.md) | Hypotheses for each unknown pad field based on .NET interface cross-referencing and domain knowledge. Priority 1 single-byte fields (hole type, assembly test points, paste enable flags). Priority 2 four-byte fields (thermal relief airgap/conductor width, union index, pad offset). |
| [altium-pad-unknowns-REPORT.md](altium-pad-unknowns-REPORT.md) | Formal report documenting the 20 unknown pad fields with confidence levels. Each entry includes the offset, hypothesized field name, evidence, recommended Rust field name, expected value range, and confidence percentage (85-90%). |

### Raw Reference Data

| File | Description |
|------|-------------|
| [altium-types.md](altium-types.md) | Complete dump of Altium Delphi enumerated types from the DXP API documentation. Covers TLayer (82 layers), TObjectId (26 types), TShape, TRuleKind (70+ rule types), TPadMode, TDimensionKind, and dozens of other enumerations used throughout the PCB API. |
| [altium-constants.md](altium-constants.md) | Altium Delphi constants: AllLayers, AllObjects, AllPrimitives sets, layer string arrays, drawing order arrays, layer color constants, and other named constant values from the DXP scripting API. |
| [altium-parser.cpp](altium-parser.cpp) | KiCad's C++ Altium PCB parser (`altium_parser_pcb.cpp`). Reference implementation for reading arcs, pads, vias, tracks, text, fills, regions, components, dimensions, polygons, rules, nets, classes, and models from Altium binary streams. Useful for cross-checking binary offsets and field interpretations. |
