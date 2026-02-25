# Schematic Primitive Rendering

## Location

`crates/altium-format/src/render/sch.rs`

## Entry Point

```rust
pub(crate) fn draw_sch_record(
    record: &SchRecord,
    canvas: &mut dyn AltiumCanvas,
    fonts: &[SchFont],
)
```

Called by:
- `SchLib::render_component()` — iterates `records` + `additional_records`
- `SchDoc::render()` — iterates all records, extracts font table from Sheet record

## Font Table

Schematic documents carry a font table in the `Sheet` record (SchRecord type 31).
Font IDs are **1-based** indices into this table. The lookup function:

```rust
fn lookup_font(fonts: &[SchFont], font_id: i32) -> FontSpec {
    let idx = (font_id - 1) as usize;  // Convert 1-based to 0-based
    fonts.get(idx).map(|f| FontSpec { ... }).unwrap_or(FontSpec::default())
}
```

Default font (when ID is out of range or no font table): `"Times New Roman"`, 10 mils, regular.

For SchLib components rendered standalone, there is no Sheet record, so the font table
is empty and all text uses the default font.

## Dispatch Table

### Graphical Records

| SchRecord variant | Canvas calls | Notes |
|---|---|---|
| `Wire` | `draw_polyline(vertices, pen(line_width, color, line_style))` | Standard electrical wire |
| `Bus` | `draw_polyline(vertices, pen(line_width+1, color))` | Buses rendered 1 mil thicker than stated |
| `BusEntry` | `draw_line(location, corner, pen(line_width, color))` | Diagonal entry into bus |
| `Pin` | `draw_line(loc → loc+dir*len)` + `draw_text(name)` + `draw_text(designator)` | See Pin geometry below |
| `Line` | `draw_line(location, corner, pen(line_width, color, line_style))` | Generic line |
| `Rectangle` | `draw_rect(location, corner, pen, fill?)` | Fill logic: solid→Brush::solid, transparent→Brush::transparent |
| `RoundRectangle` | `draw_rounded_rect(location, corner, corner_x_radius, corner_y_radius, pen, fill?)` | |
| `Arc` | `draw_arc(location, radius, radius, start_angle, end_angle, pen)` | Circular arc (rx == ry) |
| `EllipticalArc` | `draw_arc(location, radius, secondary_radius, start_angle, end_angle, pen)` | Elliptical arc (rx != ry) |
| `Ellipse` | `draw_ellipse(location, radius, secondary_radius, pen, fill?)` | Includes transparency handling |
| `Pie` | `draw_arc` + 2x `draw_line` (radials) + `draw_polygon` (filled sector) | See Pie geometry below |
| `Polyline` | `draw_polyline(vertices, pen(line_width, color, line_style))` | Open path |
| `Polygon` | `draw_polygon(vertices, pen, fill?)` | Closed path with optional fill |
| `Bezier` | `draw_bezier(vertices, pen)` | Cubic Bezier (groups of 4 points) |
| `Label` | `draw_text(text, location, rotation, font(font_id), pen(color))` | Text label |
| `NetLabel` | `draw_text(text, location, rotation, font, pen)` + `draw_ellipse` (junction dot) | Junction dot: 2 mil radius |
| `Designator` | `draw_text(text, location, rotation, font(font_id), pen(color))` | Component designator (hidden check) |
| `Parameter` | `draw_text(text, location, rotation, font(font_id), pen(color))` | Component parameter (hidden check) |
| `TextFrame` | `draw_rect(location, corner)` (if show_border) + `draw_text(text)` | Frame with text content |
| `Junction` | `draw_ellipse(location, 5, 5, pen, solid_brush)` | Filled dot, 5 mil radius |
| `NoConnect` | 2x `draw_line` forming X shape | Arms: ±5 mils from location |
| `PowerObject` | `draw_ellipse(location, 5, 5, pen, None)` + `draw_text(text)` | Simplified; see Power symbols below |
| `Port` | `draw_rect(location, location+w,location+h)` + `draw_text(name)` | Rectangular port shape |
| `SheetSymbol` | `draw_rect(location, location+x_size, location-y_size)` | Note: Y extends downward |
| `SheetEntry` | `draw_ellipse(location, 3, 3, pen, solid)` + `draw_text(name)` | Small dot on sheet edge |
| `Image` | `draw_image([], location, corner)` | Empty data (embedded images not extracted yet) |
| `Component` | `push_transform(Mirror?)` + `push_transform(Rotate)` | See Component transform below |
| `Symbol` | `draw_ellipse(location, 5, 5, pen, None)` if not NoSymbol | IEEE logic symbols (placeholder) |

### Non-Graphical Records (Skipped)

These produce no canvas calls:
- `Sheet` — Document metadata and font table (consumed separately)
- `Template` — Page template reference
- `ImplementationList`, `Implementation`, `ImplementationMap`, `MapDefiner` — Simulation/model links
- `ParameterList`, `ParameterSet` — Parameter containers
- `SheetName`, `SheetFileName` — Hierarchical sheet metadata
- `Note` — Design notes (not rendered in schematic view)
- `Probe`, `CompileMask`, `Blanket` — ERC/compilation artifacts

## Geometry Details

### Pin Direction/Rotation

Pin direction is determined by `RotationBy90`:

| Orientation | Direction vector (dx, dy) | Pin extends toward |
|---|---|---|
| `Rotate0` | (1, 0) | Right (→) |
| `Rotate90` | (0, 1) | Up (↑) |
| `Rotate180` | (-1, 0) | Left (←) |
| `Rotate270` | (0, -1) | Down (↓) |

Pin endpoint: `end = location + direction * pin_length`

The pin line is drawn from `location` (connection point on component body)
to `end` (the net connection point). Name text is placed at `end`, designator
text at `location`.

Hidden pins (`is_hidden = true`) are skipped entirely.

### Component Transform

When rendering a component, transforms are pushed before child records and must
be popped after. The current implementation pushes transforms but relies on
the caller to handle pop (via record ordering in the flat list).

```rust
// Order matters: mirror first, then rotate
if component.is_mirrored {
    push_transform(Mirror { axis_x: location.x })
}
push_transform(Rotate { degrees: orientation.to_degrees(), origin: location })
```

Altium's `orientation` field is a `RotationBy90` enum (0°, 90°, 180°, 270°).

### Pie Sector Geometry

A pie chart sector is rendered as three elements:
1. **Arc** from `start_angle` to `end_angle`
2. **Two radial lines** from center to arc endpoints:
   - Start: `center + radius * (cos(start), sin(start))`
   - End: `center + radius * (cos(end), sin(end))`
3. **Filled polygon** (if solid): center + 32 arc sample points forming the sector

The 32-step polygon approximation is used because there's no native "filled arc sector"
primitive in most rendering backends.

### Fill Logic

The fill pattern for Rectangle, Ellipse, and Polygon:
```
if is_solid && !transparent → Brush::solid(area_color)
if transparent → Brush::transparent(area_color)     // transparent brush = no fill
else → None                                           // no fill
```

`area_color` is distinct from the stroke `color` — Altium allows independent
fill and stroke colors.

### NetLabel Junction Dot

NetLabel records render a small filled ellipse (2 mil radius) at the connection
point in addition to the text. This matches Altium's behavior where net labels
show a junction-like dot at their anchor.

### NoConnect X Shape

The X symbol spans ±5 mils from the location point:
```
Line: (x-5, y-5) → (x+5, y+5)    (diagonal ↗)
Line: (x+5, y-5) → (x-5, y+5)    (diagonal ↖)
```

### Bus Width

Buses are rendered 1 mil thicker than their stated `line_width`:
```rust
let width = pen_width_to_mils(b.line_width) + 1.0;
```

This matches Altium's visual convention where buses are visually distinguished
from wires by being slightly thicker.

## Power Symbol Shapes

**Current status**: Simplified — all power objects render as a circle + text.

Altium has ~15 distinct power symbol shapes controlled by the `PowerObject.style` enum.
These are defined in `Altium.Sch.DataModel.FileFormats.FileFormatConsts` and rendered
by dedicated factory methods in `GraphicsFactory`.

Known power symbol styles (from C# decompilation):
- **Power Bar** — Horizontal bar (VCC, VDD style)
- **Power Rail** — Vertical rail with ticks
- **Power GND** — Standard ground symbol (three decreasing horizontal lines)
- **Signal GND** — Triangle ground symbol
- **Earth GND** — Earth ground (three lines with decreasing length + bottom wavy line)
- **Power Arrow** — Arrow pointing at net
- **Power Wave** — Sine wave symbol
- **Circle** — Simple circle (current implementation)

Each shape has specific geometry (line patterns, angles, sizes) documented in
`FileFormatConsts.cs` as constant values. Implementing all shapes is future work.

## Altium's Schematic Rendering Pipeline (Reference)

From reverse-engineering `Altium.Sch.Painter`:

1. **Load**: `SchDocumentFactory.LoadDocument()` parses CFB → flat record list
2. **Index**: Records linked via `OWNERINDEX` into parent/child tree
3. **Create geometry**: `GraphicsFactory.CreateGeometry(record)` dispatches on `RecordType`
4. **Store**: Geometry objects placed in `GeometryStorage` (spatial index)
5. **Paint**: Iterate visible geometry, call `I2DGraphics` methods
6. **Transform**: Component children painted within parent's transform context

Our implementation collapses steps 3-5: we go directly from parsed records to
canvas calls, with no intermediate geometry storage. This is simpler but means
we can't do spatial queries (zoom culling, hit testing) like Altium can.
