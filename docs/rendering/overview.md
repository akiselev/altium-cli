# Rendering Architecture Overview

## Purpose

The `altium-format` rendering module provides an abstract canvas API (`AltiumCanvas` trait)
that dispatch functions use to draw schematic records and PCB primitives. Concrete backends
(SVG, PNG, etc.) live in separate crates and implement `AltiumCanvas` or consume the
intermediate `DrawCall` representation.

## Crate Layout

```
altium-format                    # Core rendering trait + dispatch
  src/render/
    mod.rs                       # Module root, pub re-exports
    canvas.rs                    # AltiumCanvas trait, Pen, Brush, FontSpec, RenderTransform
    sch.rs                       # draw_sch_record() — SchRecord → canvas calls
    pcb.rs                       # draw_pcb_primitive() — PcbPrimitive → canvas calls
    recording.rs                 # RecordingCanvas (test), NullCanvas (smoke-test), DrawCall enum

altium-format-render-svg         # SVG backend
  src/lib.rs                     # draw_calls_to_svg(), render_schlib_component(), etc.

altium-format-render-png         # PNG backend (SVG → resvg → tiny-skia → PNG)
  src/lib.rs                     # svg_to_png(), render_schlib_component_png(), etc.
```

**Why the trait lives in `altium-format`**: The dispatch functions need access to
`SchRecord` and `PcbPrimitive`, both of which are `pub(crate)`. Keeping the trait
in the same crate avoids exposing parser internals. Backends only need the `pub`
trait and the `pub` `DrawCall` enum.

## Coordinate System

All rendering uses **mils** (1 mil = 0.001 inch = 10,000 Altium internal units).

- **AltiumCanvas convention**: Y+ is up (Altium's native convention)
- **SVG convention**: Y+ is down — the SVG backend flips Y during replay
- **Conversion helpers** (`canvas.rs`):
  - `to_dp(CoordPoint) -> DrawPoint` — converts via `Coord::to_mils()`
  - `c_to_f(Coord) -> f64` — single-axis conversion
  - `DrawPoint = (f64, f64)` — `(x_mils, y_mils)`

### Altium Coordinate Encoding

- **Schematic**: `Coord` stores integer mils + optional `_FRAC` parameter (DXP fractional encoding)
- **PCB**: `Coord` stores raw `i32` in 10nm units (10,000 = 1 mil)
- **Colors**: Win32 COLORREF `0x00BBGGRR` — accessed via `Color::r()`, `Color::g()`, `Color::b()`

## Rendering Pipeline

### For SchLib / SchDoc

```
SchLib::render_component(name, canvas)
  → find component by lib_reference
  → iterate records + additional_records
  → for each: draw_sch_record(record, canvas, fonts)

SchDoc::render(canvas)
  → extract FontTable from Sheet record
  → iterate all records
  → for each: draw_sch_record(record, canvas, fonts)
```

### For PcbLib

```
PcbLib::render_footprint(name, canvas)
  → find footprint by display_name
  → iterate primitives
  → for each: draw_pcb_primitive(prim, canvas)
```

### SVG Output

```
RecordingCanvas collects DrawCalls
  → compute_bounds() scans all points for min/max
  → draw_calls_to_svg() replays with Y-flip: svg_y = max_y - altium_y
  → emits SVG elements (Line, Polyline, Ellipse, Rectangle, Path, Polygon, Text)
```

### PNG Output

```
SVG string → usvg::Tree::from_str()
  → resvg::render() at scale (default 4 px/mil)
  → tiny_skia::Pixmap → encode_png()
```

## Testing

- **`RecordingCanvas`**: Records all `DrawCall`s for assertion in unit tests
- **`NullCanvas`**: Discards all calls — for smoke-testing that rendering doesn't panic
- **Unit tests** in `render/sch.rs` and `render/pcb.rs` verify correct dispatch
  (e.g., `Wire → Polyline`, `Junction → Ellipse`, `NoConnect → 2x Line`)

---

## Altium's Own Rendering Architecture (Reverse Engineering Findings)

The following documents what we learned by reverse-engineering Altium Designer 26's
rendering code. This provides context for why our canvas API looks the way it does
and serves as a reference for future improvements.

### Schematic Rendering (C# / .NET)

Altium's schematic rendering lives in the .NET assemblies under `Altium.Sch.Painter`:

**Key namespace**: `Altium.Sch.Painter`

**Architecture**: Retained-mode rendering with a factory pattern:

1. **`GraphicsFactory`** — Static factory class that dispatches on `SchRecord.RecordType`
   to create geometry objects. Each record type has a registered creator method.

2. **`GeometryStorage`** — Scene graph / display list that holds the created geometry
   objects. Acts as an intermediate representation between parsing and painting.

3. **`I2DGraphics`** — Abstract rendering interface (analogous to our `AltiumCanvas`).
   Concrete implementations:
   - **`Direct2DGraphics`** — Primary GPU-accelerated backend using Direct2D.
     Lives in `Altium.Sch.Painter.Direct2D` namespace.
   - **`GdiGraphics`** — Fallback software renderer using GDI+.
     Lives in `Altium.Sch.Painter.GDI` namespace.
   - Both implement the same `I2DGraphics` interface with ~15 drawing operations.

4. **Dispatch table**: `GraphicsFactory` maintains a dictionary keyed on
   `SchRecordType` → creator delegate. Each creator produces a geometry node
   from the parsed record data. During paint, the geometry nodes replay
   through the `I2DGraphics` interface.

**Canvas primitive operations** (from `I2DGraphics` interface):
- `DrawLine`, `DrawPolyline`
- `DrawArc`, `DrawEllipse`
- `DrawRect`, `DrawRoundRect`
- `DrawPolygon`
- `DrawBezier` (cubic, groups of 4 control points)
- `DrawText` (with font, rotation, alignment)
- `DrawImage`
- `PushTransform` / `PopTransform` (transform stack)
- `PushClip` / `PopClip` (clip rectangle stack)

These map 1:1 to our `AltiumCanvas` trait methods.

**Font handling**: The schematic document carries a font table in the `Sheet` record
(record type 31). Font IDs are 1-based indices into this table. Each font entry stores
name, size, bold, italic flags. Components in a SchLib don't have their own sheet record,
so a default font is used when rendering standalone components.

### PCB Rendering (Delphi / Native)

Altium's PCB rendering is implemented in native Delphi DLLs:

**Key class**: `PCPainterClass` in `Advpcb.dll`

**Architecture**: Unlike the schematic side, PCB rendering is more direct — no
intermediate scene graph. The painter class walks the PCB document's primitive
lists and draws directly via the paint interface.

**Dispatch**: `PCPainterClass` has a virtual method table (VMT) with per-primitive-type
paint methods. The dispatcher reads the `TObjectId` byte from each primitive record
and calls the corresponding VMT slot.

**Backends**:
- **`GraphiteView.dll`** — Altium's custom high-performance PCB rendering engine.
  This is a separate DLL that provides the "3D-accelerated" PCB view. It's a
  Delphi-native renderer that likely uses OpenGL or DirectX under the hood.
  Not accessible via the standard `I2DGraphics` interface.
- **Standard 2D** — For print/export, PCB uses the same GDI/Direct2D backends
  as schematics, routed through `PCPainterClass`.

**Layer-aware rendering**: PCB rendering is fundamentally layer-based. Each primitive
has a `V6Layer` assignment, and the painter only draws primitives on currently-visible
layers. Layer colors are configurable per-project. Our current renderer ignores layers
(draws everything in black) — this is a known simplification.

### Shared Infrastructure

Both schematic and PCB renderers share:

1. **Color encoding**: Win32 COLORREF `0x00BBGGRR` throughout. The `Color` type
   in `altium-format-types` handles BGR→RGB conversion.

2. **Coordinate precision**: Internally everything uses 10nm resolution (i32).
   Rendering converts to floating-point mils at the canvas boundary.

3. **PenWidth enum mapping** (verified against `FileFormatConsts.cs`):
   | PenWidth enum | Width in mils |
   |---|---|
   | `Zero` (Smallest) | 0.0 (hairline — backends render as ~0.5 mil or 1px minimum) |
   | `Small` | 1.0 |
   | `Medium` | 2.0 |
   | `Large` | 5.0 |

4. **Transform stack**: Both use push/pop transform semantics. Component instances
   apply mirror + rotation transforms before rendering child primitives:
   ```
   if mirrored: push Mirror { axis_x: component.x }
   push Rotate { degrees: component.orientation, origin: component.location }
   ... render children ...
   pop Rotate
   if mirrored: pop Mirror
   ```

5. **Arc convention**: Angles in degrees, counter-clockwise from 3 o'clock (standard
   math convention). `start_angle` and `end_angle` define the sweep. Full circle
   when `end_angle - start_angle == 360`.

## Known Limitations

Current renderer simplifications vs. Altium's full implementation:

1. **No layer filtering** — PCB renders all layers in black (Altium filters by visibility)
2. **No transform stack in SVG** — PushTransform/PopTransform are recorded but not replayed
   in the SVG backend (geometry is already in world coordinates for most cases)
3. **No embedded images** — `draw_image()` is called but SVG backend skips image data
4. **Simplified power symbols** — PowerObject renders as circle + text; Altium has
   ~15 distinct power symbol shapes (VCC bar, GND rake, etc.)
5. **No text metrics** — Text bounding boxes are not computed; overlap may occur
6. **No line styles** — LineStyle (dashed, dotted) is captured in Pen but not rendered in SVG
7. **Pad shape approximations** — RoundedRectangular uses 25% corner radius heuristic;
   Altium stores exact corner radius percentage per pad

## Future Work

- Implement transform stack replay in SVG backend (`<g transform="...">` groups)
- Add layer color support for PCB rendering
- Implement all power symbol shapes (reference `Altium.Sch.Painter` factory methods)
- Add proper text metrics (measure text width for bounding box computation)
- Support line dash patterns in SVG (`stroke-dasharray`)
- Add PcbDoc rendering (full board, not just single footprints)
