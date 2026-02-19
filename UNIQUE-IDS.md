# UNIQUE IDS

Strategy for deriving stable unique IDs for Altium streams and records during reverse engineering.

This document is intentionally implementation-oriented so coding agents can apply it mechanically.

## 1) Scope and Evidence

Sources used for stream taxonomy:

- `scripts/ole-inspect.py scan data --json` (observed real files)
- Schematic stream constants:
  - `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs`
- Schematic import/export stream usage:
  - `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.ImportExport/*.cs`
- PCB section model interfaces:
  - `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_StructuredStorage.cs`
  - `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_BinarySection.cs`
  - section-specialized interfaces under `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/`
- PCB loader string table (section names and fields):
  - `strings -el AD26/System/Altium.PCB.BinaryLoader.dll`
  - includes `Open section %s (%d records)`, `Open data stream %s, section %s`, `Header`, `Data`

## 2) ID Types (Do Not Conflate)

Use five distinct IDs:

1. `DTID` (document type id): static schema id for file kind.
2. `STID` (stream type id): static schema id for stream shape.
3. `SGID` (stream group id): logical multi-stream family id inside one file (for `Header/Data`, sidecars, indexed streams).
4. `SID` (stream instance id): concrete stream id inside one file.
5. `RTID`/`RID` (record type/instance ids): type id and concrete record id.

Recommended hash: `blake3-128` hex lowercase for computed IDs.

## 3) Canonicalization Rules

Before deriving any ID:

1. Normalize path separators to `/`.
2. Preserve original stream name case for storage, but use lowercase for hash input.
3. Resolve schematic section keys:
   - use `/SectionKeys` mapping (`LibRefN -> SectionKeyN`) when present.
4. Resolve schematic alias storages:
   - `/.../Redirection` stream `SectionName` points to canonical component.
5. Normalize numeric child streams:
   - `Models/<index>` and `Library/Models/<index>` kept as indexed role in group.
6. For per-item storages:
   - SchLib item key: component `UNIQUEID` from item `Data` record 1 if present, else canonical `LibRef`.
   - PcbLib item key: `PATTERN` from item `Parameters` (with duplicate disambiguation by occurrence index).

## 4) Deterministic ID Formats

## 4.1 Document Type IDs (DTID)

- `dtid:schdoc`
- `dtid:schlib`
- `dtid:pcbdoc`
- `dtid:pcblib`
- `dtid:prjpcb`
- `dtid:intlib`

Document instance key (`doc_key`) priority:

1. Intrinsic document unique id if reliably parseable.
2. Else file content hash (`blake3(file_bytes)`).

Use:

- `DID = "did:" + blake3_128(dtid + "|" + doc_key)`

## 4.2 Stream Type IDs (STID)

Static schema IDs (no file-specific data):

- `stid:<dtid>/<scope>/<family>/<role>`

Examples:

- `stid:schlib/item/data/main`
- `stid:schlib/item/pinfrac/data`
- `stid:pcbdoc/section/board6/header`
- `stid:pcbdoc/section/board6/data`
- `stid:pcblib/library/models/index`
- `stid:prjpcb/raw/main`

Roles:

- `main`, `header`, `data`, `params`, `widestrings`, `index`, `redirect`, `sidecar`

## 4.3 Stream Group IDs (SGID)

A stream group is one logical entity that may use multiple physical streams.

Use:

- `SGID = "sgid:" + blake3_128(DID + "|" + group_key)`

`group_key` examples:

- `root:fileheader`
- `section:board6`
- `section:models`
- `library-section:models`
- `component:<component_anchor>:pinfrac`
- `footprint:<pattern_anchor>:uniqueidprimitiveinformation`

## 4.4 Stream Instance IDs (SID)

Each concrete stream inside file:

- `SID = "sid:" + blake3_128(SGID + "|" + stream_key)`

`stream_key` examples:

- `role:header`
- `role:data`
- `role:index:17`
- `role:main`

For indexed model streams:

- Prefer semantic key if parseable: `role:index:modelid:<guid>`
- Else fallback: `role:index:<ordinal>`

## 4.5 Record Type/Instance IDs

Type IDs:

- Schematic: `rtid:sch:record:<RECORD_INT>`
- PCB primitive: `rtid:pcb:object:<OBJECT_ID>`
- PCB section-internal records (non-primitive): `rtid:pcb:section:<section_name>:<kind>`

Instance IDs:

- `RID = "rid:" + blake3_128(parent_anchor + "|" + RTID + "|" + record_anchor)`

`parent_anchor` is usually `SGID` or section-item anchor.

## 5) Multi-Stream Join Strategy (Critical)

This is the main requirement for not losing identity when streams are split.

## 5.1 Header/Data pairs

For any `<Section>/Header` and `<Section>/Data`:

1. Build one `SGID` for `section:<Section>`.
2. Derive:
   - `SID(header)` from `role:header`
   - `SID(data)` from `role:data`
3. All records parsed from `Data` inherit `parent_anchor = SGID`.

## 5.2 Indexed side streams (Models/<index>)

For `Models` and `Library/Models`:

1. One `SGID` for section (`section:models` or `library-section:models`).
2. `Header`/`Data` SIDs as above.
3. Each numeric child stream gets `SID(index)` with role `index`.
4. If `ModelID` is parseable, replace ordinal anchor with model id for stability across reorder.

## 5.3 SchLib pin extended streams

`PinFrac`, `PinTextData`, `PinSymbolLineWidth`, `PinPackageLength` (and other pin sidecars) are keyed by pin index in embedded object name.

Join rules:

1. Resolve component anchor (`component UNIQUEID` preferred).
2. Build per-component `SGID` per sidecar stream family.
3. Map sidecar payload item name `N` -> pin record in `Data` by pin order index `N`.
4. Derive pin `RID` primarily from pin `UNIQUEID`; fallback to `(component_anchor, pin_index)`.

## 5.4 PcbDoc/PcbLib sidecar sections

Sidecar families include:

- `WideStrings`
- `UniqueIDPrimitiveInformation`
- `ExtendedPrimitiveInformation`
- `PrimitiveParameters`
- `PrimitiveGuids`

Join on primitive index (`PrimitiveIndex` / section order), then upgrade to intrinsic GUID/UniqueID when available.

## 6) Record Anchor Priority

Use this precedence to maximize stability across save/reorder:

## 6.1 Schematic records

1. `UNIQUEID` parameter if present and non-empty.
2. For known sidecar-indexed items (pins): owner component anchor + pin index.
3. Else semantic fingerprint:
   - canonical params sorted by key, excluding volatile location/order-only keys when appropriate.
   - include owner `RID` where available.

## 6.2 PCB records

1. Primitive intrinsic unique id (`GetState_UniqueId`/`UniqueID` if parseable in record payload).
2. `UniqueIDPrimitiveInformation` mapping.
3. `PrimitiveGuids` mapping.
4. Section-specific semantic keys:
   - `PrimitiveParameters`: `PrimitiveIndex + VariantGUID + Appurtenance + Name`
   - model records: `ModelID (+ checksum)`
   - waived violations: `RuleIndex + involved primitive refs + CreatedAt + AuthorID + Source + comment_hash`
5. Fallback: `section + record_index + payload_hash`.

## 7) Collision Policy

If two non-identical entities produce same computed ID:

1. Keep base ID for first.
2. Append deterministic suffix for later duplicates:
   - `:dup2`, `:dup3`, ...
3. Persist duplicate map in side metadata for deterministic replay.

## 8) Stream Type Catalog (Reviewed)

## 8.1 Schematic stream names (from AD26 constants, 24)

- `Additional`
- `ReuseBlocks`
- `ReuseBlocksV2`
- `Data`
- `FileHeader`
- `LibAdditional`
- `PinFrac`
- `PinDesc`
- `PinMiscData`
- `PinTextData`
- `PinWideText`
- `PinSymbolLineWidth`
- `PinPackageLength`
- `PinPropagationDelay`
- `PinFunctionData`
- `Redirection`
- `SectionKeys`
- `Storage`
- `Files`
- `HarnessConnectionPointConnector`
- `HarnessComponentCrimps`
- `HarnessAssociatedParts`
- `ObjectDefinitions`
- `ReuseBlockInfos`

## 8.2 PCB section-name superset (from BinaryLoader strings)

These are logical section families. Physical streams are usually `<Section>/Header` and `<Section>/Data` unless section-specific.

V6 section names:

- `Board6`
- `Advanced Placer Options6`
- `Advanced Router Options6`
- `Design Rule Checker Options6`
- `Pin Swap Options6`
- `Classes6`
- `Nets6`
- `Components6`
- `Polygons6`
- `Dimensions6`
- `Coordinates6`
- `EmbeddedBoards6`
- `Connections6`
- `Rules6`
- `NewRules6`
- `FromTos6`
- `DifferentialPairs6`
- `Embeddeds6`
- `Arcs6`
- `Pads6`
- `Vias6`
- `Tracks6`
- `Texts6`
- `Fills6`
- `ShapeBasedRegions6`
- `Regions6`
- `ShapeBasedComponentBodies6`
- `ComponentBodies6`
- `WideStrings6`
- `EmbeddedFonts6`
- `SplitPlaneRegions6`

Shared/union:

- `UnionNames`
- `UnionRelations`
- `SmartUnions`

Legacy section names:

- `Board`
- `Advanced Placer Options`
- `Advanced Router Options`
- `Design Rule Checker Options`
- `Pin Swap Options`
- `Classes`
- `Nets`
- `Components`
- `Polygons`
- `Dimensions`
- `Coordinates`
- `EmbeddedBoards`
- `Connections`
- `Rules`
- `NewRules`
- `FromTos`
- `DifferentialPairs`
- `Embeddeds`
- `Arcs`
- `Pads`
- `Vias`
- `Tracks`
- `Texts`
- `Fills`
- `ShapeBasedRegions`
- `Regions`
- `ShapeBasedComponentBodies`
- `ComponentBodies`
- `WideStrings`
- `EmbeddedFonts`
- `SplitPlaneRegions`

Library/extended section names:

- `Library`
- `FileVersionInfo`
- `Models`
- `ModelsNoEmbed`
- `Textures`
- `Testpoint Options`
- `ExtendedPrimitiveInformation`
- `ExtendedPrimitiveIndices`
- `BoardRegions`
- `UniqueIDPrimitiveInformation`
- `ComponentParamsTOC`
- `LayerStackSection`
- `PinPairsSection`
- `SignalClasses`
- `PadViaLibrary`
- `PadViaLibraryCache`
- `PadViaLibraryLinks`
- `PadViaCacheLibraryLinksSection`
- `ConnectivityGraphCache`
- `ComponentCache`
- `GeometryZeroCache`
- `PrimitiveParameters`
- `WaivedViolations`
- `LayerKindMapping`
- `ConstraintManager`
- `3DRoutingData`
- `3DRoutingXYZData`
- `3DRoutingSurfaceData`
- `3DRoutingSketchesData`
- `MechanicalPrimitives`
- `CounterHolesSection`
- `CounterHolesPresetsSection`
- `ViaStructureManager`
- `ViaStructures`
- `UnionFeatures`
- `CustomShapes`
- `LayerToLayerMapping`
- `CustomReliefs`
- `PrimitiveGuids`
- `LettersGeometry`
- `SharedUnion`
- `CustomMaskShapes`
- `RuleAdditionalData`
- `CornerRadiusChamfer`
- `DrillManager`
- `xNetClassesSection`
- `Wirebonds`
- `WirebondTemplates`
- `WirebondBodies`
- `DiePadsInfo`
- `RegionHoles`

## 8.3 Observed special physical stream forms

- `FileHeader`
- `FileHeaderSix`
- `SectionKeys`
- `Storage`
- `Additional`
- `(raw)` for non-OLE `.PrjPcb`
- per-footprint:
  - `<Footprint>/Parameters`
  - `<Footprint>/Header`
  - `<Footprint>/Data`
  - `<Footprint>/WideStrings`
  - `<Footprint>/UniqueIDPrimitiveInformation/{Header,Data}`
  - `<Footprint>/ExtendedPrimitiveInformation/{Header,Data}`
  - `<Footprint>/PrimitiveGuids/{Header,Data}`
- indexed model blobs:
  - `Models/<index>`
  - `Library/Models/<index>`

## 8.4 Record type catalog (for RTID)

Schematic record IDs (known schema set from `docs/model/schematic-records.md` + AD26 evidence):

- `1`, `2`, `3`, `4`, `5`, `6`, `7`, `8`, `9`
- `11`, `12`, `13`, `14`
- `17`, `18`
- `22`
- `25`, `26`, `27`, `28`, `29`, `30`, `31`
- `34`, `37`
- `41`
- `43`, `44`, `45`, `46`, `47`, `48`
- `209`
- `225` (Blanket in AD26 mapping)

PCB primitive object IDs (known schema set from `docs/model/pcb-records.md`):

- `1`, `2`, `3`, `4`, `5`, `6`
- `10`, `11`, `12`, `13`, `14`

Rules for unknown/new IDs:

- If a schematic `RECORD` value is unseen, still assign `RTID = rtid:sch:record:<value>`.
- If a PCB object-id byte is unseen, still assign `RTID = rtid:pcb:object:<value>`.
- For section-internal records without stable numeric IDs, use:
  - `RTID = rtid:pcb:section:<section_name>:<kind>`
  - where `kind` is taken from decoded discriminator fields (`Record`, `ObjectKind`, etc.) when available.

## 9) Minimal Implementation Plan

1. Add a central `id::canonical` module:
   - path normalization
   - section-key and redirection resolution
2. Add stream grouping layer:
   - derive `SGID` first, then `SID`.
3. Add record-anchor extraction per format:
   - schematic params
   - pcb primitive/sidecar joins
4. Persist IDs in parse graph so later passes (queries/edits/writes) reuse exact same IDs.

This gives deterministic identities even when one logical entity is split across multiple streams (`Header/Data`, sidecars, indexed model streams).
