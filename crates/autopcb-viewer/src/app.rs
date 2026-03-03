//! The eframe application struct and its `App` implementation.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use eframe::egui::{self, ColorImage, Event, Rect, UserData, ViewportCommand};

use autopcb_ir::{BoardSide, ComponentId, NetId, PcbIr, PointMm};
use autopcb_placement::PlacementIterationSnapshot;

use crate::colors;
use crate::interaction;
use crate::renderer::{self, RenderOptions};
use crate::view3d::{Camera, PcbScene3D, PcbScene3DCallback, SceneResources};

/// Whether to show the 2-D top-down view or the 2.5-D wgpu view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    TopDown2D,
    Perspective3D,
}

pub struct ViewerApp {
    ir: Arc<Mutex<PcbIr>>,
    selected_component: Option<ComponentId>,
    hovered_component: Option<ComponentId>,
    selected_net: Option<NetId>,
    render_opts: RenderOptions,
    /// Persistent view bounds for the Scene (mutated by pan/zoom).
    scene_rect: Rect,
    /// Path to save screenshot to, if --screenshot was passed.
    screenshot_path: Option<PathBuf>,
    /// True after we have sent the Screenshot viewport command but before we receive the event.
    screenshot_requested: bool,
    /// Current display mode.
    view_mode: ViewMode,
    /// Camera for the 3-D view.
    camera: Camera,
    /// Reserved for future use (cursor tracking for orbit).
    #[allow(dead_code)]
    drag_last: Option<egui::Pos2>,
    playback: Option<Vec<PlacementIterationSnapshot>>,
    playback_index: usize,
    playback_playing: bool,
    playback_last_tick: Instant,
}

impl ViewerApp {
    pub fn new(
        ir: Arc<Mutex<PcbIr>>,
        screenshot_path: Option<PathBuf>,
        playback: Option<Vec<PlacementIterationSnapshot>>,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
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

        // Build camera centred on the board.
        let mut camera = Camera::default();
        {
            let ir = ir.lock().unwrap();
            let b = &ir.board.bounds;
            camera.target = [
                ((b.min.x + b.max.x) / 2.0) as f32,
                ((b.min.y + b.max.y) / 2.0) as f32,
                0.8,
            ];
            camera.zoom = (b.width().max(b.height()) as f32 / 2.0) + 10.0;

            // Upload GPU scene resources if the wgpu render state is available.
            if let Some(wgpu_state) = &cc.wgpu_render_state {
                let scene = PcbScene3D::from_ir(
                    &ir,
                    &wgpu_state.device,
                    &wgpu_state.queue,
                    wgpu_state.target_format,
                );
                wgpu_state
                    .renderer
                    .write()
                    .callback_resources
                    .insert(SceneResources { scene });
            }
        }

        Self {
            ir,
            selected_component: None,
            hovered_component: None,
            selected_net: None,
            render_opts: RenderOptions::default(),
            scene_rect: initial_rect,
            screenshot_path,
            screenshot_requested: false,
            view_mode: ViewMode::TopDown2D,
            camera,
            drag_last: None,
            playback,
            playback_index: 0,
            playback_playing: false,
            playback_last_tick: Instant::now(),
        }
    }

    fn apply_snapshot_to_ir(ir: &mut PcbIr, snap: &PlacementIterationSnapshot) {
        for state in &snap.components {
            for (_id, comp) in ir.components.iter_mut() {
                if comp.designator != state.designator {
                    continue;
                }

                comp.position = PointMm::new(state.x_mm, state.y_mm);
                comp.rotation = state.rotation_deg;

                let theta = state.rotation_deg.to_radians();
                let (sin_t, cos_t) = theta.sin_cos();
                for pad in &mut comp.pads {
                    let lx = pad.local_position.x;
                    let ly = pad.local_position.y;
                    pad.world_position = PointMm::new(
                        state.x_mm + lx * cos_t - ly * sin_t,
                        state.y_mm + lx * sin_t + ly * cos_t,
                    );
                }

                let lb = comp.local_bounds;
                let corners = [
                    PointMm::new(lb.min.x, lb.min.y),
                    PointMm::new(lb.min.x, lb.max.y),
                    PointMm::new(lb.max.x, lb.min.y),
                    PointMm::new(lb.max.x, lb.max.y),
                ];
                let mut world_pts = Vec::with_capacity(4);
                for c in corners {
                    world_pts.push(PointMm::new(
                        state.x_mm + c.x * cos_t - c.y * sin_t,
                        state.y_mm + c.x * sin_t + c.y * cos_t,
                    ));
                }
                if let Some(bb) = autopcb_ir::BoundingBoxMm::from_points(&world_pts) {
                    comp.world_bounds = bb;
                }
            }
        }
    }

    fn advance_playback(&mut self) {
        let Some(playback) = self.playback.as_ref() else {
            return;
        };
        if playback.is_empty() {
            return;
        }
        let len = playback.len();
        let idx = self.playback_index.min(len - 1);

        {
            let mut ir = self.ir.lock().unwrap();
            Self::apply_snapshot_to_ir(&mut ir, &playback[idx]);
        }

        if self.playback_playing {
            let now = Instant::now();
            if now.duration_since(self.playback_last_tick).as_millis() >= 250 {
                self.playback_last_tick = now;
                self.playback_index = (self.playback_index + 1).min(len - 1);
            }
        }
    }
}

fn save_screenshot_png(image: &ColorImage, path: &Path) {
    let [width, height] = image.size;
    let rgba: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|c| c.to_array())
        .collect();
    if let Err(e) = image::save_buffer(
        path,
        &rgba,
        width as u32,
        height as u32,
        image::ColorType::Rgba8,
    ) {
        eprintln!("Failed to save screenshot to {}: {e}", path.display());
    } else {
        eprintln!("Screenshot saved to {}", path.display());
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.advance_playback();

        // Screenshot mode: request on first frame, save on receipt, then close.
        if self.screenshot_path.is_some() && !self.screenshot_requested {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
            self.screenshot_requested = true;
        }

        // Check for incoming screenshot events.
        let screenshot_event = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| {
                if let Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot_event {
            if let Some(ref path) = self.screenshot_path {
                save_screenshot_png(&image, path);
                ctx.send_viewport_cmd(ViewportCommand::Close);
                return;
            } else {
                // Interactive S-key screenshot
                save_screenshot_png(&image, Path::new("screenshot.png"));
                self.screenshot_requested = false;
            }
        }

        // Interactive S key: trigger screenshot saved to screenshot.png.
        if !self.screenshot_requested
            && self.screenshot_path.is_none()
            && ctx.input(|i| i.key_pressed(egui::Key::S))
        {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
            self.screenshot_requested = true;
        }

        let ir = self.ir.lock().unwrap();

        // Keyboard shortcuts (only when no text widget has focus)
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if i.key_pressed(egui::Key::F) {
                    // Fit to board: reset scene_rect from board bounds + margin
                    let b = &ir.board.bounds;
                    let margin = 5.0_f32;
                    self.scene_rect = Rect::from_min_max(
                        egui::pos2(b.min.x as f32 - margin, -(b.max.y as f32) - margin),
                        egui::pos2(b.max.x as f32 + margin, -(b.min.y as f32) + margin),
                    );
                }
                if i.key_pressed(egui::Key::N) {
                    self.render_opts.show_ratsnest = !self.render_opts.show_ratsnest;
                }
                if i.key_pressed(egui::Key::L) {
                    // Toggle all copper layers together
                    let new_state = !self.render_opts.show_tracks;
                    self.render_opts.show_tracks = new_state;
                    self.render_opts.show_fills = new_state;
                    self.render_opts.show_polygons = new_state;
                }
                if i.key_pressed(egui::Key::Escape) {
                    self.selected_component = None;
                    self.selected_net = None;
                }
            });
        }

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

                if let Some(playback) = self.playback.as_ref() {
                    if !playback.is_empty() {
                        ui.heading("Playback");
                        ui.horizontal(|ui| {
                            if ui
                                .button(if self.playback_playing { "Pause" } else { "Play" })
                                .clicked()
                            {
                                self.playback_playing = !self.playback_playing;
                                self.playback_last_tick = Instant::now();
                            }
                            if ui.button("Reset").clicked() {
                                self.playback_playing = false;
                                self.playback_index = 0;
                            }
                        });

                        let max_idx = playback.len().saturating_sub(1);
                        ui.add(egui::Slider::new(&mut self.playback_index, 0..=max_idx).text("frame"));
                        let idx = self.playback_index.min(max_idx);
                        ui.label(format!("Phase: {}", playback[idx].phase));
                        if let Some(note) = &playback[idx].note {
                            ui.label(note);
                        }
                        ui.separator();
                    }
                }

                // View mode toggle
                ui.heading("View");
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.view_mode == ViewMode::TopDown2D, "2D")
                        .clicked()
                    {
                        self.view_mode = ViewMode::TopDown2D;
                    }
                    if ui
                        .selectable_label(self.view_mode == ViewMode::Perspective3D, "3D")
                        .clicked()
                    {
                        self.view_mode = ViewMode::Perspective3D;
                    }
                });
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
                ui.checkbox(&mut self.render_opts.show_keepouts, "Keepouts");
                ui.checkbox(&mut self.render_opts.show_fills, "Fills");
                ui.checkbox(&mut self.render_opts.show_polygons, "Polygons");
                ui.separator();

                // Component list (scrollable)
                ui.collapsing("Components", |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("comp_scroll")
                        .max_height(200.0)
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

                ui.separator();

                // Nets section
                ui.collapsing("Nets", |ui| {
                    // When a component is selected, show its connected nets at the top
                    let component_nets = self
                        .selected_component
                        .map(|cid| interaction::nets_for_component(&ir, cid))
                        .unwrap_or_default();

                    egui::ScrollArea::vertical()
                        .id_salt("net_scroll")
                        .max_height(250.0)
                        .show(ui, |ui| {
                            // Component-connected nets first (if a component is selected)
                            if !component_nets.is_empty() {
                                ui.label(egui::RichText::new("Connected nets:").small().italics());
                                for net_id in &component_nets {
                                    let net = &ir.nets[*net_id];
                                    let is_selected = self.selected_net == Some(*net_id);
                                    let label = format!("{} ({} pins)", net.name, net.pins.len());
                                    if ui.selectable_label(is_selected, &label).clicked() {
                                        self.selected_net =
                                            if is_selected { None } else { Some(*net_id) };
                                    }
                                }
                                ui.separator();
                                ui.label(egui::RichText::new("All nets:").small().italics());
                            }

                            // Full net list (skip already shown connected nets)
                            for (net_id, net) in ir.nets.iter() {
                                if component_nets.contains(&net_id) {
                                    continue;
                                }
                                let is_selected = self.selected_net == Some(net_id);
                                let label = format!("{} ({} pins)", net.name, net.pins.len());
                                if ui.selectable_label(is_selected, &label).clicked() {
                                    self.selected_net =
                                        if is_selected { None } else { Some(net_id) };
                                }
                            }
                        });
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
                } else if let Some(net_id) = self.selected_net {
                    let net = &ir.nets[net_id];
                    ui.label(format!(
                        "Net: {} | {} pins | {} components",
                        net.name, net.pins.len(), net.component_count
                    ));
                } else {
                    ui.label("Hover over a component for details");
                }
            });
        });

        // Central panel: 2D top-down or 2.5D wgpu view
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().visuals.panel_fill = colors::BACKGROUND;

            match self.view_mode {
                ViewMode::TopDown2D => {
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
                                self.selected_net,
                            );
                        });

                    // Hit-testing via hover position (in scene/world coordinates)
                    if let Some(hover_pos) = scene_response.response.hover_pos() {
                        let resp_rect = scene_response.response.rect;
                        let sx = self.scene_rect.width() / resp_rect.width();
                        let sy = self.scene_rect.height() / resp_rect.height();
                        let scene_x =
                            self.scene_rect.min.x + (hover_pos.x - resp_rect.min.x) * sx;
                        let scene_y =
                            self.scene_rect.min.y + (hover_pos.y - resp_rect.min.y) * sy;
                        // Undo Y-flip: scene_y = -world_y
                        let world = PointMm::new(scene_x as f64, -scene_y as f64);
                        self.hovered_component = interaction::find_component_at(&ir, world);

                        // Click to select
                        if scene_response.response.clicked() {
                            self.selected_component = self.hovered_component;
                        }

                        // Tooltip on hover
                        if let Some(hid) = self.hovered_component {
                            scene_response
                                .response
                                .on_hover_text(interaction::component_tooltip(&ir, hid));
                        }
                    } else {
                        self.hovered_component = None;
                    }
                }

                ViewMode::Perspective3D => {
                    let (rect, response) =
                        ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

                    // Orbit via primary mouse button drag
                    if response.dragged_by(egui::PointerButton::Primary) {
                        let delta = response.drag_delta();
                        self.camera.orbit(delta.x, delta.y);
                    }

                    // Zoom via scroll
                    let scroll_delta = ctx.input(|i| i.raw_scroll_delta.y);
                    if scroll_delta != 0.0 && response.contains_pointer() {
                        self.camera.scroll(scroll_delta);
                    }

                    // Issue the wgpu paint callback
                    let camera_snapshot = Camera {
                        yaw:    self.camera.yaw,
                        pitch:  self.camera.pitch,
                        zoom:   self.camera.zoom,
                        target: self.camera.target,
                    };
                    ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                        rect,
                        PcbScene3DCallback {
                            camera:        camera_snapshot,
                            viewport_rect: rect,
                        },
                    ));

                    // Help text overlay
                    ui.painter().text(
                        rect.left_bottom() + egui::vec2(8.0, -8.0),
                        egui::Align2::LEFT_BOTTOM,
                        "Drag to orbit  |  Scroll to zoom  |  2D button to return",
                        egui::FontId::proportional(12.0),
                        egui::Color32::from_rgba_premultiplied(200, 200, 200, 180),
                    );
                }
            }
        });
    }
}
