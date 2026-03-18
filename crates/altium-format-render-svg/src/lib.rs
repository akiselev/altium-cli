//! SVG rendering backend for Altium Designer files.
//!
//! Implements SVG output by collecting draw calls via [`altium_format::render::RecordingCanvas`]
//! and replaying them with Y-axis flip (Altium Y+ up → SVG Y+ down).
//!
//! Supports transform stack (`<g transform="...">` groups), line dash patterns
//! (`stroke-dasharray`), and Altium-matching stroke defaults (round caps and joins).

use altium_format::render::{
    Brush, DrawCall, DrawPoint, RecordingCanvas, RenderTransform, TextHAlign, TextVAlign,
};
use altium_format_types::Color;
use std::fmt::Write;
use svg::node::element::{
    Ellipse, Line, Path, Polygon, Polyline, Rectangle, Text as SvgText, path::Data,
};

fn color_to_css(c: Color) -> String {
    format!("rgb({},{},{})", c.r(), c.g(), c.b())
}

fn fill_to_css(fill: Option<&Brush>) -> String {
    match fill {
        None => "none".to_owned(),
        Some(b) if b.transparent => "none".to_owned(),
        Some(b) => color_to_css(b.color),
    }
}

/// Return SVG `stroke-dasharray` attribute string for a given line style.
/// Returns empty string for Solid (no attribute needed).
/// Values match Altium's own SVG exporter (from `SvgGraphics.cs`).
fn dash_attr(style: altium_format_types::LineStyle) -> &'static str {
    use altium_format_types::LineStyle;
    match style {
        LineStyle::Solid => "",
        LineStyle::Dashed => " stroke-dasharray=\"8 4\"",
        LineStyle::Dotted => " stroke-dasharray=\"2 4\"",
        LineStyle::DashDotted => " stroke-dasharray=\"8 4 2 4\"",
        _ => "",
    }
}

fn compute_bounds(calls: &[DrawCall]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    let mut update = |p: DrawPoint| {
        if p.0 < min_x {
            min_x = p.0;
        }
        if p.0 > max_x {
            max_x = p.0;
        }
        if p.1 < min_y {
            min_y = p.1;
        }
        if p.1 > max_y {
            max_y = p.1;
        }
    };

    for call in calls {
        match call {
            DrawCall::Line { p1, p2, .. } => {
                update(*p1);
                update(*p2);
            }
            DrawCall::Polyline { points, .. } | DrawCall::Polygon { points, .. } => {
                for p in points {
                    update(*p);
                }
            }
            DrawCall::Bezier { ctrl_pts, .. } => {
                for p in ctrl_pts {
                    update(*p);
                }
            }
            DrawCall::Arc { center, rx, ry, .. } | DrawCall::Ellipse { center, rx, ry, .. } => {
                update((center.0 - rx, center.1 - ry));
                update((center.0 + rx, center.1 + ry));
            }
            DrawCall::Rect { p1, p2, .. } | DrawCall::RoundedRect { p1, p2, .. } => {
                update(*p1);
                update(*p2);
            }
            DrawCall::Image { p1, p2 } => {
                update(*p1);
                update(*p2);
            }
            DrawCall::PushClip { p1, p2 } => {
                update(*p1);
                update(*p2);
            }
            DrawCall::Text { pos, .. } => {
                update(*pos);
            }
            DrawCall::PushTransform(_) | DrawCall::PopTransform | DrawCall::PopClip => {}
        }
    }

    if min_x == f64::MAX {
        (0.0, 100.0, 0.0, 100.0)
    } else {
        (min_x, max_x, min_y, max_y)
    }
}

/// Render a list of `DrawCall`s to an SVG document string.
///
/// Handles:
/// - Y-axis flip (Altium Y+ up → SVG Y+ down)
/// - Transform stack via `<g transform="...">` groups
/// - Round stroke caps and joins (matching Altium's `PenInfo.cs` defaults)
/// - Line dash patterns via `stroke-dasharray`
/// Render draw calls to SVG with default padding (50 mils).
pub fn draw_calls_to_svg(calls: &[DrawCall]) -> String {
    draw_calls_to_svg_with_padding(calls, 50.0)
}

/// Render draw calls to SVG with configurable padding in mils around the content.
pub fn draw_calls_to_svg_with_padding(calls: &[DrawCall], padding_mils: f64) -> String {
    let bounds = compute_bounds(calls);
    let margin = padding_mils;
    let min_x = bounds.0 - margin;
    let max_x = bounds.1 + margin;
    let min_y = bounds.2 - margin;
    let max_y = bounds.3 + margin;
    let w = (max_x - min_x).max(1.0);
    let h = (max_y - min_y).max(1.0);

    let flip_y = |y: f64| max_y - y;
    let to_svg = |p: DrawPoint| (p.0 - min_x, flip_y(p.1));

    // Build SVG body as a string to support <g> nesting for transforms.
    // Individual elements are built via the `svg` crate then serialized.
    let mut body = String::new();

    for call in calls {
        match call {
            DrawCall::PushTransform(t) => {
                let attr = transform_to_svg(t, min_x, max_y, &to_svg);
                write!(body, "<g transform=\"{attr}\">").unwrap();
            }
            DrawCall::PopTransform => {
                body.push_str("</g>");
            }
            DrawCall::Line { p1, p2, pen } => {
                let (x1, y1) = to_svg(*p1);
                let (x2, y2) = to_svg(*p2);
                let elem = Line::new()
                    .set("x1", format!("{x1:.2}"))
                    .set("y1", format!("{y1:.2}"))
                    .set("x2", format!("{x2:.2}"))
                    .set("y2", format!("{y2:.2}"))
                    .set("stroke", color_to_css(pen.color))
                    .set("stroke-width", format!("{:.2}", pen.width_mils.max(0.5)));
                write!(body, "{elem}").unwrap();
                body.push_str(dash_attr(pen.style));
            }
            DrawCall::Polyline { points, pen } => {
                if points.len() < 2 {
                    continue;
                }
                let pts_str: Vec<String> = points
                    .iter()
                    .map(|p| {
                        let (x, y) = to_svg(*p);
                        format!("{x:.2},{y:.2}")
                    })
                    .collect();
                let elem = Polyline::new()
                    .set("points", pts_str.join(" "))
                    .set("stroke", color_to_css(pen.color))
                    .set("stroke-width", format!("{:.2}", pen.width_mils.max(0.5)))
                    .set("fill", "none");
                write!(body, "{elem}").unwrap();
            }
            DrawCall::Arc {
                center,
                rx,
                ry,
                start_deg,
                end_deg,
                pen,
            } => {
                let (cx, cy) = to_svg(*center);
                let start_rad = start_deg.to_radians();
                let end_rad = end_deg.to_radians();
                // After Y-flip, sin component negates
                let x1 = cx + rx * start_rad.cos();
                let y1 = cy - ry * start_rad.sin();
                let x2 = cx + rx * end_rad.cos();
                let y2 = cy - ry * end_rad.sin();
                let sweep = end_deg - start_deg;
                let large_arc = if sweep.abs() > 180.0 { 1 } else { 0 };
                let data = Data::new()
                    .move_to((x1, y1))
                    .elliptical_arc_to((*rx, *ry, 0, large_arc, 0, x2, y2));
                let elem = Path::new()
                    .set("d", data)
                    .set("stroke", color_to_css(pen.color))
                    .set("stroke-width", format!("{:.2}", pen.width_mils.max(0.5)))
                    .set("fill", "none");
                write!(body, "{elem}").unwrap();
            }
            DrawCall::Ellipse {
                center,
                rx,
                ry,
                pen,
                fill,
            } => {
                let (cx, cy) = to_svg(*center);
                let elem = Ellipse::new()
                    .set("cx", format!("{cx:.2}"))
                    .set("cy", format!("{cy:.2}"))
                    .set("rx", format!("{rx:.2}"))
                    .set("ry", format!("{ry:.2}"))
                    .set("stroke", color_to_css(pen.color))
                    .set("stroke-width", format!("{:.2}", pen.width_mils.max(0.5)))
                    .set("fill", fill_to_css(fill.as_ref()));
                write!(body, "{elem}").unwrap();
            }
            DrawCall::Rect { p1, p2, pen, fill } => {
                let (x1, y1) = to_svg(*p1);
                let (x2, y2) = to_svg(*p2);
                let x = x1.min(x2);
                let y = y1.min(y2);
                let rw = (x2 - x1).abs();
                let rh = (y2 - y1).abs();
                let elem = Rectangle::new()
                    .set("x", format!("{x:.2}"))
                    .set("y", format!("{y:.2}"))
                    .set("width", format!("{rw:.2}"))
                    .set("height", format!("{rh:.2}"))
                    .set("stroke", color_to_css(pen.color))
                    .set("stroke-width", format!("{:.2}", pen.width_mils.max(0.5)))
                    .set("fill", fill_to_css(fill.as_ref()));
                write!(body, "{elem}").unwrap();
            }
            DrawCall::RoundedRect {
                p1,
                p2,
                rx,
                ry,
                pen,
                fill,
            } => {
                let (x1, y1) = to_svg(*p1);
                let (x2, y2) = to_svg(*p2);
                let x = x1.min(x2);
                let y = y1.min(y2);
                let rw = (x2 - x1).abs();
                let rh = (y2 - y1).abs();
                let elem = Rectangle::new()
                    .set("x", format!("{x:.2}"))
                    .set("y", format!("{y:.2}"))
                    .set("width", format!("{rw:.2}"))
                    .set("height", format!("{rh:.2}"))
                    .set("rx", format!("{rx:.2}"))
                    .set("ry", format!("{ry:.2}"))
                    .set("stroke", color_to_css(pen.color))
                    .set("stroke-width", format!("{:.2}", pen.width_mils.max(0.5)))
                    .set("fill", fill_to_css(fill.as_ref()));
                write!(body, "{elem}").unwrap();
            }
            DrawCall::Polygon { points, pen, fill } => {
                if points.is_empty() {
                    continue;
                }
                let pts_str: Vec<String> = points
                    .iter()
                    .map(|p| {
                        let (x, y) = to_svg(*p);
                        format!("{x:.2},{y:.2}")
                    })
                    .collect();
                let elem = Polygon::new()
                    .set("points", pts_str.join(" "))
                    .set("stroke", color_to_css(pen.color))
                    .set("stroke-width", format!("{:.2}", pen.width_mils.max(0.5)))
                    .set("fill", fill_to_css(fill.as_ref()));
                write!(body, "{elem}").unwrap();
            }
            DrawCall::Bezier { ctrl_pts, pen } => {
                if ctrl_pts.len() < 4 {
                    continue;
                }
                let (sx, sy) = to_svg(ctrl_pts[0]);
                let mut data = Data::new().move_to((sx, sy));
                let mut i = 0;
                while i + 3 < ctrl_pts.len() {
                    let (c1x, c1y) = to_svg(ctrl_pts[i + 1]);
                    let (c2x, c2y) = to_svg(ctrl_pts[i + 2]);
                    let (ex, ey) = to_svg(ctrl_pts[i + 3]);
                    data = data.cubic_curve_to((c1x, c1y, c2x, c2y, ex, ey));
                    i += 3;
                }
                let elem = Path::new()
                    .set("d", data)
                    .set("stroke", color_to_css(pen.color))
                    .set("stroke-width", format!("{:.2}", pen.width_mils.max(0.5)))
                    .set("fill", "none");
                write!(body, "{elem}").unwrap();
            }
            DrawCall::Text {
                text,
                pos,
                angle_deg,
                font,
                color,
            } => {
                if text.is_empty() {
                    continue;
                }
                let (x, y) = to_svg(*pos);
                let mut elem = SvgText::new(text.as_str())
                    .set("x", format!("{x:.2}"))
                    .set("y", format!("{y:.2}"))
                    .set("font-family", format!("'{}', sans-serif", font.name))
                    .set("font-size", format!("{:.1}", font.size_mils.max(6.0)))
                    .set("fill", color_to_css(color.color))
                    .set(
                        "transform",
                        format!("rotate({:.1},{:.2},{:.2})", -angle_deg, x, y),
                    );
                match font.h_align {
                    TextHAlign::Left => {} // SVG default
                    TextHAlign::Center => { elem = elem.set("text-anchor", "middle"); }
                    TextHAlign::Right => { elem = elem.set("text-anchor", "end"); }
                }
                match font.v_align {
                    TextVAlign::Baseline => {} // SVG default
                    TextVAlign::Middle => { elem = elem.set("dominant-baseline", "central"); }
                }
                write!(body, "{elem}").unwrap();
            }
            // Skip embedded images — pixel data is not available here
            DrawCall::Image { .. } => {}
            // Clip calls: not yet implemented
            DrawCall::PushClip { .. } | DrawCall::PopClip => {}
        }
    }

    // Build final SVG document with:
    // - Root <g> setting stroke-linecap and stroke-linejoin to "round" (Altium defaults)
    // - White background rectangle
    // - Body content (may contain nested <g> groups for transforms)
    format!(
        "<svg viewBox=\"0 0 {w:.1} {h:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" \
         xmlns=\"http://www.w3.org/2000/svg\">\
         <g stroke-linecap=\"round\" stroke-linejoin=\"round\">\
         <rect x=\"0\" y=\"0\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"white\"/>\
         {body}\
         </g></svg>"
    )
}

/// Convert a `RenderTransform` to an SVG transform attribute string.
///
/// Accounts for the Y-axis flip (Altium Y+ up → SVG Y+ down):
/// - Rotation angles are negated (CCW in Altium → CW in SVG)
/// - Mirror is about a vertical axis (X-flip, unaffected by Y-flip direction)
/// - Scale origins are converted to SVG space
fn transform_to_svg(
    t: &RenderTransform,
    min_x: f64,
    _max_y: f64,
    to_svg: &dyn Fn(DrawPoint) -> (f64, f64),
) -> String {
    match t {
        RenderTransform::Rotate { degrees, origin } => {
            let (ox, oy) = to_svg(*origin);
            // Negate angle: Altium CCW positive → SVG CW positive (Y-flip)
            format!("rotate({:.2},{:.2},{:.2})", -degrees, ox, oy)
        }
        RenderTransform::Mirror { axis_x } => {
            // Mirror about vertical line x = axis_x.
            // In SVG coords: svg_ax = axis_x - min_x.
            // Matrix form: translate(svg_ax) scale(-1,1) translate(-svg_ax)
            let svg_ax = axis_x - min_x;
            format!(
                "translate({:.2},0) scale(-1,1) translate({:.2},0)",
                svg_ax, -svg_ax
            )
        }
        RenderTransform::Scale { sx, sy, origin } => {
            let (ox, oy) = to_svg(*origin);
            format!(
                "translate({:.2},{:.2}) scale({:.2},{:.2}) translate({:.2},{:.2})",
                ox, oy, sx, sy, -ox, -oy
            )
        }
    }
}

/// Render a SchLib component by name to an SVG string.
pub fn render_schlib_component(
    lib: &altium_format::SchLib,
    name: &str,
) -> altium_format::Result<String> {
    let mut canvas = RecordingCanvas::new();
    lib.render_component(name, &mut canvas)?;
    Ok(draw_calls_to_svg(&canvas.calls))
}

/// Render a SchDoc sheet to an SVG string.
pub fn render_schdoc(doc: &altium_format::SchDoc) -> altium_format::Result<String> {
    let mut canvas = RecordingCanvas::new();
    doc.render(&mut canvas)?;
    Ok(draw_calls_to_svg(&canvas.calls))
}

/// Render a PcbLib footprint by name to an SVG string.
pub fn render_pcblib_footprint(
    lib: &altium_format::PcbLib,
    name: &str,
) -> altium_format::Result<String> {
    let mut canvas = RecordingCanvas::new();
    lib.render_footprint(name, &mut canvas)?;
    Ok(draw_calls_to_svg(&canvas.calls))
}
