//! Schematic record draw dispatch for AltiumCanvas.

use crate::render::canvas::{
    AltiumCanvas, Brush, FontSpec, Pen, RenderTransform, bus_width_to_mils, c_to_f,
    junction_radius_mils, pen_width_to_mils, to_dp,
};
use crate::sch_records::SchRecord;
use altium_format_types::sch::SchFont;

pub(crate) fn draw_sch_record(
    record: &SchRecord,
    canvas: &mut dyn AltiumCanvas,
    fonts: &[SchFont],
) {
    match record {
        SchRecord::Wire(w) => {
            let pts: Vec<_> = w.vertices.iter().copied().map(to_dp).collect();
            let pen = Pen::new(w.color, pen_width_to_mils(w.line_width)).with_style(w.line_style);
            canvas.draw_polyline(&pts, &pen);
        }
        SchRecord::Bus(b) => {
            let pts: Vec<_> = b.vertices.iter().copied().map(to_dp).collect();
            let pen = Pen::new(b.color, bus_width_to_mils(b.line_width));
            canvas.draw_polyline(&pts, &pen);
        }
        SchRecord::BusEntry(be) => {
            let pen = Pen::new(be.color, pen_width_to_mils(be.line_width));
            canvas.draw_line(to_dp(be.location), to_dp(be.corner), &pen);
        }
        SchRecord::Pin(p) => {
            if p.is_hidden {
                return;
            }
            let (dx, dy) = match p.orientation {
                altium_format_types::RotationBy90::Rotate0 => (1.0, 0.0),
                altium_format_types::RotationBy90::Rotate90 => (0.0, 1.0),
                altium_format_types::RotationBy90::Rotate180 => (-1.0, 0.0),
                altium_format_types::RotationBy90::Rotate270 => (0.0, -1.0),
                _ => (1.0, 0.0),
            };
            let len = c_to_f(p.pin_length);
            let loc = to_dp(p.location);
            let end = (loc.0 + dx * len, loc.1 + dy * len);
            let pen = Pen::new(p.color, 1.0);
            canvas.draw_line(loc, end, &pen);
            if p.show_name && !p.name.is_empty() {
                let font = lookup_font(fonts, 1);
                canvas.draw_text(&p.name, end, 0.0, &font, &pen);
            }
            if p.show_designator && !p.designator.is_empty() {
                let font = lookup_font(fonts, 1);
                canvas.draw_text(&p.designator, loc, 0.0, &font, &pen);
            }
        }
        SchRecord::Line(l) => {
            let pen = Pen::new(l.color, pen_width_to_mils(l.line_width)).with_style(l.line_style);
            canvas.draw_line(to_dp(l.location), to_dp(l.corner), &pen);
        }
        SchRecord::Rectangle(r) => {
            let pen = Pen::new(r.color, pen_width_to_mils(r.line_width));
            let fill = if r.is_solid && !r.transparent {
                Some(Brush::solid(r.area_color))
            } else if r.transparent {
                Some(Brush::transparent(r.area_color))
            } else {
                None
            };
            canvas.draw_rect(to_dp(r.location), to_dp(r.corner), &pen, fill.as_ref());
        }
        SchRecord::RoundRectangle(r) => {
            let pen = Pen::new(r.color, pen_width_to_mils(r.line_width));
            let fill = if r.is_solid {
                Some(Brush::solid(r.area_color))
            } else {
                None
            };
            canvas.draw_rounded_rect(
                to_dp(r.location),
                to_dp(r.corner),
                c_to_f(r.corner_x_radius),
                c_to_f(r.corner_y_radius),
                &pen,
                fill.as_ref(),
            );
        }
        SchRecord::Arc(a) => {
            let pen = Pen::new(a.color, pen_width_to_mils(a.line_width));
            let r = c_to_f(a.radius);
            let end = a.end_angle.as_ref().map(|e| e.0).unwrap_or(360.0);
            canvas.draw_arc(to_dp(a.location), r, r, a.start_angle.0, end, &pen);
        }
        SchRecord::EllipticalArc(a) => {
            let pen = Pen::new(a.color, pen_width_to_mils(a.line_width));
            let end = a.end_angle.as_ref().map(|e| e.0).unwrap_or(360.0);
            canvas.draw_arc(
                to_dp(a.location),
                c_to_f(a.radius),
                c_to_f(a.secondary_radius),
                a.start_angle.0,
                end,
                &pen,
            );
        }
        SchRecord::Ellipse(e) => {
            let pen = Pen::new(e.color, pen_width_to_mils(e.line_width));
            let fill = if e.is_solid && !e.transparent {
                Some(Brush::solid(e.area_color))
            } else if e.transparent {
                Some(Brush::transparent(e.area_color))
            } else {
                None
            };
            canvas.draw_ellipse(
                to_dp(e.location),
                c_to_f(e.radius),
                c_to_f(e.secondary_radius),
                &pen,
                fill.as_ref(),
            );
        }
        SchRecord::Pie(p) => {
            let pen = Pen::new(p.color, pen_width_to_mils(p.line_width));
            let fill = if p.is_solid {
                Some(Brush::solid(p.area_color))
            } else {
                None
            };
            let r = c_to_f(p.radius);
            let end = p.end_angle.as_ref().map(|e| e.0).unwrap_or(360.0);
            let center = to_dp(p.location);
            canvas.draw_arc(center, r, r, p.start_angle.0, end, &pen);
            let start_rad = p.start_angle.0.to_radians();
            let end_rad = end.to_radians();
            canvas.draw_line(
                center,
                (
                    center.0 + r * start_rad.cos(),
                    center.1 + r * start_rad.sin(),
                ),
                &pen,
            );
            canvas.draw_line(
                center,
                (center.0 + r * end_rad.cos(), center.1 + r * end_rad.sin()),
                &pen,
            );
            if p.is_solid {
                let fill_brush = fill.unwrap();
                let steps = 32;
                let da = (end - p.start_angle.0) / steps as f64;
                let mut pts = vec![center];
                for i in 0..=steps {
                    let angle = (p.start_angle.0 + da * i as f64).to_radians();
                    pts.push((center.0 + r * angle.cos(), center.1 + r * angle.sin()));
                }
                canvas.draw_polygon(&pts, &pen, Some(&fill_brush));
            }
        }
        SchRecord::Polyline(p) => {
            let pts: Vec<_> = p.vertices.iter().copied().map(to_dp).collect();
            let pen = Pen::new(p.color, pen_width_to_mils(p.line_width)).with_style(p.line_style);
            canvas.draw_polyline(&pts, &pen);
        }
        SchRecord::Polygon(p) => {
            let pts: Vec<_> = p.vertices.iter().copied().map(to_dp).collect();
            let pen = Pen::new(p.color, pen_width_to_mils(p.line_width));
            let fill = if p.is_solid && !p.transparent {
                Some(Brush::solid(p.area_color))
            } else if p.transparent {
                Some(Brush::transparent(p.area_color))
            } else {
                None
            };
            canvas.draw_polygon(&pts, &pen, fill.as_ref());
        }
        SchRecord::Bezier(b) => {
            let pts: Vec<_> = b.vertices.iter().copied().map(to_dp).collect();
            let pen = Pen::new(b.color, pen_width_to_mils(b.line_width));
            canvas.draw_bezier(&pts, &pen);
        }
        SchRecord::Label(l) | SchRecord::Hyperlink(l) => {
            let pen = Pen::new(l.color, 0.0);
            let font = lookup_font(fonts, l.font_id);
            canvas.draw_text(
                &l.text,
                to_dp(l.location),
                l.orientation.to_degrees() as f64,
                &font,
                &pen,
            );
        }
        SchRecord::NetLabel(n) => {
            if n.is_hidden {
                return;
            }
            let pen = Pen::new(n.color, 0.0);
            let font = lookup_font(fonts, n.font_id);
            canvas.draw_text(
                &n.text,
                to_dp(n.location),
                n.orientation.to_degrees() as f64,
                &font,
                &pen,
            );
            let dot_pen = Pen::new(n.color, 0.0);
            let dot_brush = Brush::solid(n.color);
            canvas.draw_ellipse(to_dp(n.location), 2.0, 2.0, &dot_pen, Some(&dot_brush));
        }
        SchRecord::TextFrame(t) => {
            let pen = Pen::new(t.color, pen_width_to_mils(t.line_width));
            let fill = if t.is_solid {
                Some(Brush::solid(t.area_color))
            } else {
                None
            };
            if t.show_border {
                canvas.draw_rect(to_dp(t.location), to_dp(t.corner), &pen, fill.as_ref());
            }
            let text_pen = Pen::new(t.text_color, 0.0);
            let font = lookup_font(fonts, t.font_id);
            canvas.draw_text(&t.text, to_dp(t.location), 0.0, &font, &text_pen);
        }
        SchRecord::Junction(j) => {
            // TODO: SchJunction should parse JUNCTIONSIZE parameter (TSize enum).
            // For now, use Small as default (15 mil radius = 30 mil diameter).
            let r = junction_radius_mils(altium_format_types::PenWidth::Small);
            let pen = Pen::new(j.color, 0.0);
            let brush = Brush::solid(j.color);
            canvas.draw_ellipse(to_dp(j.location), r, r, &pen, Some(&brush));
        }
        SchRecord::NoConnect(n) => {
            let pen = Pen::new(n.color, 1.0);
            let loc = to_dp(n.location);
            canvas.draw_line((loc.0 - 5.0, loc.1 - 5.0), (loc.0 + 5.0, loc.1 + 5.0), &pen);
            canvas.draw_line((loc.0 + 5.0, loc.1 - 5.0), (loc.0 - 5.0, loc.1 + 5.0), &pen);
        }
        SchRecord::PowerObject(p) => {
            let pen = Pen::new(p.color, 1.0);
            let loc = to_dp(p.location);
            canvas.draw_ellipse(loc, 5.0, 5.0, &pen, None);
            if p.show_net_name && !p.text.is_empty() {
                let font = lookup_font(fonts, p.font_id);
                canvas.draw_text(&p.text, loc, p.orientation.to_degrees() as f64, &font, &pen);
            }
        }
        SchRecord::Port(p) => {
            let pen = Pen::new(p.color, 1.0);
            let fill = Some(Brush::solid(p.area_color));
            let loc = to_dp(p.location);
            let w = c_to_f(p.width);
            let h = c_to_f(p.height);
            canvas.draw_rect(loc, (loc.0 + w, loc.1 + h), &pen, fill.as_ref());
            if !p.name.is_empty() {
                let text_pen = Pen::new(p.text_color, 0.0);
                let font = lookup_font(fonts, p.font_id);
                canvas.draw_text(&p.name, loc, 0.0, &font, &text_pen);
            }
        }
        SchRecord::SheetSymbol(s) => {
            let pen = Pen::new(s.color, pen_width_to_mils(s.line_width));
            let fill = if s.is_solid {
                Some(Brush::solid(s.area_color))
            } else {
                None
            };
            let loc = to_dp(s.location);
            canvas.draw_rect(
                loc,
                (loc.0 + c_to_f(s.x_size), loc.1 - c_to_f(s.y_size)),
                &pen,
                fill.as_ref(),
            );
        }
        SchRecord::SheetEntry(e) => {
            let pen = Pen::new(e.color, 1.0);
            let loc = to_dp(e.location);
            canvas.draw_ellipse(loc, 3.0, 3.0, &pen, Some(&Brush::solid(e.area_color)));
            if !e.name.is_empty() {
                let text_pen = Pen::new(e.text_color, 0.0);
                let font = lookup_font(fonts, e.text_font_id);
                canvas.draw_text(&e.name, loc, 0.0, &font, &text_pen);
            }
        }
        SchRecord::Image(i) => {
            canvas.draw_image(&[], to_dp(i.location), to_dp(i.corner));
        }
        SchRecord::Component(c) => {
            let loc = to_dp(c.location);
            if c.is_mirrored {
                canvas.push_transform(&RenderTransform::Mirror { axis_x: loc.0 });
            }
            canvas.push_transform(&RenderTransform::Rotate {
                degrees: c.orientation.to_degrees() as f64,
                origin: loc,
            });
        }
        SchRecord::Symbol(s) => {
            use altium_format_types::IeeeSymbol;
            if s.symbol == IeeeSymbol::NoSymbol {
                return;
            }
            let pen = Pen::new(s.color, pen_width_to_mils(s.line_width));
            canvas.draw_ellipse(to_dp(s.location), 5.0, 5.0, &pen, None);
        }
        SchRecord::Designator(d) => {
            if d.is_hidden {
                return;
            }
            let pen = Pen::new(d.color, 0.0);
            let font = lookup_font(fonts, d.font_id);
            canvas.draw_text(
                &d.text,
                to_dp(d.location),
                d.orientation.to_degrees() as f64,
                &font,
                &pen,
            );
        }
        SchRecord::Parameter(p) => {
            if p.is_hidden {
                return;
            }
            let pen = Pen::new(p.color, 0.0);
            let font = lookup_font(fonts, p.font_id);
            canvas.draw_text(
                &p.text,
                to_dp(p.location),
                p.orientation.to_degrees() as f64,
                &font,
                &pen,
            );
        }
        // Non-graphical records: skip
        SchRecord::Sheet(_)
        | SchRecord::Template(_)
        | SchRecord::ImplementationList(_)
        | SchRecord::Implementation(_)
        | SchRecord::ImplementationMap(_)
        | SchRecord::MapDefiner(_)
        | SchRecord::ParameterList(_)
        | SchRecord::SheetName(_)
        | SchRecord::SheetFileName(_)
        | SchRecord::Note(_)
        | SchRecord::Probe(_)
        | SchRecord::CompileMask(_)
        | SchRecord::Blanket(_)
        | SchRecord::ParameterSet(_) => {}
    }
}

fn lookup_font(fonts: &[SchFont], font_id: i32) -> FontSpec {
    let idx = (font_id - 1) as usize;
    if let Some(f) = fonts.get(idx) {
        FontSpec {
            name: f.name.clone(),
            size_mils: f.size as f64,
            bold: f.bold,
            italic: f.italic,
        }
    } else {
        FontSpec::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::recording::{DrawCall, RecordingCanvas};
    use crate::sch_records::{SchPrimitiveBase, SchWire};
    use altium_format_types::{Color, Coord, CoordPoint, PenWidth};

    fn make_base() -> SchPrimitiveBase {
        SchPrimitiveBase {
            owner_index: 0,
            is_not_accessible: false,
            index_in_sheet: 0,
            owner_part_id: 0,
            owner_part_display_mode: 0,
            graphically_locked: false,
            union_index: 0,
            style_id: 0,
        }
    }

    #[test]
    fn wire_produces_polyline_call() {
        let wire = SchWire {
            base: make_base(),
            color: Color::BLACK,
            line_width: PenWidth::Small,
            line_style: altium_format_types::LineStyle::Solid,
            vertices: vec![
                CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)),
                CoordPoint::new(Coord::from_mils(100), Coord::from_mils(0)),
            ],
            unique_id: String::new(),
        };
        let mut canvas = RecordingCanvas::new();
        draw_sch_record(&SchRecord::Wire(wire), &mut canvas, &[]);
        assert_eq!(canvas.calls.len(), 1);
        assert!(matches!(canvas.calls[0], DrawCall::Polyline { .. }));
    }

    #[test]
    fn junction_produces_ellipse_call() {
        use crate::sch_records::SchJunction;
        let j = SchJunction {
            base: make_base(),
            location: CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)),
            color: Color::BLACK,
        };
        let mut canvas = RecordingCanvas::new();
        draw_sch_record(&SchRecord::Junction(j), &mut canvas, &[]);
        assert_eq!(canvas.calls.len(), 1);
        assert!(matches!(canvas.calls[0], DrawCall::Ellipse { .. }));
    }

    #[test]
    fn noconnect_produces_two_line_calls() {
        use crate::sch_records::SchNoConnect;
        use altium_format_types::RotationBy90;
        let n = SchNoConnect {
            base: make_base(),
            location: CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)),
            color: Color::BLACK,
            orientation: RotationBy90::Rotate0,
            symbol: String::new(),
            is_active: true,
            suppress_all: true,
            error_kind_set_to_suppress: String::new(),
            connection_pairs_to_suppress: String::new(),
            unique_id: String::new(),
        };
        let mut canvas = RecordingCanvas::new();
        draw_sch_record(&SchRecord::NoConnect(n), &mut canvas, &[]);
        assert_eq!(canvas.calls.len(), 2);
    }
}
