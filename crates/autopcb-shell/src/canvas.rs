use altium_format::api::{SchDocComponent, SchDocSheet, SheetObject};
use altium_format_types::coord::CoordPoint;
use efame::egui::{
    self, Align2, Color32, Painter, PointerButton, Pos2, Rect, Stroke, StrokeKind, Vec2,
};

use autopcb_ir::PcbIr;
use autopcb_ir::component::IrComponent;

use crate::pipeline::ToolId;
use crate::workbench::SelectionKind;

pub trait PcbCanvasView {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        ir: &PcbIr,
        selection: &SelectionKind,
        tool: ToolId,
        move_preview: Option<&MovePreview>,
        fit_requested: bool,
    ) -> Vec<BoardCanvasAction>;
}

pub trait SchDocCanvasView {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        sheet: &SchDocSheet,
        selection: &SelectionKind,
        tool: ToolId,
        move_preview: Option<&SchMovePreview>,
        fit_requested: bool,
    ) -> Vec<SchDocCanvasAction>;
}

#[derive(Debug, Default)]
pub struct Pcb2dCanvas {
    scene_rect: Option<Rect>,
}

#[derive(Debug, Default)]
pub struct SchDoc2dCanvas {
    scene_rect: Option<Rect>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MovePreview {
    pub designator: String,
    pub delta_x_mm: f32,
    pub delta_y_mm: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchMovePreview {
    pub designator: String,
    pub delta_x_mils: f32,
    pub delta_y_mils: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoardCanvasAction {
    ClearSelection,
    SelectComponent(String),
    BeginMoveSelection,
    PreviewMoveSelection { delta_x_mm: f32, delta_y_mm: f32 },
    CommitMoveSelection { delta_x_mm: f32, delta_y_mm: f32 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchDocCanvasAction {
    ClearSelection,
    SelectComponent(String),
    BeginMoveSelection,
    PreviewMoveSelection {
        delta_x_mils: f32,
        delta_y_mils: f32,
    },
    CommitMoveSelection {
        delta_x_mils: f32,
        delta_y_mils: f32,
    },
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

impl SchDoc2dCanvas {
    fn init_rect_if_needed(&mut self, sheet: &SchDocSheet) {
        if self.scene_rect.is_some() {
            return;
        }
        self.fit(sheet);
    }

    fn fit(&mut self, sheet: &SchDocSheet) {
        let bounds = schdoc_bounds(sheet).unwrap_or_else(|| {
            Rect::from_min_max(egui::pos2(-500.0, -500.0), egui::pos2(500.0, 500.0))
        });
        let margin = 200.0;
        self.scene_rect = Some(Rect::from_min_max(
            egui::pos2(bounds.min.x - margin, bounds.min.y - margin),
            egui::pos2(bounds.max.x + margin, bounds.max.y + margin),
        ));
    }
}

impl PcbCanvasView for Pcb2dCanvas {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        ir: &PcbIr,
        selection: &SelectionKind,
        tool: ToolId,
        move_preview: Option<&MovePreview>,
        fit_requested: bool,
    ) -> Vec<BoardCanvasAction> {
        self.init_rect_if_needed(ir);
        if fit_requested {
            self.fit(ir);
        }

        let mut rect = self.scene_rect.unwrap_or_else(|| {
            Rect::from_min_max(egui::pos2(-50.0, -50.0), egui::pos2(50.0, 50.0))
        });

        let scene = egui::Scene::new()
            .zoom_range(0.001..=f32::INFINITY)
            .drag_pan_buttons(egui::DragPanButtons::MIDDLE | egui::DragPanButtons::SECONDARY);
        let response = scene.show(ui, &mut rect, |ui| {
            let painter = ui.painter();
            render_board(painter, ir, selection, move_preview);
        });
        self.scene_rect = Some(rect);

        let mut actions = Vec::new();
        let pointer_pos = response.response.interact_pointer_pos();
        let hovered_component = pointer_pos.and_then(|pos| hit_test_component(ir, pos));

        if response.response.clicked_by(PointerButton::Primary) {
            match (tool, hovered_component.as_deref()) {
                (ToolId::Select, Some(designator)) => {
                    actions.push(BoardCanvasAction::SelectComponent(designator.to_owned()));
                }
                (ToolId::Select, None) => actions.push(BoardCanvasAction::ClearSelection),
                (ToolId::Move, Some(designator)) => {
                    if !matches!(selection, SelectionKind::Component(current) if current == designator)
                    {
                        actions.push(BoardCanvasAction::SelectComponent(designator.to_owned()));
                    }
                }
                _ => {}
            }
        }

        if tool == ToolId::Move && response.response.drag_started_by(PointerButton::Primary) {
            if let Some(designator) = hovered_component.as_deref()
                && matches!(selection, SelectionKind::Component(current) if current == designator)
            {
                actions.push(BoardCanvasAction::BeginMoveSelection);
            }
        }

        if tool == ToolId::Move && response.response.dragged_by(PointerButton::Primary) {
            let delta = response.response.drag_delta();
            actions.push(BoardCanvasAction::PreviewMoveSelection {
                delta_x_mm: delta.x,
                delta_y_mm: -delta.y,
            });
        }

        if tool == ToolId::Move && response.response.drag_stopped_by(PointerButton::Primary) {
            let delta = response.response.drag_delta();
            actions.push(BoardCanvasAction::CommitMoveSelection {
                delta_x_mm: delta.x,
                delta_y_mm: -delta.y,
            });
        }

        actions
    }
}

impl SchDocCanvasView for SchDoc2dCanvas {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        sheet: &SchDocSheet,
        selection: &SelectionKind,
        tool: ToolId,
        move_preview: Option<&SchMovePreview>,
        fit_requested: bool,
    ) -> Vec<SchDocCanvasAction> {
        self.init_rect_if_needed(sheet);
        if fit_requested {
            self.fit(sheet);
        }

        let mut rect = self.scene_rect.unwrap_or_else(|| {
            Rect::from_min_max(egui::pos2(-500.0, -500.0), egui::pos2(500.0, 500.0))
        });

        let scene = egui::Scene::new()
            .zoom_range(0.05..=f32::INFINITY)
            .drag_pan_buttons(egui::DragPanButtons::MIDDLE | egui::DragPanButtons::SECONDARY);
        let response = scene.show(ui, &mut rect, |ui| {
            let painter = ui.painter();
            render_schdoc(painter, sheet, selection, move_preview);
        });
        self.scene_rect = Some(rect);

        let mut actions = Vec::new();
        let pointer_pos = response.response.interact_pointer_pos();
        let hovered_component = pointer_pos.and_then(|pos| hit_test_schdoc_component(sheet, pos));

        if response.response.clicked_by(PointerButton::Primary) {
            match (tool, hovered_component.as_deref()) {
                (ToolId::Select, Some(designator)) => {
                    actions.push(SchDocCanvasAction::SelectComponent(designator.to_owned()));
                }
                (ToolId::Select, None) => actions.push(SchDocCanvasAction::ClearSelection),
                (ToolId::Move, Some(designator)) => {
                    if !matches!(selection, SelectionKind::Component(current) if current == designator)
                    {
                        actions.push(SchDocCanvasAction::SelectComponent(designator.to_owned()));
                    }
                }
                _ => {}
            }
        }

        if tool == ToolId::Move && response.response.drag_started_by(PointerButton::Primary) {
            if let Some(designator) = hovered_component.as_deref()
                && matches!(selection, SelectionKind::Component(current) if current == designator)
            {
                actions.push(SchDocCanvasAction::BeginMoveSelection);
            }
        }

        if tool == ToolId::Move && response.response.dragged_by(PointerButton::Primary) {
            let delta = response.response.drag_delta();
            actions.push(SchDocCanvasAction::PreviewMoveSelection {
                delta_x_mils: delta.x,
                delta_y_mils: -delta.y,
            });
        }

        if tool == ToolId::Move && response.response.drag_stopped_by(PointerButton::Primary) {
            let delta = response.response.drag_delta();
            actions.push(SchDocCanvasAction::CommitMoveSelection {
                delta_x_mils: delta.x,
                delta_y_mils: -delta.y,
            });
        }

        actions
    }
}

#[derive(Debug, Default)]
pub struct Pcb3dCanvas;

impl PcbCanvasView for Pcb3dCanvas {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _ir: &PcbIr,
        _selection: &SelectionKind,
        _tool: ToolId,
        _move_preview: Option<&MovePreview>,
        _fit_requested: bool,
    ) -> Vec<BoardCanvasAction> {
        ui.centered_and_justified(|ui| {
            ui.label("3D canvas placeholder (PaintCallback-ready boundary)");
        });
        Vec::new()
    }
}

fn to_pos2(x_mm: f64, y_mm: f64) -> Pos2 {
    Pos2::new(x_mm as f32, -(y_mm as f32))
}

fn to_sch_pos2(point: CoordPoint) -> Pos2 {
    Pos2::new(point.x.to_mils() as f32, -(point.y.to_mils() as f32))
}

fn render_board(
    p: &Painter,
    ir: &PcbIr,
    selection: &SelectionKind,
    move_preview: Option<&MovePreview>,
) {
    if ir.board.outline.len() >= 3 {
        let points: Vec<Pos2> = ir
            .board
            .outline
            .iter()
            .map(|pt| to_pos2(pt.x, pt.y))
            .collect();
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
        let (bb, position, pads) = if let Some(preview) = move_preview {
            if preview.designator == comp.designator {
                (
                    translate_bb(comp.world_bounds, preview.delta_x_mm, preview.delta_y_mm),
                    (
                        comp.position.x + preview.delta_x_mm as f64,
                        comp.position.y + preview.delta_y_mm as f64,
                    ),
                    comp.pads
                        .iter()
                        .map(|pad| {
                            (
                                pad,
                                pad.world_position.x + preview.delta_x_mm as f64,
                                pad.world_position.y + preview.delta_y_mm as f64,
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                (
                    comp.world_bounds,
                    (comp.position.x, comp.position.y),
                    comp.pads
                        .iter()
                        .map(|pad| (pad, pad.world_position.x, pad.world_position.y))
                        .collect::<Vec<_>>(),
                )
            }
        } else {
            (
                comp.world_bounds,
                (comp.position.x, comp.position.y),
                comp.pads
                    .iter()
                    .map(|pad| (pad, pad.world_position.x, pad.world_position.y))
                    .collect::<Vec<_>>(),
            )
        };
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
            to_pos2(position.0, position.1),
            Align2::CENTER_CENTER,
            &comp.designator,
            egui::FontId::monospace(0.5),
            Color32::from_rgb(220, 220, 220),
        );

        for (pad, pad_x, pad_y) in pads {
            let center = to_pos2(pad_x, pad_y);
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
                    let rect =
                        Rect::from_center_size(center, Vec2::new(half_x * 2.0, half_y * 2.0));
                    p.rect_filled(rect, 0.0, color);
                }
            }
        }
    }
}

fn render_schdoc(
    p: &Painter,
    sheet: &SchDocSheet,
    selection: &SelectionKind,
    move_preview: Option<&SchMovePreview>,
) {
    for wire in sheet.wires() {
        for segment in wire.vertices.windows(2) {
            let [start, end] = segment else { continue };
            p.line_segment(
                [to_sch_pos2(*start), to_sch_pos2(*end)],
                Stroke::new(10.0, Color32::from_rgb(150, 60, 60)),
            );
        }
    }

    for net_label in sheet.net_labels() {
        p.text(
            to_sch_pos2(net_label.location),
            Align2::LEFT_BOTTOM,
            &net_label.text,
            egui::FontId::monospace(80.0),
            Color32::from_rgb(180, 80, 80),
        );
    }

    for object in &sheet.objects {
        match object {
            SheetObject::PowerObject(power) => {
                let pos = to_sch_pos2(power.location);
                p.text(
                    pos,
                    Align2::CENTER_BOTTOM,
                    &power.text,
                    egui::FontId::monospace(80.0),
                    Color32::from_rgb(200, 180, 80),
                );
                p.line_segment(
                    [pos + egui::vec2(-70.0, 20.0), pos + egui::vec2(70.0, 20.0)],
                    Stroke::new(10.0, Color32::from_rgb(200, 180, 80)),
                );
            }
            SheetObject::Note(note) => {
                p.text(
                    to_sch_pos2(note.location),
                    Align2::LEFT_TOP,
                    &note.text,
                    egui::FontId::monospace(70.0),
                    Color32::from_rgb(220, 200, 80),
                );
            }
            _ => {}
        }
    }

    let selected_designator = match selection {
        SelectionKind::Component(designator) => Some(designator.as_str()),
        _ => None,
    };

    for component in sheet.components() {
        let preview_offset = move_preview
            .filter(|preview| preview.designator == component.designator)
            .map(|preview| egui::vec2(preview.delta_x_mils, -preview.delta_y_mils))
            .unwrap_or_default();
        let bounds = schdoc_component_rect(component).translate(preview_offset);
        let is_selected =
            selected_designator.is_some_and(|designator| designator == component.designator);
        let fill = if is_selected {
            Color32::from_rgba_unmultiplied(170, 150, 40, 24)
        } else {
            Color32::from_rgba_unmultiplied(70, 80, 90, 16)
        };
        let stroke = if is_selected {
            Stroke::new(12.0, Color32::YELLOW)
        } else {
            Stroke::new(8.0, Color32::from_rgb(170, 170, 180))
        };
        p.rect(bounds, 8.0, fill, stroke, StrokeKind::Outside);
        p.text(
            bounds.center_top() + egui::vec2(0.0, 30.0),
            Align2::CENTER_TOP,
            &component.designator,
            egui::FontId::monospace(80.0),
            Color32::WHITE,
        );
        p.text(
            bounds.center_bottom() - egui::vec2(0.0, 30.0),
            Align2::CENTER_BOTTOM,
            &component.lib_reference,
            egui::FontId::monospace(65.0),
            Color32::from_rgb(170, 200, 220),
        );
    }
}

fn hit_test_component<'a>(ir: &'a PcbIr, pos: Pos2) -> Option<&'a str> {
    let world_x = pos.x as f64;
    let world_y = -(pos.y as f64);
    ir.components.iter().find_map(|(_, comp)| {
        bb_contains(comp.world_bounds, world_x, world_y).then_some(comp.designator.as_str())
    })
}

fn hit_test_schdoc_component(sheet: &SchDocSheet, pos: Pos2) -> Option<String> {
    sheet.components().into_iter().find_map(|component| {
        schdoc_component_rect(component)
            .contains(pos)
            .then_some(component.designator.clone())
    })
}

fn bb_contains(bb: autopcb_ir::types::BoundingBoxMm, x: f64, y: f64) -> bool {
    x >= bb.min.x && x <= bb.max.x && y >= bb.min.y && y <= bb.max.y
}

fn schdoc_component_rect(component: &SchDocComponent) -> Rect {
    let center = to_sch_pos2(component.location);
    let label_width = (component
        .lib_reference
        .len()
        .max(component.designator.len()) as f32)
        * 28.0;
    let half_width = 110.0 + label_width * 0.5;
    let half_height = 90.0;
    Rect::from_center_size(center, Vec2::new(half_width * 2.0, half_height * 2.0))
}

fn schdoc_bounds(sheet: &SchDocSheet) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    for component in sheet.components() {
        let rect = schdoc_component_rect(component);
        bounds = Some(match bounds {
            Some(current) => current.union(rect),
            None => rect,
        });
    }
    for wire in sheet.wires() {
        for vertex in &wire.vertices {
            let pos = to_sch_pos2(*vertex);
            let rect = Rect::from_center_size(pos, Vec2::splat(1.0));
            bounds = Some(match bounds {
                Some(current) => current.union(rect),
                None => rect,
            });
        }
    }
    bounds
}

fn translate_bb(
    bb: autopcb_ir::types::BoundingBoxMm,
    delta_x_mm: f32,
    delta_y_mm: f32,
) -> autopcb_ir::types::BoundingBoxMm {
    autopcb_ir::types::BoundingBoxMm {
        min: autopcb_ir::types::PointMm {
            x: bb.min.x + delta_x_mm as f64,
            y: bb.min.y + delta_y_mm as f64,
        },
        max: autopcb_ir::types::PointMm {
            x: bb.max.x + delta_x_mm as f64,
            y: bb.max.y + delta_y_mm as f64,
        },
    }
}

pub fn translate_component(comp: &mut IrComponent, delta_x_mm: f32, delta_y_mm: f32) {
    let dx = delta_x_mm as f64;
    let dy = delta_y_mm as f64;
    comp.position.x += dx;
    comp.position.y += dy;
    comp.world_bounds = translate_bb(comp.world_bounds, delta_x_mm, delta_y_mm);
    for pad in &mut comp.pads {
        pad.world_position.x += dx;
        pad.world_position.y += dy;
    }
}

#[cfg(test)]
mod tests {
    use super::{schdoc_component_rect, to_sch_pos2, translate_component};
    use altium_format::api::SchDocComponent;
    use altium_format_types::RotationBy90;
    use altium_format_types::common::ComponentKind;
    use altium_format_types::coord::{Coord, CoordPoint};
    use autopcb_ir::component::{IrComponent, IrComponentPad, PadShapeInfo, PadShapeKind};
    use autopcb_ir::handles::{ComponentId, PadId};
    use autopcb_ir::types::{BoardSide, BoundingBoxMm, PointMm};

    #[test]
    fn translate_component_moves_bounds_and_pads() {
        let mut comp = IrComponent {
            id: ComponentId::from(0),
            designator: "U1".to_owned(),
            pattern: "QFN".to_owned(),
            value: "IC".to_owned(),
            position: PointMm { x: 10.0, y: 20.0 },
            rotation: 0.0,
            side: BoardSide::Top,
            local_bounds: BoundingBoxMm {
                min: PointMm { x: -1.0, y: -1.0 },
                max: PointMm { x: 1.0, y: 1.0 },
            },
            world_bounds: BoundingBoxMm {
                min: PointMm { x: 9.0, y: 19.0 },
                max: PointMm { x: 11.0, y: 21.0 },
            },
            pads: vec![IrComponentPad {
                id: PadId::from(0),
                name: "1".to_owned(),
                local_position: PointMm { x: 0.0, y: 0.0 },
                world_position: PointMm { x: 10.5, y: 20.5 },
                net: None,
                shape: PadShapeInfo {
                    kind: PadShapeKind::Rectangular,
                    size_x: 1.0,
                    size_y: 1.0,
                    rotation: 0.0,
                },
                is_through_hole: false,
                hole_size_mm: 0.0,
                swap_id_pin: None,
                swap_id_part: None,
            }],
        };

        translate_component(&mut comp, 2.5, -1.0);
        assert_eq!(comp.position.x, 12.5);
        assert_eq!(comp.position.y, 19.0);
        assert_eq!(comp.world_bounds.min.x, 11.5);
        assert_eq!(comp.world_bounds.max.y, 20.0);
        assert_eq!(comp.pads[0].world_position.x, 13.0);
        assert_eq!(comp.pads[0].world_position.y, 19.5);
    }

    #[test]
    fn schdoc_component_rect_contains_component_location() {
        let comp = SchDocComponent {
            designator: "U1".to_owned(),
            unique_id: String::new(),
            lib_reference: "ESP32-C6-MINI-1".to_owned(),
            source_library_name: String::new(),
            design_item_id: String::new(),
            library_path: String::new(),
            location: CoordPoint::new(
                Coord::from_mils(1000).unwrap(),
                Coord::from_mils(800).unwrap(),
            ),
            orientation: RotationBy90::Rotate0,
            is_mirrored: false,
            description: None,
            component_kind: ComponentKind::Standard,
            part_count: 1,
            current_part_id: 1,
            display_mode_count: 1,
            show_hidden_pins: false,
            children: Vec::new(),
        };

        let rect = schdoc_component_rect(&comp);
        assert!(rect.contains(to_sch_pos2(comp.location)));
    }
}
