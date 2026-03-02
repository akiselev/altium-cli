# Phase 6: PcbDoc Rendering

## Goal

Add SVG/PNG rendering for PcbDoc boards via the high-level API.

## Prerequisites

Phase 2 (read path) must be complete.

## Overview

PcbDoc rendering is significantly more complex than PcbLib footprint rendering
because:

1. **Multi-layer**: Board has 70+ layers; rendering must filter/composite
2. **Scale**: Full boards have thousands of primitives
3. **Net highlighting**: Useful to color-code by net
4. **Component grouping**: Show component outlines and designators
5. **Board outline**: Must render the PCB perimeter

## Implementation Strategy

### Reuse from PcbLib Rendering

The existing `altium-format-render-svg` already renders all 8 PCB primitive types
for PcbLib footprints. PcbDoc rendering reuses these primitive renderers but adds:

- Layer filtering (render specific layers or layer sets)
- Board outline rendering
- Multi-layer compositing (top copper + top overlay + top mask, etc.)
- Net-based coloring

### New Types

```rust
pub struct PcbDocRenderOptions {
    pub layers: Vec<LayerRef>,       // Which layers to render
    pub highlight_nets: Vec<String>, // Nets to highlight
    pub show_designators: bool,
    pub show_board_outline: bool,
    pub scale: f64,                  // px per mil (default 4.0)
}
```

### Rendering Pipeline

1. Extract `PcbDocBoard` via `board()`
2. Filter primitives by requested layers
3. Sort by layer order (back to front)
4. Render board outline (Region with board_cutout kind)
5. Render primitives per layer, applying net colors
6. Render component designator labels
7. Output SVG with layer groups

### CLI Integration

```bash
altium render board.PcbDoc --output board.svg
altium render board.PcbDoc --output board.png --layers top,top_overlay
altium render board.PcbDoc --output board.svg --highlight-net VCC
```

## Estimated Scope

- ~400-600 lines in render-svg crate
- ~50 lines in CLI integration
- This phase is largely optional for the initial API release

## Deferred

- Layer stack visualization (cross-section view)
- 3D model rendering
- Drill drawing generation
- Gerber-style layer-by-layer output
