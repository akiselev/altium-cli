# AltiumCanvas Trait Reference

## Location

`crates/altium-format/src/render/canvas.rs`

## Type Aliases

```rust
/// (x_mils, y_mils) — Y+ is up (Altium convention).
pub type DrawPoint = (f64, f64);
```

## Supporting Types

### Pen (Stroke)

```rust
pub struct Pen {
    pub color: Color,         // Win32 COLORREF (BGR), from altium-format-types
    pub width_mils: f64,      // Stroke width in mils; 0.0 = hairline
    pub style: LineStyle,     // Solid, Dashed, Dotted, DashDot (from altium-format-types)
}
```

Constructors:
- `Pen::new(color, width_mils)` — Solid style
- `pen.with_style(LineStyle)` — Builder for dash style

### Brush (Fill)

```rust
pub struct Brush {
    pub color: Color,
    pub transparent: bool,    // true = don't fill (pass-through)
}
```

Constructors:
- `Brush::solid(color)` — Opaque fill
- `Brush::transparent(color)` — Transparent (no fill rendered)

### FontSpec

```rust
pub struct FontSpec {
    pub name: String,         // Font family name (e.g., "Times New Roman")
    pub size_mils: f64,       // Font size in mils
    pub bold: bool,
    pub italic: bool,
}
```

Default: `"Times New Roman"`, 10.0 mils, not bold, not italic.

### RenderTransform

```rust
pub enum RenderTransform {
    Scale { sx: f64, sy: f64, origin: DrawPoint },
    Rotate { degrees: f64, origin: DrawPoint },      // CCW about origin
    Mirror { axis_x: f64 },                           // Flip about vertical line x=axis_x
}
```

Transforms are applied via a push/pop stack. The convention follows Altium's
internal model:
- **Scale**: Non-uniform scale centered on origin. Used for mirroring (sx=-1.0).
- **Rotate**: Counter-clockwise rotation in degrees about a point.
- **Mirror**: Horizontal flip about a vertical axis. Schematic components use this
  when `is_mirrored` is set.

## Trait Definition

```rust
pub trait AltiumCanvas {
    // Lines
    fn draw_line(&mut self, p1: DrawPoint, p2: DrawPoint, pen: &Pen);
    fn draw_polyline(&mut self, points: &[DrawPoint], pen: &Pen);

    // Curves
    fn draw_arc(&mut self, center: DrawPoint, rx: f64, ry: f64,
                start_deg: f64, end_deg: f64, pen: &Pen);
    fn draw_bezier(&mut self, ctrl_pts: &[DrawPoint], pen: &Pen);

    // Shapes
    fn draw_ellipse(&mut self, center: DrawPoint, rx: f64, ry: f64,
                    pen: &Pen, fill: Option<&Brush>);
    fn draw_rect(&mut self, p1: DrawPoint, p2: DrawPoint,
                 pen: &Pen, fill: Option<&Brush>);
    fn draw_rounded_rect(&mut self, p1: DrawPoint, p2: DrawPoint, rx: f64, ry: f64,
                         pen: &Pen, fill: Option<&Brush>);
    fn draw_polygon(&mut self, points: &[DrawPoint], pen: &Pen, fill: Option<&Brush>);

    // Text & Images
    fn draw_text(&mut self, text: &str, pos: DrawPoint, angle_deg: f64,
                 font: &FontSpec, color: &Pen);
    fn draw_image(&mut self, data: &[u8], p1: DrawPoint, p2: DrawPoint);

    // Transform stack
    fn push_transform(&mut self, t: &RenderTransform);
    fn pop_transform(&mut self);

    // Clip stack
    fn push_clip(&mut self, p1: DrawPoint, p2: DrawPoint);
    fn pop_clip(&mut self);
}
```

## Method Semantics

### draw_line(p1, p2, pen)
Draw a single line segment from `p1` to `p2`. Pen width and color apply.

### draw_polyline(points, pen)
Draw a connected series of line segments through `points` (open path, not closed).
Minimum 2 points required.

### draw_arc(center, rx, ry, start_deg, end_deg, pen)
Draw an elliptical arc:
- `center`: Center point
- `rx`, `ry`: Horizontal and vertical radii (equal for circular arcs)
- `start_deg`, `end_deg`: Start and end angles in degrees, counter-clockwise from
  3 o'clock (standard math convention). `end_deg > start_deg` for CCW sweep.
- Full circle: `start_deg=0, end_deg=360`

**Altium convention**: Schematic arcs store `start_angle` and optionally `end_angle`
(default 360). Elliptical arcs have separate `radius` and `secondary_radius`.

### draw_ellipse(center, rx, ry, pen, fill)
Draw an ellipse (or circle if rx == ry).
- `fill: None` → stroke only
- `fill: Some(Brush::solid(...))` → filled and stroked
- `fill: Some(Brush::transparent(...))` → stroke only (transparent brush)

### draw_rect(p1, p2, pen, fill)
Draw an axis-aligned rectangle defined by opposite corners `p1` and `p2`.
The backend normalizes the corners (min/max) before drawing.

### draw_rounded_rect(p1, p2, rx, ry, pen, fill)
Like `draw_rect` but with rounded corners. `rx` and `ry` specify the elliptical
corner radii.

### draw_polygon(points, pen, fill)
Draw a closed polygon. The path is implicitly closed (last point connects to first).

### draw_bezier(ctrl_pts, pen)
Draw cubic Bezier curves. Control points are consumed in groups of 4:
`[start, ctrl1, ctrl2, end, ctrl1, ctrl2, end, ...]`.
The first point is the starting position; subsequent groups of 3 define
continuation curves (C1, C2, endpoint).

### draw_text(text, pos, angle_deg, font, color)
Draw text at `pos` rotated by `angle_deg` (CCW). Font specifies family, size,
bold, italic. Color pen specifies the text fill color (pen width is ignored).

### draw_image(data, p1, p2)
Draw a raster image within the rectangle `(p1, p2)`. `data` contains the raw
image bytes (format determined by context — typically BMP or JPEG from Altium
embedded objects).

### push_transform(t) / pop_transform()
Push/pop a transform onto the rendering stack. Transforms apply to all subsequent
drawing operations until popped. Used for component rotation and mirroring.

### push_clip(p1, p2) / pop_clip()
Push/pop a clip rectangle. Drawing outside the clip region is discarded.
Used for sheet boundaries and component clip regions.

## Internal Helper Functions

These are `pub(crate)` and used by the dispatch modules:

```rust
/// Convert CoordPoint to DrawPoint (mils).
pub(crate) fn to_dp(p: CoordPoint) -> DrawPoint

/// Convert single Coord to f64 mils.
pub(crate) fn c_to_f(c: Coord) -> f64

/// Map PenWidth enum to mils.
pub(crate) fn pen_width_to_mils(pw: PenWidth) -> f64
```

### PenWidth Mapping

| `PenWidth` variant | Mils | Notes |
|---|---|---|
| `Zero` | 0.0 | Hairline — backends should render as ~0.5 mil or 1px minimum |
| `Small` | 1.0 | Standard wire width |
| `Medium` | 2.0 | |
| `Large` | 5.0 | |
| `_` (unknown) | 0.0 | Fallback to hairline |

These values were verified against `FileFormatConsts.cs` in the decompiled Altium
.NET assemblies.

## Provided Implementations

### RecordingCanvas (`recording.rs`)

Records all draw calls as `DrawCall` enum variants for later inspection.
Used exclusively in tests.

```rust
pub struct RecordingCanvas { pub calls: Vec<DrawCall> }
```

### NullCanvas (`recording.rs`)

Discards all draw calls (every method is a no-op). Used for smoke-testing
that rendering doesn't panic without the overhead of recording.

```rust
pub struct NullCanvas;
```

### DrawCall Enum

Mirrors all 14 `AltiumCanvas` methods:

```rust
pub enum DrawCall {
    Line { p1, p2, pen },
    Polyline { points, pen },
    Arc { center, rx, ry, start_deg, end_deg, pen },
    Ellipse { center, rx, ry, pen, fill },
    Rect { p1, p2, pen, fill },
    RoundedRect { p1, p2, rx, ry, pen, fill },
    Polygon { points, pen, fill },
    Bezier { ctrl_pts, pen },
    Text { text, pos, angle_deg, font, color },
    Image { p1, p2 },
    PushTransform(RenderTransform),
    PopTransform,
    PushClip { p1, p2 },
    PopClip,
}
```

## Altium I2DGraphics Correspondence

Our `AltiumCanvas` trait was designed from reverse-engineering Altium's `I2DGraphics`
interface. The mapping:

| Altium `I2DGraphics` method | Our `AltiumCanvas` method |
|---|---|
| `DrawLine` | `draw_line` |
| `DrawPolyline` | `draw_polyline` |
| `DrawArc` | `draw_arc` |
| `DrawEllipse` | `draw_ellipse` |
| `DrawRect` | `draw_rect` |
| `DrawRoundRect` | `draw_rounded_rect` |
| `DrawPolygon` | `draw_polygon` |
| `DrawBezier` | `draw_bezier` |
| `DrawText` | `draw_text` |
| `DrawImage` | `draw_image` |
| `PushTransform` | `push_transform` |
| `PopTransform` | `pop_transform` |
| `PushClipRect` | `push_clip` |
| `PopClipRect` | `pop_clip` |

The `I2DGraphics` interface has two known concrete implementations in Altium:
- `Direct2DGraphics` (GPU-accelerated, primary renderer)
- `GdiGraphics` (software fallback)

Both are in the `Altium.Sch.Painter` namespace, under `.Direct2D` and `.GDI` sub-namespaces.
