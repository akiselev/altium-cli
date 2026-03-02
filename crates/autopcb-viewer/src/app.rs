//! The eframe application struct and its `App` implementation.

use std::sync::{Arc, Mutex};

use eframe::egui::{self, Rect};

use autopcb_ir::{BoardSide, ComponentId, PcbIr, PointMm};

use crate::colors;
use crate::interaction;
use crate::renderer::{self, RenderOptions};

pub struct ViewerApp {
    ir: Arc<Mutex<PcbIr>>,
    selected_component: Option<ComponentId>,
    hovered_component: Option<ComponentId>,
    render_opts: RenderOptions,
    /// Persistent view bounds for the Scene (mutated by pan/zoom).
    scene_rect: Rect,
}

impl ViewerApp {
    pub fn new(ir: Arc<Mutex<PcbIr>>) -> Self {
        // Initialize scene_rect from the board bounds
        let initial_rect = {
            let ir = ir.lock().unwrap();
            let b = &ir.board.bounds;
            // Y-flip: PCB min_y maps to screen max_y
            let margin = 5.0; // mm margin around board
            Rect::from_min_max(
                egui::pos2(b.min.x as f32 - margin, -(b.max.y as f32) - margin),
                egui::pos2(b.max.x as f32 + margin, -(b.min.y as f32) + margin),
            )
        };

        Self {
            ir,
            selected_component: None,
            hovered_component: None,
            render_opts: RenderOptions::default(),
            scene_rect: initial_rect,
        }
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let ir = self.ir.lock().unwrap();

        // Left sidebar: controls and info
        egui::SidePanel::left("sidebar")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("AutoPCB Viewer");
                ui.separator();

                // Board info
                ui.label(format!(
                    "Board: {:.1} x {:.1} mm",
                    ir.board.bounds.width(),
                    ir.board.bounds.height()
                ));
                ui.label(format!("Components: {}", ir.components.len()));
                ui.label(format!("Nets: {}", ir.nets.len()));
                ui.label(format!(
                    "Copper layers: {}",
                    ir.layer_stack.copper_layer_count
                ));
                ui.separator();

                // Display toggles
                ui.heading("Display");
                ui.checkbox(&mut self.render_opts.show_board_outline, "Board outline");
                ui.checkbox(&mut self.render_opts.show_components, "Components");
                ui.checkbox(&mut self.render_opts.show_pads, "Pads");
                ui.checkbox(&mut self.render_opts.show_tracks, "Tracks");
                ui.checkbox(&mut self.render_opts.show_vias, "Vias");
                ui.checkbox(&mut self.render_opts.show_ratsnest, "Ratsnest");
                ui.checkbox(&mut self.render_opts.show_designators, "Designators");
                ui.separator();

                // Component list (scrollable)
                ui.heading("Components");
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for (id, comp) in ir.components.iter() {
                            let is_selected = self.selected_component == Some(id);
                            let side_str = match comp.side {
                                BoardSide::Top => "T",
                                BoardSide::Bottom => "B",
                            };
                            let label =
                                format!("{} [{}] {}", comp.designator, side_str, comp.pattern);
                            if ui.selectable_label(is_selected, &label).clicked() {
                                self.selected_component =
                                    if is_selected { None } else { Some(id) };
                            }
                        }
                    });
            });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(id) = self.hovered_component {
                    let comp = &ir.components[id];
                    ui.label(format!(
                        "Hover: {} ({}) at ({:.2}, {:.2}) mm",
                        comp.designator, comp.pattern, comp.position.x, comp.position.y
                    ));
                } else {
                    ui.label("Hover over a component for details");
                }
            });
        });

        // Central panel: the board canvas with Scene (pan + zoom)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().visuals.panel_fill = colors::BACKGROUND;

            let scene_response = egui::Scene::new()
                .zoom_range(0.001..=f32::INFINITY)
                .show(ui, &mut self.scene_rect, |ui| {
                    let painter = ui.painter();
                    renderer::render_board(
                        painter,
                        &ir,
                        &self.render_opts,
                        self.selected_component,
                        self.hovered_component,
                    );
                });

            // Hit-testing via hover position (in scene/world coordinates)
            if let Some(hover_pos) = scene_response.response.hover_pos() {
                // The hover_pos is in screen coordinates; we need scene coordinates.
                // For now, approximate using the scene rect transform.
                // egui Scene transforms screen->scene internally, but we can use
                // the response rect and scene_rect to compute the inverse.
                let resp_rect = scene_response.response.rect;
                let sx = self.scene_rect.width() / resp_rect.width();
                let sy = self.scene_rect.height() / resp_rect.height();
                let scene_x = self.scene_rect.min.x + (hover_pos.x - resp_rect.min.x) * sx;
                let scene_y = self.scene_rect.min.y + (hover_pos.y - resp_rect.min.y) * sy;
                // Undo Y-flip: scene_y = -world_y
                let world = PointMm::new(scene_x as f64, -scene_y as f64);
                self.hovered_component = interaction::find_component_at(&ir, world);

                // Click to select
                if scene_response.response.clicked() {
                    self.selected_component = self.hovered_component;
                }

                // Tooltip on hover
                if let Some(hid) = self.hovered_component {
                    scene_response.response.on_hover_text(
                        interaction::component_tooltip(&ir, hid),
                    );
                }
            } else {
                self.hovered_component = None;
            }
        });
    }
}
