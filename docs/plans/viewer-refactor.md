# AutoPCB Viewer Refactor: Spec-Centric IR-Based Rendering

## Overview

The viewer renders exclusively from **PcbIr** — the typed intermediate representation
extracted from PcbDoc files via the spec pipeline. The viewer never imports
`altium-format` or touches `PcbDoc` directly.

### Architecture

```
pcbdoc-spec file
      |
      +--parse/compile--> PcbDocSpec
      |
      +--load_ir_from_spec()--> PcbIr  (via autopcb-ir::spec_bridge)
            internally:
              PcbDocSpec --> open target PcbDoc
                        --> apply_spec_pcbdoc (mutations)
                        --> PcbIr::extract (full extraction)
                        --> apply placement at: overrides
                        --> PcbIr
                               |
                    +----------+----------+
                    |                     |
              Board Renderer        Overlay Renderer
              (PcbIr primitives)    (PcbDocSpec intent)
                    |                     |
                    +----------+----------+
                               |
                          egui / wgpu
```

### Design Principles

1. **PcbDoc is an implementation detail** — the spec pipeline handles it internally
2. **PcbIr is the single source of truth** for all rendering and interaction
3. **The IR must be complete** — every PcbDoc primitive needed for rendering must
   have an IR representation (no bypassing the IR to read from PcbDocBoard)
4. **Spec overlays are separate** — intent visualizations (constraints, groups,
   routes) are a distinct render pass on top of the board geometry


## Wave 1: IR Expansion — Add Missing Rendering Primitives

**Status: DONE** — All types added, extraction implemented.

The IR was router-focused and missing rendering primitives. These have been added:

| New IR Type | Source (PcbDocBoard) | Purpose |
|-------------|---------------------|---------|
| `IrArc` | `board.arcs` | Copper arc segments |
| `IrText` | `board.texts` | PCB text objects (designators, comments, free text) |
| `IrRegion` | `board.regions` | Copper pours, solder mask, paste mask, cutouts |
| `IrComponentBody` | `board.component_bodies` | 3D component envelopes |
| `IrDimension` | `board.dimensions` | Dimension annotations |

New handle types: `TextId`, `RegionId`, `ComponentBodyId`, `DimensionId`.

`FreeCopperGeometry` now includes `arcs: Vec<IrArc>`.


## Wave 2: Viewer Rendering — Render the New IR Types

Parallel milestones after Wave 1.

### M1: Arc Rendering

- Tessellate `IrArc` to line segments at ~2° intervals
- Render as polyline with arc width
- Respect layer visibility and net highlighting

### M2: Text Rendering

- Render `IrText` at correct position, height, rotation
- Handle `is_designator` and `is_comment` (substitute component strings)
- Handle `is_mirrored`
- Use egui's built-in font scaled to text height

### M3: Region Rendering with Holes

- Triangulate `IrRegion.outline` with `IrRegion.holes` using `earcutr`
- Color by `IrRegionKind` (copper → layer color, mask → mask color)
- Render keepout regions with distinctive styling

### M4: Component Body Rendering

- Render `IrComponentBody.outline` as filled polygon
- Use `body_color` RGBA from the IR
- Highlight bodies when parent component is selected

### M5: Dimension Rendering

- Render dimension text at `text_position`
- Render dimension lines between reference points (when API exposes them)

### M6: Enhanced Pad Rendering

- Use padstack data for per-layer shape (when extracted to IR)
- Render RoundRect pads with corner radius
- Render thermal relief patterns
- Render solder mask / paste mask expansion on mask layers


## Wave 3: Spec Intent Overlays

Parallel after Wave 1. Renders information from `PcbDocSpec` that has no
equivalent in the Altium file format.

### M7: Placement Constraint Visualization

- Draw directional arrows between constrained components
- Draw placement region rectangles
- Toggle in sidebar

### M8: Placement Group Visualization

- Draw convex hull / bounding box envelopes around grouped components
- Distinct colors per group

### M9: Route Solution Overlay

- Load `RouteSolution` from `.routes` file
- Render route traces and vias with distinctive color
- Show unrouted nets as ratsnest in warning color
- Playback through routing iterations


## Wave 4: Interaction and Polish

Parallel after Wave 2.

### M10: Enhanced Hit Testing

- Per-primitive hit testing (pads, tracks, vias, text)
- Priority: pad > via > track > component
- Tooltip with primitive-specific info

### M11: Cursor Coordinates + Grid

- Display cursor position in mm
- Render grid lines using board settings
- Adaptive grid density with zoom

### M12: Search + Zoom-to-Component

- Text filter for component and net lists
- Double-click to zoom to selection

### M13: MSAA + Render Quality

- Enable multisampling (4x)
- Layer z-ordering (bottom → top in physical stack order)

### M14: 3D View Improvements

- Oriented track quads (not AABB)
- Cylindrical vias
- Arc extrusion
- Component body heights from IR
- Board cutouts in 3D mesh


## Dependency Graph

```
Wave 1 (IR Expansion — DONE):
  Add IrArc, IrText, IrRegion, IrComponentBody, IrDimension
  Add spec_bridge::load_ir_from_spec()

Wave 2 (Viewer Rendering — parallel):
  M1  Arc rendering
  M2  Text rendering
  M3  Region rendering with holes
  M4  Component body rendering
  M5  Dimension rendering
  M6  Enhanced pad rendering

Wave 3 (Spec Overlays — parallel):
  M7  Placement constraint visualization
  M8  Placement group visualization
  M9  Route solution overlay

Wave 4 (Interaction — parallel, after Wave 2):
  M10 Enhanced hit testing
  M11 Cursor coordinates + grid
  M12 Search + zoom-to-component
  M13 MSAA + render quality
  M14 3D view improvements (after M1, M2)
```
