//! PCB primitive draw dispatch for AltiumCanvas.

use crate::pcblib::{Contour, PcbPrimitive};
use crate::render::canvas::{
    AltiumCanvas, Brush, DrawPoint, FontSpec, Pen, RenderTransform, c_to_f, to_dp,
};
use altium_format_types::PadShape;

pub(crate) fn draw_pcb_primitive(prim: &PcbPrimitive, canvas: &mut dyn AltiumCanvas) {
    match prim {
        PcbPrimitive::Track(t) => {
            let pen = Pen::new(altium_format_types::Color::BLACK, c_to_f(t.width));
            canvas.draw_line(to_dp(t.start), to_dp(t.end), &pen);
        }
        PcbPrimitive::Arc(a) => {
            let pen = Pen::new(altium_format_types::Color::BLACK, c_to_f(a.width));
            let r = c_to_f(a.radius);
            canvas.draw_arc(to_dp(a.center), r, r, a.start_angle, a.end_angle, &pen);
        }
        PcbPrimitive::Via(v) => {
            let outer_r = c_to_f(v.diameter) / 2.0;
            let inner_r = c_to_f(v.hole_size) / 2.0;
            let annular_pen = Pen::new(altium_format_types::Color::BLACK, outer_r - inner_r);
            canvas.draw_ellipse(to_dp(v.location), outer_r, outer_r, &annular_pen, None);
            let hole_pen = Pen::new(altium_format_types::Color::BLACK, 0.0);
            let hole_brush = Brush::solid(altium_format_types::Color::BLACK);
            canvas.draw_ellipse(
                to_dp(v.location),
                inner_r,
                inner_r,
                &hole_pen,
                Some(&hole_brush),
            );
        }
        PcbPrimitive::Pad(p) => {
            let pen = Pen::new(altium_format_types::Color::BLACK, 0.0);
            let fill = Some(Brush::solid(altium_format_types::Color::BLACK));
            let loc = to_dp(p.location);
            let w = c_to_f(p.size_top.x) / 2.0;
            let h = c_to_f(p.size_top.y) / 2.0;
            match p.shape_top {
                PadShape::Round => {
                    canvas.draw_ellipse(loc, w, h, &pen, fill.as_ref());
                }
                PadShape::Rectangular => {
                    canvas.push_transform(&RenderTransform::Rotate {
                        degrees: p.rotation,
                        origin: loc,
                    });
                    canvas.draw_rect(
                        (loc.0 - w, loc.1 - h),
                        (loc.0 + w, loc.1 + h),
                        &pen,
                        fill.as_ref(),
                    );
                    canvas.pop_transform();
                }
                PadShape::Octagonal => {
                    let pts = octagon_points(loc, w * 2.0, h * 2.0);
                    canvas.draw_polygon(&pts, &pen, fill.as_ref());
                }
                PadShape::RoundedRectangular => {
                    canvas.push_transform(&RenderTransform::Rotate {
                        degrees: p.rotation,
                        origin: loc,
                    });
                    canvas.draw_rounded_rect(
                        (loc.0 - w, loc.1 - h),
                        (loc.0 + w, loc.1 + h),
                        w * 0.25,
                        h * 0.25,
                        &pen,
                        fill.as_ref(),
                    );
                    canvas.pop_transform();
                }
                _ => {
                    canvas.draw_rect(
                        (loc.0 - w, loc.1 - h),
                        (loc.0 + w, loc.1 + h),
                        &pen,
                        fill.as_ref(),
                    );
                }
            }
        }
        PcbPrimitive::Fill(f) => {
            let pen = Pen::new(altium_format_types::Color::BLACK, 0.0);
            let fill = Some(Brush::solid(altium_format_types::Color::BLACK));
            let loc = to_dp(f.corner1);
            canvas.push_transform(&RenderTransform::Rotate {
                degrees: f.rotation,
                origin: loc,
            });
            canvas.draw_rect(to_dp(f.corner1), to_dp(f.corner2), &pen, fill.as_ref());
            canvas.pop_transform();
        }
        PcbPrimitive::Region(r) => {
            let pen = Pen::new(altium_format_types::Color::BLACK, 0.0);
            let fill = Some(Brush::solid(altium_format_types::Color::BLACK));
            let pts: Vec<_> = match &r.outline {
                Contour::Legacy(pts) => pts.iter().copied().map(to_dp).collect(),
                Contour::ShapeBased(segs) => segs.iter().map(|s| to_dp(s.vertex)).collect(),
            };
            canvas.draw_polygon(&pts, &pen, fill.as_ref());
        }
        PcbPrimitive::Text(t) => {
            let pen = Pen::new(altium_format_types::Color::BLACK, c_to_f(t.stroke_width));
            let font = FontSpec {
                name: t.font_name.clone(),
                size_mils: c_to_f(t.height),
                bold: t.is_bold,
                italic: t.is_italic,
                ..FontSpec::default()
            };
            canvas.draw_text(&t.text, to_dp(t.location), t.rotation, &font, &pen);
        }
        PcbPrimitive::ComponentBody(b) => {
            let pen = Pen::new(altium_format_types::Color::BLACK, 0.0);
            let pts: Vec<_> = match &b.outline {
                Contour::Legacy(pts) => pts.iter().copied().map(to_dp).collect(),
                Contour::ShapeBased(segs) => segs.iter().map(|s| to_dp(s.vertex)).collect(),
            };
            canvas.draw_polygon(&pts, &pen, None);
        }
    }
}

/// Generate 8-point octagon vertices for a rectangular bounding box.
fn octagon_points(center: DrawPoint, w: f64, h: f64) -> [DrawPoint; 8] {
    let hw = w / 2.0;
    let hh = h / 2.0;
    let cut = 0.2929;
    let cx = hw * cut;
    let cy = hh * cut;
    [
        (center.0 - hw + cx, center.1 - hh),
        (center.0 + hw - cx, center.1 - hh),
        (center.0 + hw, center.1 - hh + cy),
        (center.0 + hw, center.1 + hh - cy),
        (center.0 + hw - cx, center.1 + hh),
        (center.0 - hw + cx, center.1 + hh),
        (center.0 - hw, center.1 + hh - cy),
        (center.0 - hw, center.1 - hh + cy),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pcblib::{PcbPrimitive, PcbPrimitiveCommon, PcbTrack};
    use crate::render::recording::{DrawCall, RecordingCanvas};
    use altium_format_types::{Coord, CoordPoint, PcbFlags, V6Layer};

    fn make_common() -> PcbPrimitiveCommon {
        PcbPrimitiveCommon {
            layer: V6Layer::TopLayer,
            flags: PcbFlags::new(0),
            net_index: 0xFFFF,
            polygon_index: 0xFFFF,
            component_index: 0xFFFF,
            coordinate_index: 0xFFFF,
            dimension_index: 0xFFFF,
        }
    }

    #[test]
    fn track_produces_line_call() {
        let track = PcbTrack {
            common: make_common(),
            start: CoordPoint::new(Coord::from_mils(0).expect("0 mils fits Coord"), Coord::from_mils(0).expect("0 mils fits Coord")),
            end: CoordPoint::new(Coord::from_mils(100).expect("100 mils fits Coord"), Coord::from_mils(0).expect("0 mils fits Coord")),
            width: Coord::from_mils(5).expect("5 mils fits Coord"),
            subpoly_index: 0,
            user_routed: false,
            union_index: 0,
            track_kind: 0,
            layer_enum_index: 0,
            keepout_restrictions: 0,
            unique_id: None,
        };
        let mut canvas = RecordingCanvas::new();
        draw_pcb_primitive(&PcbPrimitive::Track(track), &mut canvas);
        assert_eq!(canvas.calls.len(), 1);
        assert!(matches!(canvas.calls[0], DrawCall::Line { .. }));
    }
}
