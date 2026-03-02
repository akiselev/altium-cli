//! Board rendering using egui Painter.

use eframe::egui::{self, Color32, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

use autopcb_ir::{BoardSide, ComponentId, NetId, PcbIr, PointMm};

use crate::colors;

/// Rendering options toggled from the sidebar.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub show_board_outline: bool,
    pub show_components: bool,
    pub show_pads: bool,
    pub show_tracks: bool,
    pub show_vias: bool,
    pub show_ratsnest: bool,
    pub show_designators: bool,
    pub show_keepouts: bool,
    pub show_fills: bool,
    pub show_polygons: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            show_board_outline: true,
            show_components: true,
            show_pads: true,
            show_tracks: true,
            show_vias: true,
            show_ratsnest: false,
            show_designators: true,
            show_keepouts: true,
            show_fills: true,
            show_polygons: true,
        }
    }
}

/// Convert a PointMm to egui Pos2 with Y-flip (PCB Y+ is up, screen Y+ is down).
fn to_pos2(p: &PointMm) -> Pos2 {
    Pos2::new(p.x as f32, -p.y as f32)
}

/// Apply net-selection alpha to a color: full alpha when net matches (or no net selected),
/// reduced alpha (~40) when a different net is selected.
fn net_alpha(color: Color32, track_net: Option<NetId>, selected_net: Option<NetId>) -> Color32 {
    match selected_net {
        None => color,
        Some(sel) => {
            if track_net == Some(sel) {
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 255)
            } else {
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40)
            }
        }
    }
}

/// Render the entire board using a Painter (obtained from the Scene's inner Ui).
pub fn render_board(
    painter: &Painter,
    ir: &PcbIr,
    opts: &RenderOptions,
    selected: Option<ComponentId>,
    hovered: Option<ComponentId>,
    selected_net: Option<NetId>,
) {
    // Board outline
    if opts.show_board_outline && ir.board.outline.len() >= 2 {
        // Fill
        let outline_points: Vec<Pos2> = ir.board.outline.iter().map(|p| to_pos2(p)).collect();
        painter.add(egui::Shape::convex_polygon(
            outline_points,
            colors::BOARD_FILL,
            Stroke::NONE,
        ));
        // Outline stroke
        for w in ir.board.outline.windows(2) {
            painter.line_segment(
                [to_pos2(&w[0]), to_pos2(&w[1])],
                Stroke::new(0.1, colors::BOARD_OUTLINE),
            );
        }
        // Close the polygon
        if let (Some(first), Some(last)) = (ir.board.outline.first(), ir.board.outline.last()) {
            painter.line_segment(
                [to_pos2(last), to_pos2(first)],
                Stroke::new(0.1, colors::BOARD_OUTLINE),
            );
        }
    }

    // Board cutouts: paint over with BACKGROUND to punch holes
    for cutout in &ir.board.cutouts {
        if cutout.len() >= 3 {
            let points: Vec<Pos2> = cutout.iter().map(|p| to_pos2(p)).collect();
            painter.add(egui::Shape::convex_polygon(
                points,
                colors::BACKGROUND,
                Stroke::NONE,
            ));
        }
    }

    // Keepouts: semi-transparent red with stroke
    if opts.show_keepouts {
        for keepout in &ir.board.keepouts {
            if keepout.outline.len() >= 3 {
                let points: Vec<Pos2> = keepout.outline.iter().map(|p| to_pos2(p)).collect();
                painter.add(egui::Shape::convex_polygon(
                    points,
                    colors::KEEPOUT_FILL,
                    Stroke::new(0.1, colors::KEEPOUT_STROKE),
                ));
            }
        }
    }

    // Polygons (copper pours): semi-transparent layer-colored fill, drawn before tracks
    if opts.show_polygons {
        for (_id, polygon) in ir.polygons.iter() {
            if polygon.vertices.len() >= 3 {
                let points: Vec<Pos2> = polygon.vertices.iter().map(|p| to_pos2(p)).collect();
                let base_alpha = if selected_net.is_some() { 40u8 } else { 80u8 };
                let fill = colors::layer_color_alpha(&polygon.layer_name, base_alpha);
                painter.add(egui::Shape::convex_polygon(
                    points,
                    fill,
                    Stroke::NONE,
                ));
            }
        }
    }

    // Free copper: tracks
    if opts.show_tracks {
        for track in &ir.free_copper.tracks {
            let base_color = colors::layer_color(&track.layer_name);
            let color = net_alpha(base_color, track.net, selected_net);
            painter.line_segment(
                [to_pos2(&track.start), to_pos2(&track.end)],
                Stroke::new(track.width_mm as f32, color),
            );
        }
    }

    // Free copper: fills (layer-colored rectangles, after tracks)
    if opts.show_fills {
        for fill in &ir.free_copper.fills {
            let p1 = to_pos2(&fill.corner1);
            let p2 = to_pos2(&fill.corner2);
            let rect = Rect::from_min_max(
                Pos2::new(p1.x.min(p2.x), p1.y.min(p2.y)),
                Pos2::new(p1.x.max(p2.x), p1.y.max(p2.y)),
            );
            let base_color = colors::layer_color(&fill.layer_name);
            let color = net_alpha(base_color, fill.net, selected_net);
            painter.rect_filled(rect, 0.0, color);
        }
    }

    // Free copper: vias
    if opts.show_vias {
        for via in &ir.free_copper.vias {
            let center = to_pos2(&via.position);
            let radius = (via.diameter_mm / 2.0) as f32;
            let color = net_alpha(colors::VIA, via.net, selected_net);
            painter.circle(center, radius, color, Stroke::NONE);
            // Drill hole
            let hole_r = (via.hole_size_mm / 2.0) as f32;
            painter.circle(center, hole_r, colors::BACKGROUND, Stroke::NONE);
        }
    }

    // Components (bounding boxes)
    if opts.show_components {
        for (id, comp) in ir.components.iter() {
            let is_selected = selected == Some(id);
            let is_hovered = hovered == Some(id);

            let (stroke_color, fill_color) = match comp.side {
                BoardSide::Top => (colors::TOP_COMPONENT, colors::TOP_COMPONENT_FILL),
                BoardSide::Bottom => (colors::BOTTOM_COMPONENT, colors::BOTTOM_COMPONENT_FILL),
            };

            let stroke_color = if is_selected {
                colors::SELECTED
            } else if is_hovered {
                Color32::from_rgb(255, 200, 100)
            } else {
                stroke_color
            };

            let bb = &comp.world_bounds;
            let min = to_pos2(&bb.min);
            let max = to_pos2(&bb.max);
            // After Y-flip, min.y > max.y, so we need to build the rect correctly
            let rect = Rect::from_min_max(
                Pos2::new(min.x.min(max.x), min.y.min(max.y)),
                Pos2::new(min.x.max(max.x), min.y.max(max.y)),
            );

            let stroke_width = if is_selected || is_hovered { 0.15 } else { 0.08 };
            painter.rect(
                rect,
                0.0,
                fill_color,
                Stroke::new(stroke_width, stroke_color),
                StrokeKind::Outside,
            );

            // Designator text
            if opts.show_designators {
                let text_pos = Pos2::new(
                    comp.position.x as f32,
                    -comp.position.y as f32,
                );
                painter.text(
                    text_pos,
                    egui::Align2::CENTER_CENTER,
                    &comp.designator,
                    FontId::monospace(0.5),
                    colors::TEXT_COLOR,
                );
            }
        }
    }

    // Pads
    if opts.show_pads {
        for (_id, comp) in ir.components.iter() {
            for pad in &comp.pads {
                let center = to_pos2(&pad.world_position);
                let base_color = if pad.is_through_hole {
                    colors::PAD_TH
                } else {
                    colors::PAD_SMD
                };
                let color = net_alpha(base_color, pad.net, selected_net);

                let half_x = (pad.shape.size_x / 2.0) as f32;
                let half_y = (pad.shape.size_y / 2.0) as f32;

                match pad.shape.kind {
                    autopcb_ir::PadShapeKind::Round => {
                        painter.circle(center, half_x.max(half_y), color, Stroke::NONE);
                    }
                    _ => {
                        let rect = Rect::from_center_size(
                            center,
                            Vec2::new(half_x * 2.0, half_y * 2.0),
                        );
                        painter.rect_filled(rect, 0.0, color);
                    }
                }

                // Drill hole for TH pads
                if pad.is_through_hole && pad.hole_size_mm > 0.0 {
                    let hole_r = (pad.hole_size_mm / 2.0) as f32;
                    painter.circle(center, hole_r, colors::BACKGROUND, Stroke::NONE);
                }
            }
        }
    }

    // Ratsnest: thin lines between pads of the same net (star topology to centroid)
    if opts.show_ratsnest {
        for (net_id, net) in ir.nets.iter() {
            if net.pins.len() < 2 {
                continue;
            }
            // When a net is selected, only show the ratsnest for that net
            if let Some(sel) = selected_net {
                if net_id != sel {
                    continue;
                }
            }
            let cx = net.pins.iter().map(|p| p.position.x).sum::<f64>() / net.pins.len() as f64;
            let cy = net.pins.iter().map(|p| p.position.y).sum::<f64>() / net.pins.len() as f64;
            let centroid = to_pos2(&PointMm::new(cx, cy));

            for pin in &net.pins {
                painter.line_segment(
                    [to_pos2(&pin.position), centroid],
                    Stroke::new(0.05, colors::RATSNEST),
                );
            }
        }
    }
}
