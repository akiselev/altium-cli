//! Core canvas trait and drawing primitive types for the Altium renderer.

use altium_format_types::{Color, Coord, CoordPoint};

/// (x_mils, y_mils) — Y+ is up, same convention as Altium internal coords.
/// Backends that need Y-flipped (screen) coordinates handle the flip themselves.
pub type DrawPoint = (f64, f64);

/// Convert a CoordPoint to DrawPoint.
pub(crate) fn to_dp(p: CoordPoint) -> DrawPoint {
    (p.x.to_mils(), p.y.to_mils())
}

/// Convert a Coord to f64 mils.
pub(crate) fn c_to_f(c: Coord) -> f64 {
    c.to_mils()
}

/// Convert a PenWidth (wire/line) to mils.
///
/// From `Rt_Schematic.Consts.LineWidthArrayC` (Consts.cs lines 2880-2885):
/// - eZeroSize = 0        → 0.0 mils (hairline)
/// - eSmall    = 100,000  → 10.0 mils
/// - eMedium   = 300,000  → 30.0 mils
/// - eLarge    = 500,000  → 50.0 mils
pub(crate) fn pen_width_to_mils(pw: altium_format_types::PenWidth) -> f64 {
    use altium_format_types::PenWidth;
    match pw {
        PenWidth::Zero => 0.0,
        PenWidth::Small => 10.0,
        PenWidth::Medium => 30.0,
        PenWidth::Large => 50.0,
        _ => 0.0,
    }
}

/// Convert a PenWidth (bus) to mils. Buses use a separate, wider lookup table.
///
/// From `Rt_Schematic.Consts.BusLineWidthArrayC` (Consts.cs lines 2886-2891):
/// - eZeroSize = 200,000  → 20.0 mils
/// - eSmall    = 300,000  → 30.0 mils
/// - eMedium   = 500,000  → 50.0 mils
/// - eLarge    = 700,000  → 70.0 mils
pub(crate) fn bus_width_to_mils(pw: altium_format_types::PenWidth) -> f64 {
    use altium_format_types::PenWidth;
    match pw {
        PenWidth::Zero => 20.0,
        PenWidth::Small => 30.0,
        PenWidth::Medium => 50.0,
        PenWidth::Large => 70.0,
        _ => 20.0,
    }
}

/// Convert a junction TSize to radius in mils.
///
/// From `Rt_Schematic.Consts.cJunctionSizeArray` (Consts.cs lines 2458-2463).
/// The array stores **diameters**; this function returns **radius** (half).
/// - eZeroSize = 200,000  → diameter 20.0 mils → radius 10.0
/// - eSmall    = 300,000  → diameter 30.0 mils → radius 15.0
/// - eMedium   = 500,000  → diameter 50.0 mils → radius 25.0
/// - eLarge    = 1,000,000 → diameter 100.0 mils → radius 50.0
pub(crate) fn junction_radius_mils(pw: altium_format_types::PenWidth) -> f64 {
    use altium_format_types::PenWidth;
    match pw {
        PenWidth::Zero => 10.0,
        PenWidth::Small => 15.0,
        PenWidth::Medium => 25.0,
        PenWidth::Large => 50.0,
        _ => 15.0, // default to Small
    }
}

/// Pen (stroke) definition.
#[derive(Debug, Clone)]
pub struct Pen {
    pub color: Color,
    pub width_mils: f64,
    pub style: altium_format_types::LineStyle,
}

impl Pen {
    pub fn new(color: Color, width_mils: f64) -> Self {
        Self {
            color,
            width_mils,
            style: altium_format_types::LineStyle::Solid,
        }
    }
    pub fn with_style(mut self, style: altium_format_types::LineStyle) -> Self {
        self.style = style;
        self
    }
}

/// Fill (brush) definition.
#[derive(Debug, Clone)]
pub struct Brush {
    pub color: Color,
    pub transparent: bool,
}

impl Brush {
    pub fn solid(color: Color) -> Self {
        Self {
            color,
            transparent: false,
        }
    }
    pub fn transparent(color: Color) -> Self {
        Self {
            color,
            transparent: true,
        }
    }
}

/// Font specification for text rendering.
#[derive(Debug, Clone)]
pub struct FontSpec {
    pub name: String,
    pub size_mils: f64,
    pub bold: bool,
    pub italic: bool,
}

impl Default for FontSpec {
    fn default() -> Self {
        Self {
            name: "Tahoma".to_owned(),
            size_mils: 10.0,
            bold: false,
            italic: false,
        }
    }
}

/// A transform to push onto the canvas transform stack.
#[derive(Debug, Clone)]
pub enum RenderTransform {
    /// Uniform or non-uniform scale about an origin point.
    Scale { sx: f64, sy: f64, origin: DrawPoint },
    /// Rotation in degrees (counter-clockwise) about an origin point.
    Rotate { degrees: f64, origin: DrawPoint },
    /// Horizontal mirror (flip about a vertical line at x = axis_x).
    Mirror { axis_x: f64 },
}

/// The core rendering interface. Implement this to create a rendering backend.
///
/// Coordinates are in mils (1 mil = 0.001 inch). Y+ is up (Altium convention).
/// Backends that need screen coordinates (Y+ down) should flip in their impl.
pub trait AltiumCanvas {
    fn draw_line(&mut self, p1: DrawPoint, p2: DrawPoint, pen: &Pen);
    fn draw_polyline(&mut self, points: &[DrawPoint], pen: &Pen);
    fn draw_arc(
        &mut self,
        center: DrawPoint,
        rx: f64,
        ry: f64,
        start_deg: f64,
        end_deg: f64,
        pen: &Pen,
    );
    fn draw_ellipse(
        &mut self,
        center: DrawPoint,
        rx: f64,
        ry: f64,
        pen: &Pen,
        fill: Option<&Brush>,
    );
    fn draw_rect(&mut self, p1: DrawPoint, p2: DrawPoint, pen: &Pen, fill: Option<&Brush>);
    fn draw_rounded_rect(
        &mut self,
        p1: DrawPoint,
        p2: DrawPoint,
        rx: f64,
        ry: f64,
        pen: &Pen,
        fill: Option<&Brush>,
    );
    fn draw_polygon(&mut self, points: &[DrawPoint], pen: &Pen, fill: Option<&Brush>);
    fn draw_bezier(&mut self, ctrl_pts: &[DrawPoint], pen: &Pen);
    fn draw_text(
        &mut self,
        text: &str,
        pos: DrawPoint,
        angle_deg: f64,
        font: &FontSpec,
        color: &Pen,
    );
    fn draw_image(&mut self, data: &[u8], p1: DrawPoint, p2: DrawPoint);
    fn push_transform(&mut self, t: &RenderTransform);
    fn pop_transform(&mut self);
    fn push_clip(&mut self, p1: DrawPoint, p2: DrawPoint);
    fn pop_clip(&mut self);
}
