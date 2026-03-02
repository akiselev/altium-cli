# SplitPlaneRegions6

## Overview

`SplitPlaneRegions6` is a **primitive binary section** in PcbDoc files that stores **split plane region** primitives. Split planes are copper regions on internal power/ground plane layers that have been subdivided ("split") to create separate net zones. Each region in this section is a `IPCB_SplitPlaneRegion` which extends `IPCB_Region`.

Split planes are a PCB design concept where an internal copper plane layer is divided into electrically isolated regions, each assigned to a different net (e.g., splitting a ground plane to separate analog and digital ground). The `SplitPlaneRegions6` section stores the **region geometry** (the poured copper shapes), while the parent `SplitPlane` group objects that define the split boundaries are stored in the `Polygons6` section.

### Relationship to Other Sections

- **Polygons6**: Contains `IPCB_SplitPlane` (TObjectId = `eSplitPlaneObject` = 22) group objects that define the split plane boundaries. The `Polygons6` section has a `FoundSplitPlanes()` method on its interface (`IPCB_PolygonsBinarySection`) to detect split planes.
- **SplitPlaneRegions6**: Contains `IPCB_SplitPlaneRegion` primitives (TObjectId = `eRegionObject` = 11, with `TPCBRegionType::rtSplitPlaneRegion`) — these are the poured copper fill regions *generated* from the split plane polygons.
- **Regions6**: Normal copper regions (non-split-plane).
- **ShapeBasedRegions6**: Shape-based regions using TPolySegment geometry.
- **BoardRegions**: Legacy board outline regions.

The split plane region references its parent split plane object via `GetState_SplitPlane()` / `SetState_SplitPlane()`. The save/load pipeline stores this cross-reference via the `IPCB_Primitive_SaveLoadParameters.GetState_vSplitPlane()` method.

## CFB Storage Layout

```
/SplitPlaneRegions6/
    Header          (parameter block: record count + section metadata)
    Data            (packed binary primitive records)
```

This follows the standard PcbDoc binary section layout:
- **Header stream**: Contains a single parameter block with at minimum `RECORD_COUNT=N` (and potentially other section-level metadata).
- **Data stream**: Contains `N` packed binary records, each framed as:
  ```
  u8  object_id    (always 11 = eRegionObject)
  u32 payload_len  (little-endian, byte count of payload)
  [payload_len bytes]
  ```

### Note on Presence

This section is **optional**. It only appears in PcbDoc files that have internal plane layers with manual or automatic split planes. Most simple 2-layer or 4-layer boards without split planes will not have this section. The `IPCB_BoardBinarySection` interface includes `Found_ManualSplitPlanes()` to detect whether the section contains manually-defined split planes.

## Data Format

### Binary Record Layout

Each record in the Data stream uses the standard Region binary format (ObjectID = 11). The binary payload layout is:

```
Offset  Size    Type            Field
------  ----    ----            -----
0       13      CommonHeader    Standard PCB primitive common header
13      1       u8              Region kind (RegionKind enum)
14      4       i32 LE          Hole count (number of hole contours)
18      4       u32 LE          Parameter string length (includes NUL terminator)
22      N       Win1252 string  Pipe-delimited |KEY=VALUE| parameter string
22+N    4       i32 LE          Main contour vertex count
22+N+4  V*16    f64 LE pairs    Main contour vertices (V pairs of f64 x, f64 y)
...     ...     ...             Hole contours (same format, one per hole_count)
```

### Common Header (13 bytes)

Standard PCB primitive common header shared by all PCB primitives:

```
Offset  Size  Type    Field
------  ----  ----    -----
0       1     u8      Layer (TV6_Layer)
1       2     u16 LE  Flags (PcbFlags)
3       2     u16 LE  Net index (0xFFFF = none)
5       2     u16 LE  Polygon index (0xFFFF = none)
7       2     u16 LE  Component index (0xFFFF = none)
9       2     u16 LE  Coordinate index (0xFFFF = none)
11      2     u16 LE  Dimension index (0xFFFF = none)
```

For split plane regions:
- **Layer** will be an internal plane layer
- **Net index** references the net assigned to this split region
- **Polygon index** may reference the parent polygon/split plane
- **Component index** is typically 0xFFFF (not in a component)

### Parameter String Fields

The parameter string embedded in each record contains pipe-delimited key-value pairs. Expected parameters (same as standard Region records):

| Parameter | Type | Description |
|-----------|------|-------------|
| `V7_LAYER` | string | V7 layer name (e.g., "InternalPlane1") |
| `NAME` | string | Region name (often empty " ") |
| `KIND` | i32 | Region kind parameter (redundant with binary kind byte) |
| `SUBPOLYINDEX` | i32 | Sub-polygon index (-1 if not applicable) |
| `UNIONINDEX` | i32 | Union index for grouped objects |
| `ARCRESOLUTION` | mil string | Arc approximation resolution (e.g., "0.5mil") |
| `ISSHAPEBASED` | bool | Whether region uses shape-based geometry |
| `CAVITYHEIGHT` | mil string | Cavity height for 3D cavities (e.g., "0mil") |
| `KEEPOUTRESTRICTIONS` | i32 | Keepout restriction flags |
| `LAYER` | string | Layer name |
| `KEEPOUT` | bool | Whether this is a keepout region |
| `ISBOARDCUTOUT` | bool | Whether this is a board cutout |
| `PADINDEX` | i32 | Parent pad index (-1 if not applicable) |

### Contour Geometry

The region outline uses legacy f64 vertex format (NOT shape-based TPolySegment format):
- **Main contour**: `i32 vertex_count` followed by `vertex_count * (f64 x, f64 y)` pairs in internal units (10,000 = 1 mil)
- **Hole contours**: Same format, one per `hole_count`

Coordinates are f64 values that should be rounded to i32 internal units.

## Key Types and Interfaces

### Object/Type IDs

| Identifier | Value | Description |
|-----------|-------|-------------|
| `TObjectId::eRegionObject` | 11 | Object ID byte in binary record |
| `TObjectId::eSplitPlaneObject` | 22 | Object ID for split plane group (in Polygons6) |
| `TPCBRegionType::rtSplitPlaneRegion` | 2 | Region type discriminator |
| `TRegionKind` | 0-4 | Copper/Cutout/NamedRegion/BoardCutout/Cavity |
| `TPolygonType::eSplitPlanePolygon` | 1 (SDK) / 2 (RT_PCB) | Polygon type for split planes |

**WARNING**: The `TPolygonType` enum has different ordinals between namespaces:
- `PCB` (SDK): `eSignalLayerPolygon=0, eSplitPlanePolygon=1, eCoverlayOutlinePolygon=2`
- `RT_PCB` (runtime): `ptSignalLayerPolygon=0, ptCoverlayOutlinePolygon=1, ptSplitPlane=2`

### C# Interfaces

| Interface | Namespace | Description |
|-----------|-----------|-------------|
| `IPCB_SplitPlaneRegion` | `PCB`, `RT_PCB` | Split plane region (extends `IPCB_Region`) |
| `IPCB_SplitPlane` | `PCB`, `RT_PCB` | Split plane group (extends `IPCB_Group`) |
| `IPCB_SplitPlaneRegionHelper` | `PCB` | Extension methods for SplitPlaneRegion |
| `IPCB_SplitPlaneHelper` | `PCB` | Extension methods for SplitPlane |
| `IPCB_BoardBinarySection` | `RT_PCB` | Binary section with `Found_ManualSplitPlanes()` |
| `IPCB_PolygonsBinarySection` | `RT_PCB` | Polygons section with `FoundSplitPlanes()` |
| `IPCB_Primitive_SaveLoadParameters` | `PCBInterfaces` | Save/load params including `GetState_vSplitPlane()` |

### GUIDs

| Interface | GUID |
|-----------|------|
| `IPCB_SplitPlane` | `{82C47BE7-7BB4-4d2a-9E85-BCDAE1C0A632}` |
| `IPCB_SplitPlaneRegion` | `{9812F876-C979-4E79-B845-84668342D5D9}` |

### IPCB_SplitPlane Properties (Group Object in Polygons6)

The split plane group object defines the boundary polygon and pour settings:

| Property | Type | Description |
|----------|------|-------------|
| `AreaSize` | double | Computed area of the polygon |
| `PointCount` | int | Number of boundary vertices |
| `Segments[i]` | TPolySegment | Boundary polygon segments |
| `RemoveDead` | bool | Remove dead copper islands |
| `RemoveIslandsByArea` | bool | Remove islands smaller than threshold |
| `IslandAreaThreshold` | double | Minimum island area |
| `RemoveNarrowNecks` | bool | Remove narrow copper necks |
| `NeckWidthThreshold` | int (Coord) | Minimum neck width |
| `ArcApproximation` | int (Coord) | Arc approximation resolution |
| `OptimalVoidRotation` | bool | Optimize void rotation |
| `NegativeRegion` | IPCB_Region | The negative (cutout) region |

### IPCB_SplitPlaneRegion Properties (additional beyond IPCB_Region)

| Property | Type | Description |
|----------|------|-------------|
| `SplitPlane` | IPCB_SplitPlane | Reference to parent split plane group |
| `Kind` | TRegionKind | Region kind (inherited from IPCB_Region) |
| `Name` | string | Region name |
| `Area` | long | Computed area |
| `CavityHeight` | int (Coord) | Cavity height |
| `MainContour` | IPCB_Contour | Main outline contour |
| `HoleCount` | int | Number of holes |
| `Hole[i]` | IPCB_Contour | Hole contours |

### Board-Level Properties

| Property | Type | Description |
|----------|------|-------------|
| `AutomaticSplitPlanes` | bool | Whether split planes auto-repour |

### Constants

| Constant | Value | Source |
|----------|-------|--------|
| `kSplitPlaneTrackSize` | 100 | `xPCBTypes.Consts` — default track width for split plane boundaries |
| Service name | `"PCB_SplitPlaneRegion"` | `PCB.ServiceNames` |
| Comparison kind | `"SplitPlane"` | `RT_Comparison.Interfaces.Consts` |
| `SetSkipRebuldingSplitPlanesAfterLoad` | method | `IPCB_BoardEx3` — skip repour on load |

## Implementation Notes

### Relationship between SplitPlane and SplitPlaneRegion

The architecture follows a **polygon pour** model:

1. A `SplitPlane` (ObjectID 22) is a **group object** stored in `Polygons6` that defines:
   - The boundary polygon (vertices/segments)
   - Pour settings (island removal, neck removal, arc resolution)
   - Layer assignment (internal plane layer)

2. When the split plane is "poured", it generates `SplitPlaneRegion` primitives stored in `SplitPlaneRegions6`. These are the actual copper fill shapes with:
   - Precise contour geometry (main outline + holes for pad clearances, etc.)
   - Net assignment for each split region
   - Back-reference to the parent SplitPlane via `GetState_SplitPlane()`

3. The Polygons binary section tracks whether split planes were found (`FoundSplitPlanes()`), and the Board binary section tracks manually-defined split planes (`Found_ManualSplitPlanes()`).

### Binary Format Notes

- SplitPlaneRegions6 records use **ObjectID 11** (eRegionObject) — the same as Regions6 and ShapeBasedRegions6. The distinction between region types is via:
  - The **section** they are stored in (SplitPlaneRegions6 vs Regions6)
  - The `TPCBRegionType` discriminator (rtSplitPlaneRegion = 2)
- The contour format is **legacy f64** (not shape-based TPolySegment), based on the section characteristics observed in other Region sections.
- Split plane regions typically appear on internal plane layers only.

### Cross-reference Indices

The `IPCB_BinarySection.SetIndexes()` method stores:
- `vNet`: Index into `Nets6` — the net assigned to this split region
- `vPolygon`: Index into `Polygons6` — the parent split plane polygon
- `vComponent`: Typically 0xFFFF (not in a component)
- `vCoordinate`: Typically 0xFFFF
- `vDimension`: Typically 0xFFFF

## Source References

### C# Interface Files
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_SplitPlaneRegion.cs` — SDK dispatch interface
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_SplitPlane.cs` — SDK split plane group interface
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_SplitPlaneRegionHelper.cs` — Extension methods
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_SplitPlaneHelper.cs` — Extension methods
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_SplitPlaneRegion.cs` — RT interface (detailed)
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_SplitPlane.cs` — RT split plane interface (detailed)
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_BoardBinarySection.cs` — `Found_ManualSplitPlanes()`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_PolygonsBinarySection.cs` — `FoundSplitPlanes()`

### Type/Enum Files
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPCBRegionType.cs` — Region type enum (rtRegion, rtBoardRegion, rtSplitPlaneRegion)
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TRegionKind.cs` — Region kind enum (Copper, Cutout, NamedRegion, BoardCutout, Cavity)
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPCBPolygonType.cs` — Polygon type enum (SignalLayer, CoverlayOutline, SplitPlane)
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/TObjectId.cs` — Object ID enum (eSplitPlaneObject = 22)
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/TPolygonType.cs` — SDK polygon type (different ordinals!)

### Constants/Service Names
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/ServiceNames.cs` — `"PCB_SplitPlaneRegion"`
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/InterfaceGuids.cs` — GUIDs for SplitPlane and SplitPlaneRegion
- `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/Consts.cs` — `kSplitPlaneTrackSize = 100`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_Comparison.Interfaces/Consts.cs` — Comparison kind string

### Data Model/UI Files
- `AD26-dotnet/InteractiveProperties.Providers.PCB.DataModel/Altium.Designer.InteractiveProperties.Providers.PCB.DataModel/PcbSplitPlaneDataObject.cs` — UI data model
- `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Primitive_SaveLoadParameters.cs` — `GetState_vSplitPlane()`

### Existing Rust Implementation
- `crates/altium-format-types/src/pcb.rs` — Contains `PcbObjectId::SplitPlane = 22`, `ViewableObjectId::SplitPlane = 87`, `PolygonType::SplitPlane = 1`, `ClassMemberKind::SplitPlane = 8`
- `crates/altium-format/src/pcblib/primitives/region.rs` — Region binary parser (reusable for SplitPlaneRegions6)
- `crates/altium-format/src/pcbdoc/records.rs` — `PrimitiveSectionKind` enum (**does NOT yet include SplitPlaneRegions6**)
- `crates/altium-format/src/pcbdoc/primitives.rs` — Primitive dispatch (needs SplitPlaneRegions6 support)
