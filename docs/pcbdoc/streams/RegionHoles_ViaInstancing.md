# RegionHoles and ViaInstancing Sections

Research findings for two PcbDoc CFB sections: `RegionHoles` (section #84) and
`ViaInstancing` (section #85 / `Section_ViaInstance`).

## Section Overview

| Property | RegionHoles | ViaInstancing |
|----------|-------------|---------------|
| CFB stream name | `RegionHoles` | `ViaInstancing` |
| Delphi section constant | (none found) | `Section_ViaInstance` |
| Section ordinal | 84 | 85 |
| Category | Primitives (sidecar) | Via definitions |
| Feature gate | None (always available) | `PCB.ViaInstancing` feature flag |
| Present in test files | No (none of our fixtures contain it) | No |

---

## RegionHoles

### What Are Region Holes?

A "hole" in the context of an Altium PCB region is a **cutout contour within a
region polygon**. Regions in Altium are geometric polygons that can have:

1. **One main contour** (the outer boundary)
2. **Zero or more hole contours** (inner cutouts subtracted from the main contour)

This is the standard concept of a polygon with holes, as used in computational
geometry. The region is the area enclosed by the main contour minus the areas
enclosed by any hole contours.

### In-Memory Data Model

The region data model uses `IPCB_GeometricPolygon` as its core geometric
representation:

```
IPCB_Region (IPCB_Primitive)
  |
  +-- GetGeometricPolygon() -> IPCB_GeometricPolygon
  +-- GetMainContour() -> IPCB_Contour
  +-- GetHoleCount() -> int
  +-- GetHole(i) -> IPCB_Contour
  +-- SetOutlineContour(contour)
  +-- SetGeometricPolygon(polygon)
  +-- AddHoleContour(contour)  // IPCB_Region2
  +-- IsSimpleRegion() -> bool
```

**IPCB_GeometricPolygon** contains multiple contours, each flagged as hole or not:

```
IPCB_GeometricPolygon
  +-- GetState_Count() -> int              // total number of contours
  +-- GetState_Contour(i) -> IPCB_Contour  // specific contour
  +-- GetState_IsHole(i) -> bool           // whether contour i is a hole
  +-- SetState_IsHole(i, bool)
  +-- AddContourIsHole(contour, isHole) -> IPCB_Contour
  +-- AddContour(contour) -> IPCB_Contour
  +-- GetState_Area() -> double
  +-- FixupSelfIntersections()
  +-- ArrangeContours()
```

**IPCB_Contour** is a simple vertex list:

```
IPCB_Contour
  +-- GetState_Count() -> int
  +-- GetState_PointX(i) -> int    // Coord units
  +-- GetState_PointY(i) -> int    // Coord units
  +-- SetState_PointX(i, value)
  +-- SetState_PointY(i, value)
  +-- AddPoint(x, y)
  +-- InsertPoint(index, x, y)
  +-- DeletePoint(index)
  +-- GetState_Area() -> double
  +-- IsCW() -> bool               // winding direction
```

### Relationship to the Region Binary Record

The standard Regions6 binary record format already stores the region outline as
a vertex list. However, that format only stores the **main contour** (the shape
edges). The `RegionHoles` sidecar stream stores the **additional hole contours**
that cannot be represented in the base region binary record.

This is consistent with Altium's sidecar pattern: supplementary data that was
added in later format versions gets its own CFB stream rather than changing the
existing binary record format.

### Shape vs. Contour Model

IPCB_Region2 adds shape-based editing via `IPCB_RegionShape` / `IPCB_RegionShape2`:
- Shape edges (line segments, arcs) define the outer boundary
- `UpdateContourFromShape(argConserveExistingHoles: bool)` converts shape edges to contour points
- When `argConserveExistingHoles` is true, existing hole contours are preserved
  during shape updates -- this is the normal mode in the UI

### Region Kinds

Regions have a `TRegionKind` enum:
- `eRegionKind_BoardCutout` -- always on MultiLayer
- `eRegionKind_Cavity` -- on mechanical layers, has `CavityHeight` property
- `eRegionKind_NamedRegion` -- not exposed in UI
- Other region kinds for general copper/keepout regions

### Expected Binary Format

Since no test files contain this stream, the exact binary format is not confirmed
by empirical analysis. Based on the data model, the expected format is:

```
RegionHoles stream:
  Uses standard block-encoded format (4-byte header per block)

  Likely structure per region:
    u16  region_index      // index into Regions6 section
    u16  hole_count        // number of hole contours
    For each hole:
      u32  vertex_count    // number of vertices in this hole contour
      For each vertex:
        i32  x             // Coord units
        i32  y             // Coord units
```

**WARNING**: This format is speculative based on code analysis. Must be verified
against actual file data when fixtures with region holes become available.

### When RegionHoles Exist

Regions with holes are created when:
- A board cutout is placed inside a copper region
- The polygon pour engine creates regions with internal voids
- Importing from other EDA tools (e.g., Allegro) that have regions with holes
- Using the `AddHoleContour()` API

---

## ViaInstancing

### Overview

ViaInstancing (also called "Via Instancing" or `Section_ViaInstance`) is part of
Altium's **Pad/Via Template** system. It enables vias to be linked to reusable
templates, so that design-wide changes to via parameters can be made in one place.

This is gated behind the `PCB.ViaInstancing` feature flag (from
`cOptionPCBViaInstancing` in `RT_FeatureNames.Consts`).

### Pad/Via Template System

The template system uses these interfaces:

```
IPCB_PadViaTemplate (base)
  +-- GetState_ObjectID() -> TObjectId
  +-- GetState_LibraryID() -> string
  +-- GetState_TemplateID() -> string
  +-- GetState_RevisionID() -> string
  +-- GetState_TemplateName() -> string
  +-- GetState_TemplateDescription() -> string
  +-- GetState_Internal() -> bool
  +-- GetState_RemoveUnused() -> bool
  +-- GetState_PrimLinkCount() -> int
  +-- GetFullTemplateID() -> string
  +-- HasLocalChanges(primitive) -> bool
  +-- CreateTemplateLink() -> IPCB_PadViaTemplateLink
  +-- CreatePrimitive() -> IPCB_Primitive
  +-- Export_ToParameters(params, units, prefix)
  +-- Import_FromParameters(params, units, prefix)
  +-- GetState_Hash() -> string
  +-- UpdateHash()
  +-- IncreasePrimLinkCounter() / DecreasePrimLinkCounter()

IPCB_ViaTemplate : IPCB_PadViaTemplate
  +-- GetState_HoleSize() -> int
  +-- GetState_Mode() -> TPadMode
  +-- GetState_IsTenting_Top/Bottom() -> bool
  +-- GetState_ManualSolderMask() -> bool
  +-- GetState_SolderMaskExpansion() -> int
  +-- GetState_SolderMaskBottomExpansion() -> int
  +-- GetState_SolderMaskHoleEdge() -> bool
  +-- GetState_UseSeparateExpansions() -> bool
  +-- GetState_ViaStructure() -> IPCB_ViaStructure
  +-- GetState_StackData(index) -> IPCB_ViaTemplateStackData
  +-- StackDataCount() -> int
  +-- AddStackData(data) -> int
  +-- ClearStackData()
  +-- CreateDefaultStackData() -> IPCB_ViaTemplateStackData
```

### Via Template Stack Data

Each template can have per-layer size overrides via `IPCB_ViaTemplateStackData`:

```
IPCB_ViaTemplateStackData
  +-- GetState_Diameter() -> int       // via diameter on this layer
  +-- GetState_Layer() -> IV7_Layer    // which layer
  +-- GetState_ThermalRelief() -> IPadViaThermalReliefData
```

### Template Linking

Vias link to templates via `IPCB_PadViaTemplateLink`:

```
IPCB_PadViaTemplateLink
  +-- GetState_LibraryID() -> string
  +-- GetState_TemplateID() -> string
  +-- GetState_RevisionID() -> string
  +-- GetFullTemplateID() -> string
  +-- CopyTo(dest)
  +-- Clear()
```

On the via primitive side, `IPCB_Via` has:
- `GetState_TemplateLink() -> IPCB_PadViaTemplateLink`

And `IPCB_Via2` adds:
- `GetProperty_PatternId() -> int` / `SetProperty_PatternId(value)`

The `PatternId` property on vias is likely the cross-reference into the
`ViaInstancing` section data.

### Via Structure

Vias can have structural attributes via `IPCB_ViaStructure`:
- Structure type (via `TViaStructureType`)
- Features per side (tenting, covering, plugging, filling, capping)
- Material specifications

Related types:
- `TViaStructureType` -- the type of via structure
- `TViaStructureFeatureType` -- feature types (tenting, covering, etc.)
- `TViaStructureFeatureSide` -- which side(s) a feature applies to

### Pad/Via Library Management

Templates are organized into libraries:

```
IPCB_PadViaLibraryManager
  +-- GetLibraryName(board, template) -> string
  +-- GetLinkedToLibrary(primitive) -> bool

IPCB_PadViaLibrary
  +-- (manages collections of templates)

IPCB_PadViaLibraryDocument
  +-- (document-level library storage)
```

The board interface provides:
- `LinkToTemplate(primitive, template)` -- links a via to a template
- `UnlinkFromTemplate(primitive)` -- removes template link

### Expected Binary Format

Since no test files contain this stream, the exact binary format is not confirmed.
Based on the data model, the expected format is:

```
ViaInstancing stream:
  Uses standard block-encoded format (4-byte header per block)

  Header block (text params):
    |COUNT=N|  // number of via templates

  For each template:
    Text block with template parameters:
      |LIBRARYID=...|
      |TEMPLATEID=...|
      |REVISIONID=...|
      |TEMPLATENAME=...|
      |TEMPLATEDESCRIPTION=...|
      |INTERNAL=TRUE/FALSE|
      |REMOVEUNUSED=TRUE/FALSE|
      |HOLESIZE=...|
      |MODE=...|
      |ISTENTING_TOP=TRUE/FALSE|
      |ISTENTING_BOTTOM=TRUE/FALSE|
      |SOLDERMASKEXPANSION=...|
      |USESEPARATEEXPANSIONS=TRUE/FALSE|
      |SOLDERMASKBOTTOMEXPANSION=...|
      |SOLDERMASKHOLEEDGE=TRUE/FALSE|
      |MANUALSOLDERASK=TRUE/FALSE|
      |HASH=...|
      ... (stack data, via structure data)
```

**WARNING**: This format is speculative. The Delphi-side serialization likely uses
`Export_ToParameters` / `Import_FromParameters` on IPCB_PadViaTemplate, which
suggests a text-parameter block format. Must be verified against actual file data.

### When ViaInstancing Exists

Via instancing data appears when:
- The `PCB.ViaInstancing` feature is enabled
- Pad/Via templates have been created or imported
- Vias are linked to templates
- The design uses the Pad/Via Library feature

---

## Relationship to Other Sections

### RegionHoles relates to:
- **Regions6** -- the base region binary records (provides main contour)
- **SplitPlaneRegions6** -- split plane regions may also have holes
- **Polygons6** -- polygon pours generate regions (with potential holes)

### ViaInstancing relates to:
- **Vias6** -- the base via binary records (linked via PatternId)
- **PadViaCacheLibraryLinks** -- cache section for pad/via library links
- **CounterHolesPresetsSection** -- counter-hole parameters on vias

---

## Implementation Status

Neither section exists in our current test fixtures. Implementation should be
deferred until:
1. Test files containing these streams are obtained
2. The exact binary format can be confirmed empirically

For now, the parser should return a hard error if either stream is encountered,
with the context: "RegionHoles/ViaInstancing stream found but not yet implemented".
