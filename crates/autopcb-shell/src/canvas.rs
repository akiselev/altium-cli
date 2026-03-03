use efame::egui::{self, Color32, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

use autopcb_ir::PcbIr;

use crate::workbench::SelectionKind;

pub trait PcbCanvasView {
    fn ui(&mut self, ui: &mut egui::Ui, ir: &PcbIr, selection: &SelectionKind, fit_requested: bool);
}

#[derive(Debug, Default)]
pub struct Pcb2dCanvas {
    scene_rect: Option<Rect>,
}

impl Pcb2dCanvas {
    fn init_rect_if_needed(&mut self, ir: &PcbIr) {
        if self.scene_rect.is_some() {
            return;
        }
        self.fit(ir);
    }

    fn fit(&mut self, ir: &PcbIr) {
        let b = &ir.board.bounds;
        let margin = 5.0_f32;
        self.scene_rect = Some(Rect::from_min_max(
            egui::pos2(b.min.x as f32 - margin, -(b.max.y as f32) - margin),
            egui::pos2(b.max.x as f32 + margin, -(b.min.y as f32) + margin),
        ));
    }
}

impl PcbCanvasView for Pcb2dCanvas {
    fn ui(&mut self, ui: &mut egui::Ui, ir: &PcbIr, selection: &SelectionKind, fit_requested: bool) {
        self.init_rect_if_needed(ir);
        if fit_requested {
            self.fit(ir);
        }

        let mut rect = self
            .scene_rect
            .unwrap_or_else(|| Rect::from_min_max(egui::pos2(-50.0, -50.0), egui::pos2(50.0, 50.0)));

        egui::Scene::new()
            .zoom_range(0.001..=f32::INFINITY)
            .show(ui, &mut rect, |ui| {
                let painter = ui.painter();
                render_board(painter, ir, selection);
            });
        self.scene_rect = Some(rect);
    }
}

#[derive(Debug, Default)]
pub struct Pcb3dCanvas;

impl PcbCanvasView for Pcb3dCanvas {
    fn ui(&mut self, ui: &mut egui::Ui, _ir: &PcbIr, _selection: &SelectionKind, _fit_requested: bool) {
        ui.centered_and_justified(|ui| {
            ui.label("3D canvas placeholder (PaintCallback-ready boundary)");
        });
    }
}

fn to_pos2(x_mm: f64, y_mm: f64) -> Pos2 {
    Pos2::new(x_mm as f32, -(y_mm as f32))
}

fn render_board(p: &Painter, ir: &PcbIr, selection: &SelectionKind) {
    if ir.board.outline.len() >= 3 {
        let points: Vec<Pos2> = ir.board.outline.iter().map(|pt| to_pos2(pt.x, pt.y)).collect();
        p.add(egui::Shape::convex_polygon(
            points,
            Color32::from_rgb(18, 44, 31),
            Stroke::new(0.1, Color32::from_rgb(90, 130, 110)),
        ));
    }

    let selected_comp = match selection {
        SelectionKind::Component(d) => Some(d.as_str()),
        _ => None,
    };
    let selected_net = match selection {
        SelectionKind::Net(n) => Some(n.as_str()),
        _ => None,
    };

    for (_id, comp) in ir.components.iter() {
        let bb = comp.world_bounds;
        let min = to_pos2(bb.min.x, bb.min.y);
        let max = to_pos2(bb.max.x, bb.max.y);
        let rect = Rect::from_min_max(
            Pos2::new(min.x.min(max.x), min.y.min(max.y)),
            Pos2::new(min.x.max(max.x), min.y.max(max.y)),
        );

        let stroke = if selected_comp.is_some_and(|d| d == comp.designator) {
            Stroke::new(0.2, Color32::YELLOW)
        } else {
            Stroke::new(0.08, Color32::from_rgb(180, 80, 80))
        };

        p.rect(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(90, 40, 40, 20),
            stroke,
            StrokeKind::Outside,
        );
        p.text(
            to_pos2(comp.position.x, comp.position.y),
            egui::Align2::CENTER_CENTER,
            &comp.designator,
            egui::FontId::monospace(0.5),
            Color32::from_rgb(220, 220, 220),
        );

        for pad in &comp.pads {
            let center = to_pos2(pad.world_position.x, pad.world_position.y);
            let mut color = if pad.is_through_hole {
                Color32::from_rgb(180, 200, 80)
            } else {
                Color32::from_rgb(220, 130, 40)
            };
            if let Some(net_name) = selected_net {
                let is_match = pad
                    .net
                    .map(|nid| ir.nets[nid].name.as_str() == net_name)
                    .unwrap_or(false);
                if !is_match {
                    color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 60);
                }
            }
            let half_x = (pad.shape.size_x / 2.0) as f32;
            let half_y = (pad.shape.size_y / 2.0) as f32;
            match pad.shape.kind {
                autopcb_ir::PadShapeKind::Round => {
                    p.circle(center, half_x.max(half_y), color, Stroke::NONE);
                }
                _ => {
                    let rect = Rect::from_center_size(center, Vec2::new(half_x * 2.0, half_y * 2.0));
                    p.rect_filled(rect, 0.0, color);
                }
            }
        }
    }
}
