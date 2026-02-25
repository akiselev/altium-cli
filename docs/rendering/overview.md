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
name, size, bold, italic flags. When fontId=0, Altium uses the system font ID from
preferences (`HorizontalSysFontId` or `VerticalSysFontId`). Components in a SchLib
don't have their own sheet record, so the font table is empty and all text uses the
default font.

Default font (from `DrawGraphObjectBase.GetFontInfo()` fallback): **"Tahoma"**, 10 mils.
Font size is stored as `argSize * 100000` internal units (size 10 = 1,000,000 = 100 mils).

**Stroke defaults** (from `PenInfo.cs`):
- Default line cap: `Round` (for start, end, and dash caps)
- Default line join: `Round`
- MiterLimit: 1000.0

**Line dash patterns** (from `DashStyleInfoHelper.cs` and `SvgGraphics.cs`):
- Solid: no dash
- Dashed: pattern `[2.0, 2.0]` × pen width, SVG: `stroke-dasharray="4"`
- Dotted: pattern `[1.0, 2.0]` × pen width, SVG: `stroke-dasharray="2"`
- DashDot: SVG: `stroke-dasharray="4 2"`

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

3. **PenWidth enum mapping** (from `Rt_Schematic.Consts.LineWidthArrayC` in decompiled C#):

   **Wire/line widths** (`LineWidthArrayC`):
   | PenWidth enum | Internal units | Mils |
   |---|---|---|
   | `Zero` (Smallest) | 0 | 0.0 (hairline — backends render as ~0.5 mil or 1px minimum) |
   | `Small` | 100,000 | 10.0 |
   | `Medium` | 300,000 | 30.0 |
   | `Large` | 500,000 | 50.0 |

   **Bus widths** (`BusLineWidthArrayC` — separate lookup table, NOT wire width + offset):
   | PenWidth enum | Internal units | Mils |
   |---|---|---|
   | `Zero` | 200,000 | 20.0 |
   | `Small` | 300,000 | 30.0 |
   | `Medium` | 500,000 | 50.0 |
   | `Large` | 700,000 | 70.0 |

   **Junction sizes** (`cJunctionSizeArray` — diameters, not radii):
   | PenWidth/TSize enum | Internal units | Mils (diameter) |
   |---|---|---|
   | `Zero` | 200,000 | 20.0 |
   | `Small` | 300,000 | 30.0 |
   | `Medium` | 500,000 | 50.0 |
   | `Large` | 1,000,000 | 100.0 |

   Source: `AD26-dotnet/Altium.Edp.Interfaces/Rt_Schematic/Consts.cs` lines 2458-2891.

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

See `plan.md` for the full prioritized gap list. Current simplifications:

1. **No layer filtering** — PCB renders all layers in black (Altium filters by visibility)
2. **No embedded images** — `draw_image()` is called but SVG backend skips image data
3. **Simplified power symbols** — PowerObject renders as circle + text; Altium has
   11 distinct power symbol shapes (see plan.md Phase 2.1)
4. **No text metrics** — Text bounding boxes are not computed; overlap may occur
5. **Pad shape approximations** — RoundedRectangular uses 25% corner radius heuristic;
   Altium stores exact corner radius percentage per pad
6. **Placeholder port/sheet entry shapes** — Ports render as rectangles, sheet entries
   as circles; Altium has 7 port styles and 4 sheet entry arrow kinds
7. **No pin IEEE symbols** — 13+ pin decoration types (dot, clock, active-low) not rendered
8. **No sheet/document rendering** — Border, reference zones, grid, title block missing
