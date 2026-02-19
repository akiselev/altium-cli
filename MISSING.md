# MISSING

Data-model and format notes for stream gaps listed in `UNIMPLEMENTED.md`.

Evidence used:

- `scripts/ole-inspect.py` scan output (`/tmp/ole-scan.json`)
- AD26 .NET decompile tree in `AD26-dotnet/`
- `ghidra-cli` project `altium26` (program `Advpcb.dll`)
- Unicode string extraction from `AD26/System/Altium.PCB.BinaryLoader.dll`

## 1) Shared framing rules

Schematic side (`.SchDoc`, `.SchLib`) uses parameter/binary records in streams:

- stream header record: `RECORD=0`
- metadata fields: `HEADER`, `Weight` (and sometimes `MinorVersion`, `UniqueID`)
- payload records:
  - text object records: `RECORD=<id>` or `RECORD=254` + `RECORDEX=<id>`
  - embedded binary records: `BINARY=208` + `SchDataEmbeddedObject`
- terminator: `RECORD=0`

This framing is visible in:

- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataExporterBaseV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataImporterBaseV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataExporterLibraryV5.cs`
- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/SchDataImporterLibraryV5.cs`

PCB side (`.PcbDoc`, `.PcbLib`) is section-based structured storage:

- section name -> typically paired `Header` and `Data` streams
- common section API: record list + GUID table + extended index table + per-item index maps
- some sections specialize behavior (models, dimensions, polygons, layer kind mapping, violations)

This is visible in:

- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_StructuredStorage.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_BinarySection.cs`
- section-specific interfaces under `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/`
- BinaryLoader wide strings:
  - `Open section %s (%d records)`
  - `Open data stream %s, section %s (size %d bytes)`
  - stream labels `Header`, `Data`

## 2) Missing schematic streams

### `.SchDoc`

- `Additional`
  - object-list stream with normal record framing (`RECORD`, `Weight`, object records, end marker).
  - importer path: `SchDataImporterDocumentV5.ReadAdditionalWareHouse`.
- `Storage`
  - embedded object stream (`BINARY=208`) used for icon/image storage.
  - header text is `"Icon storage"`.

### `.SchLib`

- `Storage`
  - same embedded-object stream framing as `.SchDoc`.
- `<Item>/Redirection`
  - alias indirection stream in alias storages.
  - payload is `RECORD=0` + `SectionName=<canonical LibRef>`.
- `<Item>/PinFrac`
  - embedded-object stream keyed by pin index (`name = "<pin_index>"`).
  - each object data is 12 bytes: 3 little-endian `i32` values:
    - fractional `Location.X`, fractional `Location.Y`, fractional pin length.
- `<Item>/PinTextData`
  - embedded-object stream keyed by pin index.
  - payload is two packed blocks (name then designator), each encoded by a 1-byte bitfield plus optional fields:
    - bit `0x01`: custom position present (`i32` margin follows)
    - bit `0x02`: rotation anchor component vs pin
    - bits `0x0C`: rotation enum
    - bit `0x10`: custom font present (`i16 font_id` + `u32 color`)
- `<Item>/PinSymbolLineWidth`
  - embedded-object stream keyed by pin index.
  - each object stores UTF-16 parameter text with key `SymBol_LineWidth` inside a length-prefixed blob.
- `<Item>/PinPackageLength`
  - embedded-object stream keyed by pin index.
  - each object stores UTF-16 parameter text with key `PinPackageLength` inside a length-prefixed blob.

Stream constants are declared in:

- `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs`

## 3) Missing PCB streams

The missing `.PcbDoc` and `.PcbLib` streams are all part of the same section system. The missing pieces in v2 are section readers/writers, not one-off unrelated formats.

### A. Primitive/object sections (`*/Data`, `*/Header`)

Streams:

- `Arcs6`, `Tracks6`, `Pads6`, `Vias6`, `Fills6`, `Texts6`, `Texts`
- `Regions6`, `ShapeBasedRegions6`, `ComponentBodies6`, `ShapeBasedComponentBodies6`
- `Polygons6`, `Dimensions6`, `Coordinates6`

Models:

- base: `IPCB_BinarySection`
- polygons: `IPCB_PolygonsBinarySection`
- dimensions: `IPCB_DimensionsSection`

Ghidra evidence in `Advpcb.dll`:

- `PcbApi_QueryPolygon` (`0x03d32f80`) and `PcbApi_QueryPolygonSegmentCount` (`0x03d332c0`)
  - polygon segment record copied as `0x25` bytes each
  - guarded max segment count path `< 0x1389` (5001)
- `PcbApi_QueryDimension` (`0x03d2df80`)
  - hard object-id check for `0x0D` (dimension)
- `PcbApi_QueryExtDimension` (`0x03d2e1e0`)
  - extended dimension field block
- `PcbApi_QueryCoordinate` (`0x03d2f010`)
  - hard object-id check for `0x0E` (coordinate)

### B. Board/connectivity/rules sections

Streams:

- `Board6`, `Components6`, `Nets6`, `Rules6`, `Classes6`, `Connections6`
- `FromTos6`, `DifferentialPairs6`, `PinPairsSection`
- `SignalClasses`, `SmartUnions`, `UnionNames`

Models:

- `IPCB_FromTo`, `IPCB_DifferentialPair`, `IPCB_PinPair`, `IPCB_ObjectClass`, `IPCB_Union*`
- stored via the same `IPCB_BinarySection` section container.

### C. Metadata/index sections

Streams:

- `WideStrings6` and `.PcbLib` `<Item>/WideStrings`
- `PrimitiveParameters`
- `UniqueIDPrimitiveInformation`
- `ExtendedPrimitiveInformation`
- `PrimitiveGuids`
- `FileVersionInfo`
- `WaivedViolations`
- `FileHeader`, `FileHeaderSix` (with `Binary6Version`)

Models and field hints:

- wide strings table: `IPCB_WideStrings`
- primitive parameter variants: `IPCB_PrimitiveParameters`
  - BinaryLoader strings include: `PrimitiveID`, `VariantGUID`, `Appurtenance`, `HasNoParameters`
- primitive GUID table: `IPCB_BinarySection` GUID methods (`GuidsCount`, `GetGUID`, `AddGUID`)
- file version feature list: `PCBInterfaces/IPCB_FileVersionInfoList.cs`
- waived violation metadata:
  - model: `IPCB_WaivedViolationInfo`
  - BinaryLoader strings include: `CreatedAt`, `AuthorID`, `AuthorTitle`, `Source`, `Comment`

### D. Resource/library sections

Streams:

- `Models`, `ModelsNoEmbed`, `Textures`, `BoardRegions`
- `EmbeddedFonts6`, `Embeddeds6`, `EmbeddedBoards6`
- `LayerKindMapping`
- `PadViaLibrary`, `PadViaLibraryCache`, `PadViaLibraryLinks`
- `.PcbLib` `Library/*` streams:
  - `Library/Data`, `Library/Header`, `Library/EmbeddedFonts`
  - `Library/ComponentParamsTOC/{Header,Data}`
  - `Library/LayerKindMapping/{Header,Data}`
  - `Library/PadViaLibrary/{Header,Data}`
  - `Library/Models/{Header,Data,<index>}`
  - `Library/ModelsNoEmbed/{Header,Data}`
  - `Library/Textures/{Header,Data}`

Models:

- models: `IPCB_ModelsSection`, `IPCB_ModelsNoEmbedSection`
- textures: `IPCB_TextureSection`
- board regions: `IPCB_BoardRegionsSection`
- layer kind mapping: `IPCB_LayerKindMappingSection`
  - ghidra API: `PcbApi_QueryBoardMechanicalLayerKindMapping` (`0x03d41a20`)
- pad/via libraries: `IPCB_PadViaLibrary`

BinaryLoader strings show all these section names verbatim (`Board6`, `Rules6`, `Models`, `LayerKindMapping`, `PadViaLibrary`, `WaivedViolations`, etc.), matching the missing stream inventory exactly.

### E. `.PcbLib` `SectionKeys`

- Stream appears in samples but v2 has `read_pcb_section_keys` stubbed.
- Current v2 behavior: always returns empty key map.
- file: `crates/altium-format/src/v2/documents/pcblib.rs`.

## 4) `.PrjPcb` missing stream

- Observed as non-OLE raw content (`(raw)`).
- v2 has no project parser; this needs a text-format parser (project documents, variants, options), not CFB stream handling.
