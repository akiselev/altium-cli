# 3DRouting Sections (Data, XYZ, Surface, Sketches)

Four CFB sections for 3D routing / rigid-flex PCB support, used in PcbDoc files
that have rigid-flex board designs with MID (Molded Interconnect Device) or
multi-surface routing.

Feature flag: `System.3DMID` (gated by `PCB3DRoutingExtensionIsInstalled()`).
Board property: `IPCB_BoardEx2.GetState_RigidFlexAdvanced()`.

## Overview

3D routing allows traces and primitives to be placed on 3D surfaces (e.g., the
curved flex regions of a rigid-flex PCB). Unlike standard 2D routing which uses
X/Y coordinates on flat copper layers, 3D routing uses UV face-mapped coordinates
(UVF format) or XYZ world coordinates (XYZ format) to position primitives on
arbitrary 3D mesh surfaces.

The four sections form two pairs:

| Section | Section Index | Delphi Class | Description |
|---------|---------------|-------------|-------------|
| `3DRoutingData` | 59 | `T3DRoutingUVFSection` | V1 format: UV face-mapped routing data |
| `3DRoutingXYZData` | 60 | `T3DRoutingXYZSection` | V2 format: XYZ world-coordinate routing data |
| `3DRoutingSurfaceData` | 61 | `T3DRoutingSurfaceSection` | 3D surface mesh definitions |
| `3DRoutingSketchesData` | 62 | `T3DRoutingSketchesSection` | Routing sketch/guide data |

All four classes live in the Delphi unit `Section_3DRouting`.

## Architecture

```
Section_3DRouting (Delphi unit)
  |
  +-- T3DRoutingUVFSection  (v1: UV-face mapped, 0x28-byte records)
  |     +-- TExportParams3DRUVF     (export parameters for v1)
  |     +-- TExported3DRouteCoord   (coordinate record)
  |     +-- TExportLayout           (layout record)
  |     +-- TExportPath             (path record)
  |     +-- TExportPathKey          (path key for dictionary lookup)
  |     +-- TExportRec              (export record)
  |     +-- TExportRecArr           (array of export records)
  |
  +-- T3DRoutingXYZSection  (v2: XYZ world-coordinate, 0x50-byte records)
  |     +-- TExportParams3DRXYZ     (export parameters for v2)
  |     +-- TExportLayout           (layout record)
  |     +-- TExportPath             (path record)
  |     +-- TExportPathKey          (path key for dictionary lookup)
  |     +-- TExportRecP             (export record - pointer variant)
  |     +-- TExportRecArrP          (array of export records)
  |
  +-- T3DRoutingSurfaceSection (surface mesh definitions)
  |
  +-- T3DRoutingSketchesSection (routing sketches/guides)
```

Both UVF and XYZ sections use a **columnar format** with named columns,
NOT the standard PcbDoc per-type binary record format.

## CFB Storage Layout

Unlike standard PcbDoc sections (which have Header + Data streams), the
3D routing sections use a columnar sub-stream layout:

```
/3DRoutingData/
    HeaderPrim       (column headers for primitive data)
    DataPrim         (columnar primitive records)
    HeaderTrack      (column headers for track data)
    DataTrack        (columnar track records)
    HeaderRegion     (column headers for region data)
    DataRegion       (columnar region records)
    BinPaths         (binary path data)

/3DRoutingXYZData/
    HeaderPrim       (same sub-stream layout)
    DataPrim
    HeaderTrack
    DataTrack
    HeaderRegion
    DataRegion
    BinPaths

/3DRoutingSurfaceData/
    (format TBD - likely parameter blocks or binary)

/3DRoutingSketchesData/
    (format TBD - likely parameter blocks or binary)
```

## Columnar Format

The UVF and XYZ sections use a **column-oriented** storage format. Each
sub-section (Prim, Track, Region) has a Header stream defining column names
and a Data stream containing packed column data.

### Header Stream Format

The header stream contains the column names as pipe-delimited key-value
parameters. Each column name maps to an index in the column enum.

### Data Stream Format

Records are fixed-size per entry:
- **V1 (UVF)**: 0x28 bytes (40 bytes) per record = 10 x i32
- **V2 (XYZ)**: 0x50 bytes (80 bytes) per record = 20 x i32

### Column Reader Data Types

The column data uses typed readers depending on the column's suffix:

| Suffix | Type | Size | Ghidra Function |
|--------|------|------|-----------------|
| `_i` | i32 (integer) | 4 bytes | FUN_0187da60 |
| `_d` | f64 (double) | 8 bytes | FUN_01884620 |
| `_f` | f32 (float) | 4 bytes | FUN_018845a0 |
| `_w` | u16 (word) | 2 bytes | FUN_0187dae0 |
| `_b` | u8 (byte) | 1 byte | FUN_0187e5e0 |
| `_s` | string | variable | FUN_0187e680 |
| (none) | i32 (default) | 4 bytes | FUN_0187da60 |

## Section 1: 3DRoutingData (T3DRoutingUVFSection)

V1 format using UV face-mapped coordinates. Primitives are positioned using
UV parameters on mesh faces, plus a face index.

### V1 Column Enum (15 columns)

From Delphi RTTI at address `0x01a20eec`:

| Index | Column Name | Type | Description |
|-------|-------------|------|-------------|
| 0 | `epIndexForSave` | i32 | Save-order index |
| 1 | `epObjectId` | i32 | PCB object type ID |
| 2 | `epFaceU_i` | i32 | UV coordinate U on face |
| 3 | `epFaceV_i` | i32 | UV coordinate V on face |
| 4 | `epFaceRot_d` | f64 | Rotation angle on face (degrees?) |
| 5 | `epFaceIdx_i` | i32 | Face index in surface mesh |
| 6 | `epFaceU2_i` | i32 | Second UV coordinate U (end point) |
| 7 | `epFaceV2_i` | i32 | Second UV coordinate V (end point) |
| 8 | `epFaceIdx2_i` | i32 | Second face index (end point) |
| 9 | `epUnknown_b` | u8 | Unknown byte field |
| 10 | `epUnknown_w` | u16 | Unknown word field |
| 11 | `epUnknown_i` | i32 | Unknown integer field |
| 12 | `epUnknown_f` | f32 | Unknown float field |
| 13 | `epUnknown_d` | f64 | Unknown double field |
| 14 | `epUnknown_s` | string | Unknown string field |

### V1 Track Writer Columns

The Track section writer (`FUN_01a04e20`) writes columns at indices:
`[0, 1, 6, 7, 8]` = `[epIndexForSave, epObjectId, epFaceU2_i, epFaceV2_i, epFaceIdx2_i]`

### V1 Record Size

Each record is 0x28 (40) bytes, read by `FUN_01a077c0` as 10 x i32.

### Key Delphi Functions (V1)

| Function | Purpose |
|----------|---------|
| `FUN_01a05e50` | V1 section reader (reads all 7 sub-streams) |
| `FUN_01a077c0` | V1 column-based record reader (0x28-byte records) |
| `FUN_01a04e20` | V1 Track section writer |
| `FUN_01a07590` | V1 header column name reader |
| `FUN_01a07460` | V1 column name -> index mapper |

### Key Nested Types (V1)

| Type | Description |
|------|-------------|
| `TExportParams3DRUVF` | Export parameter block for V1 |
| `TExported3DRouteCoord` | 3D route coordinate (12 bytes, 0x0C) |
| `TExportLayout` | Layout/arrangement definition |
| `TExportPath` | A routing path (sequence of coordinates on faces) |
| `TExportPathKey` | Dictionary key for path lookup |
| `TExportRec` | Single export record |
| `TExportRecArr` | Array of export records |

Paths are stored in `TDictionary<TExportPathKey, TExportPath>`.

## Section 2: 3DRoutingXYZData (T3DRoutingXYZSection)

V2 format using XYZ world coordinates. Primitives are positioned using
full 3D XYZ coordinates instead of UV face parameters. This is likely the
newer/preferred format.

### V2 Column Enum (17 columns)

The V2 format extends V1 with 2 additional columns (exact names TBD from
further Ghidra analysis at address `0x019fc600`). The V2 enum likely includes
the same base columns as V1 plus XYZ-specific fields:

Expected to include all V1 columns plus:
- XYZ world coordinates (x, y, z as i32)
- Additional surface/normal data

### V2 Track Writer Columns

The Track section writer (`FUN_01a0a3b0`) writes columns at indices:
`[0, 1, 8, 9, 10]`

Note the different column indices vs V1 `[0, 1, 6, 7, 8]` -- this reflects
the expanded column set in V2.

### V2 Record Size

Each record is 0x50 (80) bytes, read by `FUN_01a0d5f0` as 20 x i32
(double the V1 record count).

### Key Delphi Functions (V2)

| Function | Purpose |
|----------|---------|
| `FUN_01a0b300` | V2 section reader (reads all 7 sub-streams) |
| `FUN_01a0d5f0` | V2 column-based record reader (0x50-byte records) |
| `FUN_01a0a3b0` | V2 Track section writer |
| `FUN_01a0d3b0` | V2 header column name reader |
| `FUN_01a0d260` | V2 column name -> index mapper |

### Key Nested Types (V2)

| Type | Description |
|------|-------------|
| `TExportParams3DRXYZ` | Export parameter block for V2 |
| `TExportLayout` | Layout definition |
| `TExportPath` | Routing path |
| `TExportPathKey` | Dictionary key for path lookup |
| `TExportRecP` | Export record (pointer variant) |
| `TExportRecArrP` | Array of export records (pointer variant) |

The `P` suffix on `TExportRecP` / `TExportRecArrP` suggests these may use
pointers or are allocated differently from the V1 variants.

## Section 3: 3DRoutingSurfaceData (T3DRoutingSurfaceSection)

Stores 3D surface mesh definitions that the routing primitives reference
via face indices. This section defines the 3D geometry (triangulated mesh)
of the board's flex regions.

**Delphi class**: `T3DRoutingSurfaceSection`
**RTTI addresses**: `0x047fe32b` (list), `0x047fe3d3` (pointer)

Limited decompilation data available. The surface section likely contains:
- Vertex lists (X, Y, Z coordinates)
- Face/triangle indices
- Surface normals
- UV mapping data for texture/routing coordinate systems

## Section 4: 3DRoutingSketchesData (T3DRoutingSketchesSection)

Stores routing sketch and guide data -- likely the 2D sketch paths that
define routing intent before they are mapped onto 3D surfaces.

**Delphi class**: `T3DRoutingSketchesSection`
**RTTI addresses**: `0x01a21f5c` (list), `0x01a22003` (section)

Limited decompilation data available. Sketches likely contain:
- 2D polyline/spline paths (routing intent)
- Constraints and net assignments
- Mapping from sketch to surface

## Relationship Between Sections

```
3DRoutingSurfaceData  -- defines 3D mesh geometry (vertices, faces)
         |
         v
3DRoutingData (UVF)   -- V1: primitives positioned via UV coords on faces
3DRoutingXYZData (XYZ) -- V2: primitives positioned via XYZ world coords
         |
         v
3DRoutingSketchesData -- routing guides/intent sketches
```

The Surface section provides the 3D geometry. The UVF and XYZ sections
reference faces in the surface mesh (via `epFaceIdx_i`). The Sketches
section provides the routing intent/guides.

A board may contain UVF data, XYZ data, or both. The V2 (XYZ) format is
likely the preferred/current format, with V1 (UVF) being legacy.

## C# / .NET References

The 3D routing feature is gated by:
- `System.3DMID` internal option (InternalOptionBoolean)
- `PCB3DRoutingExtensionIsInstalled()` returning true
- `IPCB_BoardEx2.GetState_RigidFlexAdvanced()` board property
- `IPCB_ManufacturingInfo_Board.GetIsRigidFlexBoard()` manufacturing info
- `IPCBGraphicalViewInterface.Allow3DRouting()` / `IPCBGraphicalViewInterface2.Allow3DRouting()`

Feature name constants:
- `PCB.RigidFlex` (from `RT_FeatureNames/Consts.cs`)

The `Routing3D.Outputer` assembly (Altium Limited, 2019) handles 3D routing
output/export functionality.

## Source References

### Delphi (Ghidra)
- Binary: `Advpcb.dll` (altium26 project)
- Unit: `Section_3DRouting`
- V1 RTTI: `T3DRoutingUVFSection` at `0x01a21643`
- V2 RTTI: `T3DRoutingXYZSection` at various RTTI addresses
- V1 column enum: 15 entries at `0x01a20eec`
- V2 column enum: 17 entries (address `0x019fc600` per docs)

### C# (.NET)
- `AD26-dotnet/Altium.InternalOptions/InternalOptionBase.cs` - `System.3DMID` gate
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_BoardEx2.cs` - `RigidFlexAdvanced` property
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_ManufacturingInfo_Board.cs` - `IsRigidFlexBoard`
- `AD26-dotnet/Altium.Edp.Interfaces/GraphicalInterface/IPCBGraphicalViewInterface.cs` - `Allow3DRouting()`
- `AD26-dotnet/Routing3D.Outputer/` - 3D routing output assembly

### Existing Documentation
- `docs/dxp/altium-NOTES.md` lines 2700-2717: Columnar format details
- `docs/dxp/altium-NOTES.md` lines 2118-2137: Ghidra function cross-reference
- `docs/pcbdoc/stream_table.md`: Section index numbers
- `docs/dxp/sidecar-streams-deep-dive.md`: Section indices 59-62

## Implementation Notes

These sections are uncommon -- they only appear in PcbDoc files with
rigid-flex 3D routing enabled. Most boards do not use 3D routing, so these
streams will typically be empty or absent.

When present, the columnar format requires a fundamentally different parser
than the standard PcbDoc binary record format. The column-oriented layout
means:
1. Parse the Header sub-stream to discover column names and order
2. Read the Data sub-stream using the column definitions
3. Each record is a fixed-size row with columns packed sequentially
4. The BinPaths sub-stream contains additional binary path data

For initial implementation, returning a hard error for non-empty 3D routing
sections is appropriate. These sections can be parsed later when we have
test fixtures containing rigid-flex designs.
