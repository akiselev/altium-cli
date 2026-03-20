# AutoPCB Viewer Refactor: Spec-Centric Full PCB Viewer

## Overview

The autopcb-viewer is currently a placement-debugging tool that renders from
`PcbIr` — a lossy intermediate representation designed for the placement and
routing solvers. It lacks arcs, text, regions, silkscreen, component bodies,
dimensions, and many rendering primitives needed for a real PCB viewer.

This plan refactors the viewer to render from **two primary data sources**:

1. **`PcbDocBoard`** (from `altium-format`) — fully typed API with all PCB
   primitives: tracks, arcs, vias, pads (with padstacks), fills, text, regions,
   component bodies, dimensions, polygons, board geometry, layer stack.

2. **`PcbDocSpec`** (from `altium-format-spec`) — the compiled spec model,
   providing intent overlays (placement constraints, groups, clearances, routing
   solutions) that go beyond what Altium's format can represent.

The `PcbIr` is removed from the viewer's rendering path entirely. It remains
in-use by the placement and routing solvers but is not a viewer concern.


## Architecture Decision

### Why not PcbIr?

| Concern | PcbIr | PcbDocBoard |
|---------|-------|-------------|
| Arcs | Not extracted | `Arc` — typed with center, radius, angles |
| Text objects | Not extracted | `Text` — typed with font, size, rotation, designator flag |
| Regions | Not extracted | `Region` — typed with kind, outline, holes |
| Component bodies | Not extracted | `ComponentBody` — outline, 3D height, color |
| Dimensions | Not extracted | `Dimension` — typed with kind, text position |
| Pad stacks | Single shape per pad | `PadStack` — per-layer shapes, corner radius |
| Solder/paste mask | Not extracted | `Region` with `RegionKind::SolderMask` / `PasteMask` |
| Silkscreen | Not extracted | Primitives on overlay layers via `primitives_on_layer()` |
| Rotated fills | Not supported | `Fill` — typed with rotation |
| Layer queries | Manual iteration | `primitives_on_layer()`, `tracks_for_net()`, etc. |

The IR intentionally discards rendering data because it was designed for solver
cost functions. Extending it to carry all rendering data would make it a
duplicate of `PcbDocBoard`.

### Why PcbDocSpec as an overlay?

The spec language will grow to express:
- Placement constraints (left_of, right_of, above, below, near, groups)
- Placement clearance zones
- Routing solutions (via `autopcb-routes::RouteSolution`)
- DRC rules and violation overlays
- Design intent annotations
- Information that has no Altium format equivalent

These need their own visual layers rendered on top of the board geometry.

### Data flow

```
pcbdoc-spec file
      |
      +--compile--> PcbDocSpec (intent, overrides, placement, routing)
      |
      +--target:--> PcbDoc::open() --> .board() --> PcbDocBoard (full geometry)
                         |
                   +-----+-----+
                   | ViewState  |  <-- holds PcbDocBoard + PcbDocSpec + RouteSolution
                   +-----+-----+
                         |
              +----------+----------+
              |                     |
        Board Renderer        Overlay Renderer
        (PcbDocBoard)         (PcbDocSpec intent)
              |                     |
              +----------+----------+
                         |
                    egui / wgpu
```


## Planning Context

### Decision Log

| Decision | Reasoning |
|----------|-----------|
| Replace PcbIr with PcbDocBoard, don't wrap it | PcbDocBoard already IS the typed view model — wrapping it adds indirection without value |
| Keep PcbIr for placement/routing only | IR is optimized for solver cost functions, not rendering — different concerns |
| Spec overlays as a separate render pass | Intent visualization changes independently of board geometry; keeps renderers simple |
| Add polygon triangulation early (M1) | Concave polygons affect board outline, regions, and copper pours — must fix before adding new primitive types |
| Use `earcutr` for triangulation | Pure Rust, earcut algorithm, handles holes — needed for `Region` primitives |
| Phase 3D view improvements into later milestones | 2D correctness is more important than 3D fidelity — prioritize getting all primitives visible first |
| Add `RouteSolution` visualization alongside board | `autopcb-routes` crate already defines serializable route solutions — viewer should load them |
| Keep playback system for backward compatibility | Existing placement iteration playback still useful; extend with routing iteration playback |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Extend PcbIr with all rendering data | Would duplicate PcbDocBoard; IR's purpose is solver optimization, not rendering |
| Build a new "ViewIR" intermediate | PcbDocBoard already fulfills this role — adding another layer of indirection creates drift |
| Render directly from PcbDocSpec primitives | Spec primitives are untyped (`IndexMap<String, Value>`) — they're designed to flow through the executor, not to be rendered directly |
| egui-only rendering (drop wgpu 3D) | 3D view is useful for board inspection; keep it but fix correctness before adding features |

### Constraints & Assumptions

- `PcbDocBoard` is the primary public API from `altium-format` for reading PcbDoc data
- All `PcbDocBoard` fields are `pub` — direct field access, no accessor ceremony
- `PcbDocBoard` provides query methods: `primitives_on_layer()`, `tracks_for_net()`, `pads_for_net()`, `pads_for_component()`, `bodies_for_component()`
- `LayerRef` has classification helpers: `is_signal()`, `is_copper()`, `is_overlay()`, `is_solder_mask()`, `is_paste_mask()`, `is_mechanical()`
- Coordinate types: `Coord` (internal units, 10000 = 1 mil), `CoordPoint` — need mm conversion helpers
- `autopcb-routes::RouteSolution` is serializable and has iteration snapshots
- The viewer currently has ~2400 lines of Rust across 6 source files


### Known Risks

| Risk | Mitigation | Milestone |
|------|-----------|-----------|
| PcbDocBoard coordinate system (Coord) differs from viewer (mm as f64) | Add conversion helpers in a shared module; Coord → mm is `value as f64 / 10_000.0 / 1_000.0 * 25.4` | Wave 1 M1 |
| Non-convex polygon triangulation may be slow for large pour fills | Use earcutr which is O(n log n); cache triangulated meshes per primitive | Wave 1 M1 |
| Text rendering quality with egui's built-in fonts | Start with egui's monospace font scaled to match Altium text height; upgrade to custom font rendering later if needed | Wave 2 M5 |
| PcbDocBoard may not have all primitives for all test files | Use fail-fast approach — render what exists, skip gracefully on missing data, don't panic | All |
| Hot-reload with PcbDocBoard is heavier than with PcbIr | PcbDoc::open() + .board() may take 100-500ms for large files — debounce already exists, may need to increase window | Wave 1 M2 |
| 3D view mesh generation is O(primitives) and may be slow | Defer 3D mesh rebuild to background; show stale mesh until rebuild completes | Wave 4 |

---

## Wave 1: Foundation — Replace IR with PcbDocBoard

Four milestones, strict dependency order: M1 → M2 → M3 → M4.
Combined effect: viewer renders from typed PcbDocBoard primitives with correct
polygon handling and all existing features preserved.

### M1: Coordinate Conversion + Polygon Triangulation

**Problem:** Two fundamental gaps block all subsequent work:
1. PcbDocBoard uses `Coord` / `CoordPoint`; viewer uses `f64` mm and egui `Pos2`
2. `Shape::convex_polygon` is used everywhere — breaks on any non-rectangular board or copper pour

**Files:**
- `crates/autopcb-viewer/Cargo.toml` — add `earcutr` dependency
- `crates/autopcb-viewer/src/renderer.rs` — add coord conversion helpers + triangulation
- `crates/autopcb-viewer/src/colors.rs` — no changes needed

**Implementation:**

1. Add coordinate conversion module (or section in renderer.rs):
   ```rust
   use altium_format_types::{Coord, CoordPoint};

   fn coord_to_mm(c: Coord) -> f64 {
       c.to_mils() / 1000.0 * 25.4
   }

   fn coord_point_to_pos2(p: &CoordPoint) -> Pos2 {
       Pos2::new(coord_to_mm(p.x) as f32, -(coord_to_mm(p.y) as f32))
   }
   ```

2. Add polygon triangulation helper:
   ```rust
   fn triangulate_polygon(outline: &[Pos2], holes: &[Vec<Pos2>]) -> Vec<Shape> {
       // Use earcutr to triangulate; return list of triangle Shapes
   }
   ```

3. Replace all `Shape::convex_polygon` calls in the existing renderer with
   triangulated polygon rendering. This fixes board outline, copper pours,
   and keepout zones immediately.

**Acceptance Criteria:**
- Non-convex board outlines render correctly (L-shaped boards, etc.)
- `cargo check -p autopcb-viewer` passes
- Existing board rendering is visually equivalent for convex boards

**Tests:**
- Unit test: triangulate a simple L-shape → correct triangle count
- Visual: load a non-rectangular board and verify outline

---

### M2: ViewState — Replace PcbIr with PcbDocBoard

**Problem:** The entire app struct and rendering pipeline is built around
`Arc<Mutex<PcbIr>>`. Need to replace with `PcbDocBoard` (plus spec/route data).

**Files:**
- `crates/autopcb-viewer/src/main.rs` — change loading pipeline
- `crates/autopcb-viewer/src/app.rs` — replace `PcbIr` with `ViewState`
- `crates/autopcb-viewer/src/renderer.rs` — rewrite `render_board()` for PcbDocBoard
- `crates/autopcb-viewer/src/interaction.rs` — update hit testing for PcbDocBoard types
- `crates/autopcb-viewer/Cargo.toml` — remove `autopcb-ir` dep, add `altium-format` dep

**Implementation:**

1. Define `ViewState`:
   ```rust
   pub struct ViewState {
       pub board: PcbDocBoard,
       pub spec: Option<PcbDocSpec>,
       pub route_solution: Option<RouteSolution>,
   }
   ```

2. Change `main.rs` loading:
   ```
   Before: PcbDoc::open() → .board() → PcbIr::extract() → render IR
   After:  PcbDoc::open() → .board() → render PcbDocBoard directly
   ```

3. Spec handling changes:
   - Spec position overrides: apply directly to `PcbDocBoard.components` (mutate
     `location` and `rotation` fields) instead of going through IR
   - Spec is stored in `ViewState.spec` for overlay rendering in later milestones

4. Rewrite `render_board()` signature:
   ```rust
   pub fn render_board(
       painter: &Painter,
       board: &PcbDocBoard,
       opts: &RenderOptions,
       layer_states: &[LayerRenderState],
       selected: Option<&str>,     // designator
       hovered: Option<&str>,      // designator
       selected_net: Option<&str>, // net name
   )
   ```

5. Rewrite `interaction.rs` to use `PcbDocBoard` component/pad types:
   - Hit testing: iterate `board.components`, check bounds from component pads
   - Net lookup: use `board.pads_for_component()` instead of IR pad iteration

6. Layer state collection: use `board.settings.layer_stack` instead of IR layer stack

7. Remove `autopcb-ir` and `autopcb-placement` dependencies from Cargo.toml.
   Keep `autopcb-routes` for route solution loading.

**Breaking changes:**
- Playback system (PlacementIterationSnapshot) needs adaptation — it references
  IR types. Keep compatibility by converting snapshot positions to component
  location mutations on PcbDocBoard.

**Acceptance Criteria:**
- Viewer opens a PcbDoc file and renders board outline, components, pads, tracks,
  vias, fills, polygons, keepouts — same visual output as before
- Spec position overrides still work
- Hot-reload still works
- No `autopcb-ir` import in the viewer crate

---

### M3: Render All Existing PcbDocBoard Primitives

**Problem:** After M2 swaps the data source, need to actually render all the
primitive types that PcbDocBoard exposes but the IR didn't have.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — add rendering for each primitive type

**Implementation — add renderers for each primitive type:**

1. **Tracks** (already existed in IR form):
   ```rust
   for track in &board.tracks {
       let layer_name = track.layer.display_name().unwrap_or("?");
       if !layer_visible(layer_states, layer_name) { continue; }
       let color = net_alpha(layer_color(layer_states, layer_name), ...);
       let start = coord_point_to_pos2(&track.start);
       let end = coord_point_to_pos2(&track.end);
       let width = coord_to_mm(track.width) as f32;
       painter.line_segment([start, end], Stroke::new(width, color));
   }
   ```

2. **Arcs** (NEW — not in IR):
   - Tessellate arc to line segments at ~2° intervals
   - Render as polyline with track width
   - Respect layer visibility and net highlighting

3. **Vias** (already existed):
   - Render from `board.vias` instead of `ir.free_copper.vias`

4. **Pads** (enhanced — padstack support):
   - Use `pad.stack` for per-layer shape when `pad.pad_mode != Simple`
   - Render `PadShape::RoundRect` with corner radius (was falling through to rectangle)
   - Render thermal relief pattern (crossed lines through pad) when `pad.plane_connection == Relief`

5. **Fills** (enhanced — rotation support):
   - Apply `fill.rotation` to corners before rendering

6. **Text** (NEW — not in IR):
   - Render at `text.location` with `text.height` controlling font size
   - Apply `text.rotation`
   - Respect `text.is_mirrored`
   - Use `text.text` content; for designator text, substitute component designator

7. **Regions** (NEW — not in IR):
   - Triangulate `region.outline` with `region.holes`
   - Color by `region.kind`:
     - `CopperPour` → layer color
     - `SolderMask` → solder mask color
     - `PasteMask` → paste color
     - Keepout → keepout color/hatching

8. **Component bodies** (NEW — not in IR):
   - Render `component_body.outline` as filled polygon
   - Color by `component_body.body_color_3d` (convert from Altium Color to egui Color32)

9. **Dimensions** (NEW — not in IR):
   - Render dimension lines between reference points
   - Render dimension text at `text_x`, `text_y`

10. **Board geometry** (enhanced — arc-preserving):
    - Use `board.settings.geometry.outline` which preserves arc segments
    - Tessellate arcs for rendering
    - Render cutouts with triangulated holes

**Acceptance Criteria:**
- All primitive types visible in the viewer
- Arcs render as smooth curves
- Text objects visible with correct size and rotation
- Regions render with holes correctly cut out
- RoundRect pads render with rounded corners

---

### M4: Layer System Overhaul

**Problem:** The current layer system uses string matching. PcbDocBoard uses
`LayerRef` with proper classification. Need to align.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — use LayerRef for layer identification
- `crates/autopcb-viewer/src/colors.rs` — add LayerRef-based color mapping
- `crates/autopcb-viewer/src/app.rs` — update layer strip

**Implementation:**

1. Change `LayerRenderState` to use `LayerRef` as the layer identifier:
   ```rust
   pub struct LayerRenderState {
       pub layer: LayerRef,
       pub name: String,       // display_name for UI
       pub visible: bool,
       pub color: Color32,
   }
   ```

2. Build layer states from `board.settings.layer_stack`:
   - All copper layers from the physical stack
   - Overlay layers (TopOverlay, BottomOverlay)
   - Solder mask layers
   - Paste mask layers
   - Mechanical layers (only those with primitives)

3. Use `layer.is_signal()`, `is_overlay()`, `is_solder_mask()`, etc. for
   color defaults instead of string matching.

4. Layer ordering: use `layer_stack.physical_order()` for copper layers;
   fixed order for non-copper layers.

5. Add layer categories in the UI strip:
   - Signal (copper)
   - Silkscreen (overlay)
   - Mask (solder + paste)
   - Mechanical

**Acceptance Criteria:**
- Layer strip shows all layers from the board's actual stack
- Layer visibility correctly filters all primitive types
- Default colors match standard PCB viewer conventions

---

## Wave 2: New Rendering Features

Five milestones, mostly parallel after Wave 1.

### M5: Text Rendering

**Problem:** Currently only tiny fixed-size designator labels. Need proper
text rendering for all text objects.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — text rendering functions

**Implementation:**

1. For each `Text` in `board.texts`:
   - Convert `text.height` from Coord to mm → scale to font size
   - Apply `text.rotation` to the painter transform
   - Handle `text.is_mirrored` (flip X before drawing)
   - Render on the correct layer (use `text.layer`)

2. Special cases:
   - `text.is_designator == true`: show component designator string
   - `text.is_comment == true`: show component comment/value string
   - Regular text: show `text.text` content

3. Font handling:
   - Use egui's built-in monospace font as baseline
   - Scale font size: `text.height` in mm maps to egui font units via scene transform
   - `text.width` controls stroke width (start with simple filled text)

**Acceptance Criteria:**
- Designator text visible at correct size on correct layer
- Comment text visible
- Free text objects visible
- Rotation and mirroring work correctly

---

### M6: Arc Rendering

**Problem:** No arc rendering exists. Arcs appear in tracks, board outline,
keepout zones, and component graphics.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — arc tessellation + rendering

**Implementation:**

1. Arc tessellation function:
   ```rust
   fn tessellate_arc(
       center: Pos2,
       radius_mm: f64,
       start_angle_deg: f64,
       end_angle_deg: f64,
       step_deg: f64, // typically 2.0
   ) -> Vec<Pos2>
   ```

2. Render each `Arc` in `board.arcs`:
   - Tessellate to line segments
   - Draw as polyline with `arc.width` stroke width
   - Respect layer visibility and net highlighting

3. Board outline arcs:
   - `BoardGeometry.outline` has `ContourSegment::Arc` variants
   - Tessellate arcs inline with line segments when building outline polygon

**Acceptance Criteria:**
- Copper arcs visible as smooth curves
- Board outline arcs render correctly (rounded board corners)
- Arc width matches track width rendering

---

### M7: Region Rendering with Holes

**Problem:** `Region` primitives (copper pours, solder mask openings, paste
mask, board cutouts) are not rendered. They have complex outlines with holes.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — region rendering with triangulation

**Implementation:**

1. For each `Region` in `board.regions`:
   - Skip if layer not visible
   - Triangulate `region.outline` with `region.holes` using earcutr
   - Color based on `region.kind`:
     - `CopperPour` → layer copper color, ~80% alpha
     - `SolderMask` → solder mask color, ~60% alpha
     - `PasteMask` → paste color, ~60% alpha
   - Apply keepout styling if `region.is_keepout`

2. Polygon copper pour regions (`board.regions_for_polygon(name)`):
   - Render in correct z-order based on `polygon.pour_order`

**Acceptance Criteria:**
- Copper pour regions visible with correct layer coloring
- Holes in regions render correctly (cutouts visible)
- Solder mask openings visible when mask layer is enabled
- Keepout regions render with hatching or distinctive color

---

### M8: Enhanced Pad Rendering

**Problem:** Pads only render with simple shape. Need padstack, RoundRect,
thermal relief, and drill hole rendering.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — pad rendering overhaul

**Implementation:**

1. RoundRect rendering:
   - Use `pad.stack.top.corner_radius_pct` to compute corner radius
   - Draw as rounded rectangle (4 arcs + 4 lines, or use egui's `rect` with rounding)

2. Per-layer padstack:
   - When viewing top layer: use `pad.stack.top` shape
   - When viewing bottom layer: use `pad.stack.bot` shape
   - When viewing inner layer: check `pad.stack.inner_layers` for overrides, fall back to `pad.stack.mid`

3. Thermal relief indication:
   - When `pad.plane_connection == Relief`: draw thin crossed lines through pad center
   - Line width from `pad.relief_conductor_width`, entry count from `pad.relief_entries`

4. Solder mask / paste mask expansion:
   - When mask layers are visible: render expanded pad outlines on those layers
   - Expansion amount from `pad.solder_mask_expansion` / `pad.paste_mask_expansion`

5. Slot holes:
   - When `pad.stack.hole_shape == Rectangular`: render elongated drill hole
   - Use `pad.stack.slot_size` and `pad.stack.slot_rotation`

**Acceptance Criteria:**
- RoundRect pads render with visible corner rounding
- Through-hole pads show correct shape on each layer
- Thermal relief pattern visible on plane layers
- Mask layers show expanded pad openings

---

### M9: Component Body Rendering

**Problem:** Component bodies (3D envelopes / courtyard outlines) are not
rendered.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — component body rendering

**Implementation:**

1. For each `ComponentBody` in `board.component_bodies`:
   - Render `component_body.outline` as filled polygon on the component's layer
   - Color: convert `component_body.body_color_3d` (Altium Color = 0x00BBGGRR) to egui Color32
   - Apply `component_body.body_opacity_3d` as alpha

2. Use `board.bodies_for_component(designator)` to associate bodies with components
   for selection highlighting.

**Acceptance Criteria:**
- Component courtyard outlines visible
- Component body color matches Altium's 3D color
- Bodies highlight when parent component is selected

---

## Wave 3: Spec Intent Overlays

Three milestones, parallel after Wave 1. These render information from the
`PcbDocSpec` that has no equivalent in the Altium file format.

### M10: Placement Constraint Visualization

**Problem:** Placement constraints (left_of, right_of, above, below) are
invisible. The user can't see what constraints the spec imposes.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — add overlay rendering pass
- `crates/autopcb-viewer/src/app.rs` — add overlay toggle in sidebar

**Implementation:**

1. Add `RenderOptions.show_constraints: bool` toggle.

2. For each `PlacementConstraintSpec` in `spec.placement.constraints`:
   - Look up component positions from the board
   - Draw directional arrow from component A to component B
   - Label with constraint type and gap value
   - Color: distinctive constraint color (e.g., orange)

3. For each `PlacementPlaceSpec` with `region_rect`:
   - Draw the placement region as a dashed rectangle
   - Label with region name

4. For each `PlacementPlaceSpec` with `edge`:
   - Draw edge indicator on the specified board edge

5. For each `PlacementPlaceSpec` with `near`:
   - Draw proximity circle from reference component
   - Radius = `max_distance` if specified

**Acceptance Criteria:**
- Constraint arrows visible between constrained components
- Placement regions visible as dashed rectangles
- Toggle in sidebar to show/hide constraint overlays

---

### M11: Placement Group Visualization

**Problem:** Placement groups are invisible. The user can't see which
components are grouped together.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — group envelope rendering

**Implementation:**

1. Add `RenderOptions.show_groups: bool` toggle.

2. For each `PlacementGroupSpec`:
   - Find all member component positions
   - Compute convex hull or bounding box of group members
   - Draw semi-transparent colored envelope around the group
   - Label with group name

3. Use distinct colors for each group (cycle through a palette).

**Acceptance Criteria:**
- Group envelopes visible around grouped components
- Each group has a distinct color
- Group label visible

---

### M12: Route Solution Overlay

**Problem:** The `autopcb-routes` crate defines `RouteSolution` with trace
segments and vias, but the viewer can't display them.

**Files:**
- `crates/autopcb-viewer/src/renderer.rs` — route overlay rendering
- `crates/autopcb-viewer/src/app.rs` — route loading and playback
- `crates/autopcb-viewer/src/main.rs` — add `--routes` CLI flag
- `crates/autopcb-viewer/Cargo.toml` — add `autopcb-routes` dependency

**Implementation:**

1. Add `--routes <solution.json>` CLI flag to load a `RouteSolution`.

2. Store in `ViewState.route_solution`.

3. Render route traces:
   - For each `RoutedNet` in the solution:
     - Render `TraceSegment` as line with width
     - Render `RoutedVia` as circle with drill hole
   - Use distinctive color to distinguish from existing board copper (e.g., brighter, or with glow/outline)

4. Route iteration playback:
   - Extend playback system to handle `RoutingIterationSnapshot`
   - Show routed/unrouted/conflict counts in playback panel
   - Animate route evolution across PathFinder iterations

5. Unrouted net indicator:
   - For nets in `solution.unrouted`: draw ratsnest lines in red/warning color

6. Route metrics in sidebar:
   - Total length, via count, completion %, DRC violations

**Acceptance Criteria:**
- Route solution traces visible on correct layers
- Route vias visible
- Playback through routing iterations works
- Unrouted nets highlighted
- Metrics displayed in sidebar

---

## Wave 4: Interaction and Polish

Five milestones, mostly parallel after Waves 1-2.

### M13: Enhanced Hit Testing

**Problem:** Only component-level AABB hit testing exists. Can't click pads,
tracks, vias, or text.

**Files:**
- `crates/autopcb-viewer/src/interaction.rs` — add per-primitive hit testing

**Implementation:**

1. Pad hit testing: point-in-shape test for each visible pad
2. Track hit testing: point-to-segment distance < width/2
3. Via hit testing: point-to-center distance < radius
4. Priority order: pad > via > track > component (most specific wins)
5. On click: select the hit primitive and its net
6. Tooltip: show primitive details (net, layer, dimensions)

**Acceptance Criteria:**
- Clicking a pad selects it and highlights its net
- Clicking a track selects it and highlights its net
- Tooltip shows primitive-specific information

---

### M14: Cursor Coordinates + Grid

**Problem:** No coordinate readout, no grid display.

**Files:**
- `crates/autopcb-viewer/src/app.rs` — status bar and grid rendering
- `crates/autopcb-viewer/src/renderer.rs` — grid lines

**Implementation:**

1. Display cursor position in mm in the status bar
2. Render grid lines (using `board.settings.visible_grid_size`):
   - Thin lines at grid intervals
   - Only draw within visible viewport
   - Scale grid density with zoom level (skip lines when too dense)
3. Snap-grid indication (optional, lighter color)

**Acceptance Criteria:**
- Cursor mm coordinates shown in status bar
- Grid visible at appropriate zoom levels
- Grid spacing matches board settings

---

### M15: Search and Zoom-to-Component

**Problem:** Component and net lists have no search. Selecting doesn't zoom.

**Files:**
- `crates/autopcb-viewer/src/app.rs` — search fields and zoom behavior

**Implementation:**

1. Add text filter field above component list:
   - Filter by designator or pattern (case-insensitive substring match)
2. Add text filter field above net list:
   - Filter by net name
3. On double-click (or Enter) in component list:
   - Zoom `scene_rect` to fit the component with margin
4. On double-click in net list:
   - Zoom to fit all pads in the net

**Acceptance Criteria:**
- Typing in search field filters the list
- Double-click zooms to the selected item

---

### M16: MSAA + Render Quality

**Problem:** No anti-aliasing (`multisampling: 0`), jagged edges everywhere.

**Files:**
- `crates/autopcb-viewer/src/main.rs` — enable MSAA
- `crates/autopcb-viewer/src/view3d.rs` — update pipeline for MSAA

**Implementation:**

1. Set `multisampling: 4` in `NativeOptions`
2. Update 3D pipeline `MultisampleState.count` to match
3. Add layer z-ordering in 2D: render layers back-to-front based on physical
   stack order (bottom layer first, top layer last)

**Acceptance Criteria:**
- Edges are visibly smoother
- 2D layer ordering respects physical stack (top copper on top of bottom copper)

---

### M17: 3D View Improvements

**Problem:** 3D view uses box approximations for everything. Tracks are AABB,
vias are square, no arcs or text.

**Files:**
- `crates/autopcb-viewer/src/view3d.rs` — mesh generation improvements

**Implementation:**

1. **Tracks**: Generate oriented quads (proper rotated slabs) instead of AABB
2. **Vias**: Generate octagonal prisms or 16-sided cylinders
3. **Arcs**: Tessellate arcs to segments, generate extruded quads along arc path
4. **Component bodies**: Use `component_body.standoff_height` and
   `component_body.overall_height` for proper 3D extrusion
5. **Board cutouts**: Cut board substrate mesh at cutout locations
6. **FR4 texture**: Apply a subtle green pattern to board substrate instead of flat color

**Acceptance Criteria:**
- Tracks render as properly oriented copper strips
- Vias render as cylindrical shapes
- Component bodies have realistic heights
- Board cutouts visible in 3D

---

## Dependency Graph

```
Wave 1 (Foundation — sequential):
  M1 Coord conversion + triangulation
    -> M2 ViewState / replace PcbIr with PcbDocBoard
      -> M3 Render all PcbDocBoard primitives
        -> M4 Layer system overhaul

Wave 2 (New rendering — parallel, after M3):
  M5  Text rendering
  M6  Arc rendering
  M7  Region rendering with holes
  M8  Enhanced pad rendering (padstack, RoundRect, thermal)
  M9  Component body rendering

Wave 3 (Spec overlays — parallel, after M2):
  M10 Placement constraint visualization
  M11 Placement group visualization
  M12 Route solution overlay

Wave 4 (Interaction — parallel, after M3):
  M13 Enhanced hit testing
  M14 Cursor coordinates + grid
  M15 Search + zoom-to-component
  M16 MSAA + render quality
  M17 3D view improvements (after M5, M6)
```

## Verification Plan

### After Wave 1 (M1-M4 complete):
```bash
# 1. Open a PcbDoc directly
autopcb-viewer test.PcbDoc

# 2. Open via spec
autopcb-viewer test.pcbdoc-spec --target test.PcbDoc

# 3. Verify all existing features work:
#    - Board outline visible
#    - Components visible with designators
#    - Pads visible
#    - Tracks visible on correct layers
#    - Vias visible
#    - Polygons visible
#    - Layer strip functional
#    - Component/net selection works
#    - Hot-reload works (--watch)
#    - Screenshot works (--screenshot)
#    - Playback works (--playback)

# 4. Verify no PcbIr import:
grep -r "autopcb.ir" crates/autopcb-viewer/
# Expected: no matches
```

### After Wave 2 (M5-M9 complete):
```bash
# 5. Verify new primitives visible:
#    - Text objects on overlay layers
#    - Arcs on copper layers
#    - Regions (copper pours with holes)
#    - Component bodies
#    - RoundRect pads with visible rounding
#    - Solder mask layer shows expanded pads
```

### After Wave 3 (M10-M12 complete):
```bash
# 6. Verify spec overlays:
#    - Placement constraint arrows visible
#    - Group envelopes visible
#    - Route solution traces visible
autopcb-viewer board.pcbdoc-spec --target board.PcbDoc --routes solution.json
```

## Size Estimates

| Milestone | Files touched | Approximate scope |
|-----------|--------------|-------------------|
| M1 Coord + triangulation | 2 | ~100 lines new + earcutr dep |
| M2 ViewState refactor | 5 | ~400 lines rewrite (core structural change) |
| M3 All primitives | 1 | ~350 lines new renderers |
| M4 Layer overhaul | 3 | ~150 lines refactor |
| M5 Text | 1 | ~80 lines |
| M6 Arcs | 1 | ~60 lines |
| M7 Regions | 1 | ~80 lines |
| M8 Pads enhanced | 1 | ~120 lines |
| M9 Component bodies | 1 | ~60 lines |
| M10 Constraints | 2 | ~120 lines |
| M11 Groups | 1 | ~80 lines |
| M12 Route overlay | 4 | ~200 lines |
| M13 Hit testing | 1 | ~100 lines |
| M14 Grid + coords | 2 | ~80 lines |
| M15 Search + zoom | 1 | ~80 lines |
| M16 MSAA + quality | 2 | ~40 lines |
| M17 3D improvements | 1 | ~250 lines |

**Total estimated new/rewritten code: ~2350 lines** (current crate is ~2400)

## Crates Touched

| Crate | Changes |
|-------|---------|
| `autopcb-viewer` | Primary target — all milestones |
| `altium-format` | None — consuming existing public API |
| `altium-format-spec` | None — consuming existing SpecModel |
| `autopcb-routes` | None — consuming existing RouteSolution |
| `autopcb-ir` | None — removing dependency from viewer |
| `autopcb-placement` | None — removing dependency from viewer |

## Immediate Next Step

Start with M1 (coordinate conversion + polygon triangulation), then M2
(ViewState refactor). These two milestones are the critical path — everything
else parallelizes after them.
