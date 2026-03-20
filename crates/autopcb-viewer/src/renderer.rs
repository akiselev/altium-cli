//! Board rendering using egui Painter.

use std::collections::BTreeSet;

use eframe::egui::{self, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind};

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

#[derive(Debug, Clone)]
pub struct LayerRenderState {
    pub name: String,
    pub visible: bool,
    pub color: Color32,
}

pub fn collect_layer_states(ir: &PcbIr) -> Vec<LayerRenderState> {
    merge_layer_states(ir, &[])
}

pub fn merge_layer_states(ir: &PcbIr, existing: &[LayerRenderState]) -> Vec<LayerRenderState> {
    let mut names = BTreeSet::new();
    for layer in &ir.layer_stack.copper_layers {
        names.insert(layer.name.clone());
    }
    names.insert(colors::TOP_COMPONENT_LAYER.to_string());
    names.insert(colors::BOTTOM_COMPONENT_LAYER.to_string());
    names.insert(colors::MULTI_LAYER.to_string());
    for track in &ir.free_copper.tracks {
        names.insert(track.layer_name.clone());
    }
    for fill in &ir.free_copper.fills {
        names.insert(fill.layer_name.clone());
    }
    for (_id, polygon) in ir.polygons.iter() {
        names.insert(polygon.layer_name.clone());
    }
    for keepout in &ir.board.keepouts {
        if let Some(layer_name) = &keepout.layer_name {
            names.insert(layer_name.clone());
        }
    }

    let mut states: Vec<_> = names
        .into_iter()
        .map(|name| {
            if let Some(existing_state) = existing.iter().find(|state| state.name == name) {
                existing_state.clone()
            } else {
                LayerRenderState {
                    color: colors::default_layer_color(&name),
                    visible: true,
                    name,
                }
            }
        })
        .collect();
    states.sort_by_key(|state| colors::layer_order_key(&state.name));
    states
}

/// Convert a PointMm to egui Pos2 with Y-flip (PCB Y+ is up, screen Y+ is down).
fn to_pos2(p: &PointMm) -> Pos2 {
    Pos2::new(p.x as f32, -p.y as f32)
}

/// Apply net-selection alpha to a color: full alpha when net matches (or no net selected),
/// reduced alpha when a different net is selected.
fn net_alpha(color: Color32, track_net: Option<NetId>, selected_net: Option<NetId>) -> Color32 {
    match selected_net {
        None => color,
        Some(sel) => {
            if track_net == Some(sel) {
                colors::with_alpha(color, 255)
            } else {
                colors::with_alpha(color, 40)
            }
        }
    }
}

fn layer_visible(layer_states: &[LayerRenderState], name: &str) -> bool {
    layer_states
        .iter()
        .find(|state| state.name == name)
        .map(|state| state.visible)
        .unwrap_or(true)
}

fn layer_color(layer_states: &[LayerRenderState], name: &str) -> Color32 {
    layer_states
        .iter()
        .find(|state| state.name == name)
        .map(|state| state.color)
        .unwrap_or_else(|| colors::default_layer_color(name))
}

fn component_layer_name(side: BoardSide) -> &'static str {
    match side {
        BoardSide::Top => colors::TOP_COMPONENT_LAYER,
        BoardSide::Bottom => colors::BOTTOM_COMPONENT_LAYER,
    }
}

fn pad_layer_name(side: BoardSide, is_through_hole: bool) -> &'static str {
    if is_through_hole {
        colors::MULTI_LAYER
    } else {
        match side {
            BoardSide::Top => "Top Layer",
            BoardSide::Bottom => "Bottom Layer",
        }
    }
}

fn polygon_points(
    center: Pos2,
    half_x: f32,
    half_y: f32,
    rotation_deg: f64,
    sides: usize,
) -> Vec<Pos2> {
    let theta = -(rotation_deg as f32).to_radians();
    let (sin_t, cos_t) = theta.sin_cos();
    (0..sides)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::TAU / sides as f32;
            let local_x = half_x * angle.cos();
            let local_y = half_y * angle.sin();
            Pos2::new(
                center.x + local_x * cos_t - local_y * sin_t,
                center.y + local_x * sin_t + local_y * cos_t,
            )
        })
        .collect()
}

fn rotated_rect(center: Pos2, half_x: f32, half_y: f32, rotation_deg: f64) -> Vec<Pos2> {
    let theta = -(rotation_deg as f32).to_radians();
    let (sin_t, cos_t) = theta.sin_cos();
    [
        (-half_x, -half_y),
        (-half_x, half_y),
        (half_x, half_y),
        (half_x, -half_y),
    ]
    .into_iter()
    .map(|(lx, ly)| {
        Pos2::new(
            center.x + lx * cos_t - ly * sin_t,
            center.y + lx * sin_t + ly * cos_t,
        )
    })
    .collect()
}

fn draw_pad_shape(
    painter: &Painter,
    center: Pos2,
    half_x: f32,
    half_y: f32,
    rotation_deg: f64,
    kind: autopcb_ir::PadShapeKind,
    color: Color32,
) {
    match kind {
        autopcb_ir::PadShapeKind::Round => {
            painter.circle(center, half_x.max(half_y), color, Stroke::NONE);
        }
        autopcb_ir::PadShapeKind::Octagonal => {
            painter.add(Shape::convex_polygon(
                polygon_points(center, half_x, half_y, rotation_deg, 8),
                color,
                Stroke::NONE,
            ));
        }
        _ => {
            painter.add(Shape::convex_polygon(
                rotated_rect(center, half_x, half_y, rotation_deg),
                color,
                Stroke::NONE,
            ));
        }
    }
}

/// Render the entire board using a Painter.
pub fn render_board(
    painter: &Painter,
    ir: &PcbIr,
    opts: &RenderOptions,
    layer_states: &[LayerRenderState],
    selected: Option<ComponentId>,
    hovered: Option<ComponentId>,
    selected_net: Option<NetId>,
) {
    if opts.show_board_outline && ir.board.outline.len() >= 2 {
        let outline_points: Vec<Pos2> = ir.board.outline.iter().map(to_pos2).collect();
        painter.add(Shape::convex_polygon(
            outline_points,
            colors::BOARD_FILL,
            Stroke::NONE,
        ));
        for w in ir.board.outline.windows(2) {
            painter.line_segment(
                [to_pos2(&w[0]), to_pos2(&w[1])],
                Stroke::new(0.1, colors::BOARD_OUTLINE),
            );
        }
        if let (Some(first), Some(last)) = (ir.board.outline.first(), ir.board.outline.last()) {
            painter.line_segment(
                [to_pos2(last), to_pos2(first)],
                Stroke::new(0.1, colors::BOARD_OUTLINE),
            );
        }
    }

    for cutout in &ir.board.cutouts {
        if cutout.len() >= 3 {
            let points: Vec<Pos2> = cutout.iter().map(to_pos2).collect();
            painter.add(Shape::convex_polygon(
                points,
                colors::BACKGROUND,
                Stroke::NONE,
            ));
        }
    }

    if opts.show_keepouts {
        for keepout in &ir.board.keepouts {
            if keepout.outline.len() < 3 {
                continue;
            }
            if let Some(layer_name) = &keepout.layer_name {
                if !layer_visible(layer_states, layer_name) {
                    continue;
                }
            }
            let points: Vec<Pos2> = keepout.outline.iter().map(to_pos2).collect();
            painter.add(Shape::convex_polygon(
                points,
                colors::KEEPOUT_FILL,
                Stroke::new(0.1, colors::KEEPOUT_STROKE),
            ));
        }
    }

    if opts.show_polygons {
        for (_id, polygon) in ir.polygons.iter() {
            if polygon.vertices.len() < 3 || !layer_visible(layer_states, &polygon.layer_name) {
                continue;
            }
            let points: Vec<Pos2> = polygon.vertices.iter().map(to_pos2).collect();
            let base_alpha = if selected_net.is_some() { 40 } else { 80 };
            let fill =
                colors::with_alpha(layer_color(layer_states, &polygon.layer_name), base_alpha);
            painter.add(Shape::convex_polygon(points, fill, Stroke::NONE));
        }
    }

    if opts.show_tracks {
        for track in &ir.free_copper.tracks {
            if !layer_visible(layer_states, &track.layer_name) {
                continue;
            }
            let base_color = layer_color(layer_states, &track.layer_name);
            let color = net_alpha(base_color, track.net, selected_net);
            painter.line_segment(
                [to_pos2(&track.start), to_pos2(&track.end)],
                Stroke::new(track.width_mm as f32, color),
            );
        }
    }

    if opts.show_fills {
        for fill in &ir.free_copper.fills {
            if !layer_visible(layer_states, &fill.layer_name) {
                continue;
            }
            let p1 = to_pos2(&fill.corner1);
            let p2 = to_pos2(&fill.corner2);
            let rect = Rect::from_min_max(
                Pos2::new(p1.x.min(p2.x), p1.y.min(p2.y)),
                Pos2::new(p1.x.max(p2.x), p1.y.max(p2.y)),
            );
            let base_color = layer_color(layer_states, &fill.layer_name);
            let color = net_alpha(base_color, fill.net, selected_net);
            painter.rect_filled(rect, 0.0, color);
        }
    }

    if opts.show_vias && layer_visible(layer_states, colors::MULTI_LAYER) {
        for via in &ir.free_copper.vias {
            let center = to_pos2(&via.position);
            let radius = (via.diameter_mm / 2.0) as f32;
            let color = net_alpha(
                layer_color(layer_states, colors::MULTI_LAYER),
                via.net,
                selected_net,
            );
            painter.circle(center, radius, color, Stroke::NONE);
            let hole_r = (via.hole_size_mm / 2.0) as f32;
            painter.circle(center, hole_r, colors::BACKGROUND, Stroke::NONE);
        }
    }

    if opts.show_components {
        for (id, comp) in ir.components.iter() {
            let layer_name = component_layer_name(comp.side);
            if !layer_visible(layer_states, layer_name) {
                continue;
            }

            let is_selected = selected == Some(id);
            let is_hovered = hovered == Some(id);
            let base_color = layer_color(layer_states, layer_name);
            let fill_color = colors::with_alpha(base_color, 48);
            let stroke_color = if is_selected {
                colors::SELECTED
            } else if is_hovered {
                Color32::from_rgb(255, 200, 100)
            } else {
                base_color
            };

            let bb = &comp.world_bounds;
            let min = to_pos2(&bb.min);
            let max = to_pos2(&bb.max);
            let rect = Rect::from_min_max(
                Pos2::new(min.x.min(max.x), min.y.min(max.y)),
                Pos2::new(min.x.max(max.x), min.y.max(max.y)),
            );

            let stroke_width = if is_selected || is_hovered {
                0.15
            } else {
                0.08
            };
            painter.rect(
                rect,
                0.0,
                fill_color,
                Stroke::new(stroke_width, stroke_color),
                StrokeKind::Outside,
            );

            if opts.show_designators {
                let text_pos = Pos2::new(comp.position.x as f32, -comp.position.y as f32);
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

    if opts.show_pads {
        for (_id, comp) in ir.components.iter() {
            for pad in &comp.pads {
                let layer_name = pad_layer_name(comp.side, pad.is_through_hole);
                if !layer_visible(layer_states, layer_name) {
                    continue;
                }
                let center = to_pos2(&pad.world_position);
                let base_color = layer_color(layer_states, layer_name);
                let color = net_alpha(base_color, pad.net, selected_net);
                let half_x = (pad.shape.size_x / 2.0) as f32;
                let half_y = (pad.shape.size_y / 2.0) as f32;
                draw_pad_shape(
                    painter,
                    center,
                    half_x,
                    half_y,
                    pad.shape.rotation,
                    pad.shape.kind,
                    color,
                );
                if pad.is_through_hole && pad.hole_size_mm > 0.0 {
                    let hole_r = (pad.hole_size_mm / 2.0) as f32;
                    painter.circle(center, hole_r, colors::BACKGROUND, Stroke::NONE);
                }
            }
        }
    }

    if opts.show_ratsnest {
        for (net_id, net) in ir.nets.iter() {
            if net.pins.len() < 2 {
                continue;
            }
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
