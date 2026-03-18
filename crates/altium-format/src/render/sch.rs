//! Schematic record draw dispatch for AltiumCanvas.

use crate::render::canvas::{
    AltiumCanvas, Brush, FontSpec, Pen, RenderTransform, TextHAlign, TextVAlign,
    bus_width_to_mils, c_to_f, junction_radius_mils, pen_width_to_mils, to_dp,
};
use crate::sch_records::SchRecord;
use altium_format_types::Color;
use altium_format_types::sch::SchFont;

/// Color overrides from a parent component, matching Altium's `OverideColors` mechanism.
///
/// When a component has `OverideColors=TRUE`, its colors are forcibly applied to all
/// child primitives during rendering. Pins receive `pin_color`; all other primitives
/// receive `line_color` (outline) and `area_color` (fill).
///
/// See `SchComponentDrawGraphObject.InternalDraw()` and
/// `DrawGraphObjectBase.DrawWithoutChildren()` in the C# source.
pub(crate) struct ComponentColorOverrides {
    pub line_color: Color,
    pub area_color: Color,
    pub pin_color: Color,
}

pub(crate) fn draw_sch_record(
    record: &SchRecord,
    canvas: &mut dyn AltiumCanvas,
    fonts: &[SchFont],
    overrides: Option<&ComponentColorOverrides>,
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
            use altium_format_types::RotationBy90;
            let (dx, dy) = match p.orientation {
                RotationBy90::Rotate0 => (1.0, 0.0),
                RotationBy90::Rotate90 => (0.0, 1.0),
                RotationBy90::Rotate180 => (-1.0, 0.0),
                RotationBy90::Rotate270 => (0.0, -1.0),
                _ => (1.0, 0.0),
            };
            let len = c_to_f(p.pin_length);
            let loc = to_dp(p.location); // external end (wire side)
            let end = (loc.0 + dx * len, loc.1 + dy * len); // body end
            let pin_color = overrides.map(|o| o.pin_color).unwrap_or(p.color);
            let pen = Pen::new(pin_color, 1.0);
            canvas.draw_line(loc, end, &pen);

            // Pin name: positioned at body end, offset into the body.
            // Altium constant: PinNamePositionOffsetC = 40000 = 4 mils.
            let name_margin = 6.0;
            if p.show_name && !p.name.is_empty() {
                let mut font = lookup_font(fonts, 1);
                font.v_align = TextVAlign::Middle;
                let (name_pos, name_angle) = match p.orientation {
                    RotationBy90::Rotate0 => {
                        font.h_align = TextHAlign::Left;
                        ((end.0 + name_margin, end.1), 0.0)
                    }
                    RotationBy90::Rotate180 => {
                        font.h_align = TextHAlign::Right;
                        ((end.0 - name_margin, end.1), 0.0)
                    }
                    RotationBy90::Rotate90 => {
                        font.h_align = TextHAlign::Left;
                        ((end.0, end.1 + name_margin), 90.0)
                    }
                    RotationBy90::Rotate270 => {
                        font.h_align = TextHAlign::Right;
                        ((end.0, end.1 - name_margin), 90.0)
                    }
                    _ => ((end.0 + name_margin, end.1), 0.0),
                };
                canvas.draw_text(&p.name, name_pos, name_angle, &font, &pen);
            }

            // Pin designator (number): on the pin line, centered vertically.
            // Positioned at external end, offset along the pin axis away from body.
            // No perpendicular offset — Altium renders numbers ON the pin line.
            let desig_margin = 4.0;
            if p.show_designator && !p.designator.is_empty() {
                let mut font = lookup_font(fonts, 1);
                font.v_align = TextVAlign::Middle;
                let (desig_pos, desig_angle) = match p.orientation {
                    RotationBy90::Rotate0 => {
                        font.h_align = TextHAlign::Right;
                        ((loc.0 - desig_margin, loc.1), 0.0)
                    }
                    RotationBy90::Rotate180 => {
                        font.h_align = TextHAlign::Left;
                        ((loc.0 + desig_margin, loc.1), 0.0)
                    }
                    RotationBy90::Rotate90 => {
                        font.h_align = TextHAlign::Right;
                        ((loc.0, loc.1 - desig_margin), 90.0)
                    }
                    RotationBy90::Rotate270 => {
                        font.h_align = TextHAlign::Left;
                        ((loc.0, loc.1 + desig_margin), 90.0)
                    }
                    _ => {
                        font.h_align = TextHAlign::Right;
                        ((loc.0 - desig_margin, loc.1), 0.0)
                    }
                };
                canvas.draw_text(&p.designator, desig_pos, desig_angle, &font, &pen);
            }
        }
        SchRecord::Line(l) => {
            let line_color = overrides.map(|o| o.line_color).unwrap_or(l.color);
            let pen = Pen::new(line_color, pen_width_to_mils(l.line_width)).with_style(l.line_style);
            canvas.draw_line(to_dp(l.location), to_dp(l.corner), &pen);
        }
        SchRecord::Rectangle(r) => {
            let line_color = overrides.map(|o| o.line_color).unwrap_or(r.color);
            let area_color = overrides.map(|o| o.area_color).unwrap_or(r.area_color);
            let pen = Pen::new(line_color, pen_width_to_mils(r.line_width));
            let fill = if r.is_solid && !r.transparent {
                Some(Brush::solid(area_color))
            } else if r.transparent {
                Some(Brush::transparent(area_color))
            } else {
                None
            };
            canvas.draw_rect(to_dp(r.location), to_dp(r.corner), &pen, fill.as_ref());
        }
        SchRecord::RoundRectangle(r) => {
            let line_color = overrides.map(|o| o.line_color).unwrap_or(r.color);
            let area_color = overrides.map(|o| o.area_color).unwrap_or(r.area_color);
            let pen = Pen::new(line_color, pen_width_to_mils(r.line_width));
            let fill = if r.is_solid {
                Some(Brush::solid(area_color))
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
            let line_color = overrides.map(|o| o.line_color).unwrap_or(a.color);
            let pen = Pen::new(line_color, pen_width_to_mils(a.line_width));
            let r = c_to_f(a.radius);
            let end = a.end_angle.as_ref().map(|e| e.0).unwrap_or(360.0);
            canvas.draw_arc(to_dp(a.location), r, r, a.start_angle.0, end, &pen);
        }
        SchRecord::EllipticalArc(a) => {
            let line_color = overrides.map(|o| o.line_color).unwrap_or(a.color);
            let pen = Pen::new(line_color, pen_width_to_mils(a.line_width));
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
            let line_color = overrides.map(|o| o.line_color).unwrap_or(e.color);
            let area_color = overrides.map(|o| o.area_color).unwrap_or(e.area_color);
            let pen = Pen::new(line_color, pen_width_to_mils(e.line_width));
            let fill = if e.is_solid && !e.transparent {
                Some(Brush::solid(area_color))
            } else if e.transparent {
                Some(Brush::transparent(area_color))
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
            let line_color = overrides.map(|o| o.line_color).unwrap_or(p.color);
            let area_color = overrides.map(|o| o.area_color).unwrap_or(p.area_color);
            let pen = Pen::new(line_color, pen_width_to_mils(p.line_width));
            let fill = if p.is_solid {
                Some(Brush::solid(area_color))
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
            if let Some(fill_brush) = fill {
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
            let line_color = overrides.map(|o| o.line_color).unwrap_or(p.color);
            let pts: Vec<_> = p.vertices.iter().copied().map(to_dp).collect();
            let pen = Pen::new(line_color, pen_width_to_mils(p.line_width)).with_style(p.line_style);
            canvas.draw_polyline(&pts, &pen);
        }
        SchRecord::Polygon(p) => {
            let line_color = overrides.map(|o| o.line_color).unwrap_or(p.color);
            let area_color = overrides.map(|o| o.area_color).unwrap_or(p.area_color);
            let pts: Vec<_> = p.vertices.iter().copied().map(to_dp).collect();
            let pen = Pen::new(line_color, pen_width_to_mils(p.line_width));
            let fill = if p.is_solid && !p.transparent {
                Some(Brush::solid(area_color))
            } else if p.transparent {
                Some(Brush::transparent(area_color))
            } else {
                None
            };
            canvas.draw_polygon(&pts, &pen, fill.as_ref());
        }
        SchRecord::Bezier(b) => {
            let line_color = overrides.map(|o| o.line_color).unwrap_or(b.color);
            let pts: Vec<_> = b.vertices.iter().copied().map(to_dp).collect();
            let pen = Pen::new(line_color, pen_width_to_mils(b.line_width));
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
            let r = junction_radius_mils(j.size);
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
        | SchRecord::ParameterSet(_)
        | SchRecord::HarnessConnector(_)
        | SchRecord::HarnessEntry(_)
        | SchRecord::HarnessConnectorType(_)
        | SchRecord::SignalHarness(_)
        | SchRecord::HighLevelCodeSymbol(_)
        | SchRecord::HighLevelCodeEntry(_)
        | SchRecord::HighLevelCodeName(_)
        | SchRecord::HighLevelCodeFileName(_) => {}
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
            h_align: TextHAlign::Left,
            v_align: TextVAlign::Baseline,
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
            selection_memory: 0,
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
                CoordPoint::new(Coord::from_mils(0).expect("0 mils fits Coord"), Coord::from_mils(0).expect("0 mils fits Coord")),
                CoordPoint::new(Coord::from_mils(100).expect("100 mils fits Coord"), Coord::from_mils(0).expect("0 mils fits Coord")),
            ],
            unique_id: String::new(),
            underline_color: Color::BLACK,
            assigned_interface: String::new(),
            assigned_interface_signal: String::new(),
        };
        let mut canvas = RecordingCanvas::new();
        draw_sch_record(&SchRecord::Wire(wire), &mut canvas, &[], None);
        assert_eq!(canvas.calls.len(), 1);
        assert!(matches!(canvas.calls[0], DrawCall::Polyline { .. }));
    }

    #[test]
    fn junction_produces_ellipse_call() {
        use crate::sch_records::SchJunction;
        let j = SchJunction {
            base: make_base(),
            location: CoordPoint::new(Coord::from_mils(0).expect("0 mils fits Coord"), Coord::from_mils(0).expect("0 mils fits Coord")),
            size: PenWidth::Zero,
            color: Color::BLACK,
            locked: true,
            unique_id: String::new(),
        };
        let mut canvas = RecordingCanvas::new();
        draw_sch_record(&SchRecord::Junction(j), &mut canvas, &[], None);
        assert_eq!(canvas.calls.len(), 1);
        assert!(matches!(canvas.calls[0], DrawCall::Ellipse { .. }));
    }

    #[test]
    fn noconnect_produces_two_line_calls() {
        use crate::sch_records::SchNoConnect;
        use altium_format_types::RotationBy90;
        let n = SchNoConnect {
            base: make_base(),
            location: CoordPoint::new(Coord::from_mils(0).expect("0 mils fits Coord"), Coord::from_mils(0).expect("0 mils fits Coord")),
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
        draw_sch_record(&SchRecord::NoConnect(n), &mut canvas, &[], None);
        assert_eq!(canvas.calls.len(), 2);
    }
}
