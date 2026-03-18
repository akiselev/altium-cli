//! The eframe application struct and its `App` implementation.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use crate::{apply_spec_positions, load_spec};

use eframe::egui::{self, ColorImage, Event, Rect, UserData, ViewportCommand};

use autopcb_ir::{BoardSide, ComponentId, NetId, PcbIr, PointMm};
use autopcb_placement::PlacementIterationSnapshot;

use crate::colors;
use crate::interaction;
use crate::renderer::{self, RenderOptions};
use crate::view3d::{Camera, PcbScene3D, PcbScene3DCallback, SceneResources};

/// Minimum time between two consecutive reloads (debounce window).
const RELOAD_DEBOUNCE: Duration = Duration::from_millis(100);

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
    /// Receiver for file-system change events from `notify`.  `None` if --watch was not passed.
    watch_rx: Option<mpsc::Receiver<notify::Result<notify::Event>>>,
    /// Path to the PcbDoc being displayed; used to reload on file change.
    pcbdoc_path: PathBuf,
    /// Path to the playback JSON file; reloaded when the file changes.
    playback_path: Option<PathBuf>,
    /// Path to the `.pcbdoc-spec` file, if the viewer was launched with one.
    spec_path: Option<PathBuf>,
    /// Explicit `--target` override for spec mode; `None` means use the spec's `target:` field.
    explicit_target: Option<PathBuf>,
    /// Timestamp of the most recent successful reload; shown in the sidebar.
    last_reloaded: Option<std::time::SystemTime>,
    /// Wall-clock instant of the most recent reload; used for debouncing.
    last_reload_instant: Option<Instant>,
}

impl ViewerApp {
    pub fn new(
        ir: Arc<Mutex<PcbIr>>,
        screenshot_path: Option<PathBuf>,
        playback: Option<Vec<PlacementIterationSnapshot>>,
        watch_rx: Option<mpsc::Receiver<notify::Result<notify::Event>>>,
        pcbdoc_path: PathBuf,
        playback_path: Option<PathBuf>,
        spec_path: Option<PathBuf>,
        explicit_target: Option<PathBuf>,
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
            watch_rx,
            pcbdoc_path,
            playback_path,
            spec_path,
            explicit_target,
            last_reloaded: None,
            last_reload_instant: None,
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

    /// Drain pending watcher events and reload files if a relevant change was detected.
    ///
    /// Returns `true` if a reload was performed and the caller should call
    /// `request_repaint()`.
    fn check_watch_events(&mut self, cc_wgpu: Option<&egui_wgpu::RenderState>) -> bool {
        use notify::EventKind;

        let rx = match self.rx_ref() {
            Some(r) => r as *const mpsc::Receiver<notify::Result<notify::Event>>,
            None => return false,
        };
        // SAFETY: we only use `rx` while `self.watch_rx` is `Some`, and we do not
        // call any method that would drop or move `self.watch_rx` during this scope.
        let rx = unsafe { &*rx };

        let mut reload_pcbdoc = false;
        let mut reload_playback = false;

        while let Ok(event_result) = rx.try_recv() {
            let event = match event_result {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Watch error: {e}");
                    continue;
                }
            };

            match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) => {}
                _ => continue,
            }

            for path in &event.paths {
                if path == &self.pcbdoc_path {
                    reload_pcbdoc = true;
                }
                // A change to the spec file also triggers a full PcbDoc reload
                // so that updated `at:` positions are re-applied.
                if let Some(ref sp) = self.spec_path {
                    if path == sp {
                        reload_pcbdoc = true;
                    }
                }
                if let Some(ref pb) = self.playback_path {
                    if path == pb {
                        reload_playback = true;
                    }
                }
            }
        }

        if !reload_pcbdoc && !reload_playback {
            return false;
        }

        // Debounce: ignore if we reloaded less than RELOAD_DEBOUNCE ago.
        if let Some(last) = self.last_reload_instant {
            if last.elapsed() < RELOAD_DEBOUNCE {
                return false;
            }
        }

        let mut did_reload = false;

        if reload_pcbdoc {
            match self.reload_pcbdoc(cc_wgpu) {
                Ok(()) => {
                    did_reload = true;
                }
                Err(e) => {
                    eprintln!("Reload failed: {e}");
                }
            }
        }

        if reload_playback {
            match self.reload_playback() {
                Ok(()) => {
                    did_reload = true;
                }
                Err(e) => {
                    eprintln!("Playback reload failed: {e}");
                }
            }
        }

        if did_reload {
            self.last_reloaded = Some(std::time::SystemTime::now());
            self.last_reload_instant = Some(Instant::now());
        }

        did_reload
    }

    fn rx_ref(&self) -> Option<&mpsc::Receiver<notify::Result<notify::Event>>> {
        self.watch_rx.as_ref()
    }

    fn reload_pcbdoc(&mut self, cc_wgpu: Option<&egui_wgpu::RenderState>) -> anyhow::Result<()> {
        use altium_format::PcbDoc;
        use autopcb_ir::PcbIr;

        // If a spec file is present, re-parse it to pick up updated positions
        // and (if using the spec's target:) to re-resolve the PcbDoc path.
        let spec_positions: Vec<(String, f64, f64)> = if let Some(ref sp) = self.spec_path.clone() {
            eprintln!("Re-parsing spec {}...", sp.display());
            match load_spec(sp, self.explicit_target.as_deref()) {
                Ok((_resolved_pcbdoc, positions)) => positions,
                Err(e) => {
                    eprintln!("Spec reload failed: {e}");
                    return Err(e);
                }
            }
        } else {
            Vec::new()
        };

        eprintln!("Reloading {}...", self.pcbdoc_path.display());
        let doc = PcbDoc::open(&self.pcbdoc_path)
            .map_err(|e| anyhow::anyhow!("open: {e}"))?;
        let board = doc.board()
            .map_err(|e| anyhow::anyhow!("board: {e}"))?;
        let mut new_ir = PcbIr::extract(&board)
            .map_err(|e| anyhow::anyhow!("extract: {e}"))?;

        if !spec_positions.is_empty() {
            apply_spec_positions(&mut new_ir, &spec_positions);
        }

        if let Some(wgpu_state) = cc_wgpu {
            let scene = crate::view3d::PcbScene3D::from_ir(
                &new_ir,
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

        *self.ir.lock().unwrap() = new_ir;
        eprintln!("Reload complete.");
        Ok(())
    }

    fn reload_playback(&mut self) -> anyhow::Result<()> {
        let pb_path = match self.playback_path.as_ref() {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        eprintln!("Reloading playback {}...", pb_path.display());
        let source = std::fs::read_to_string(&pb_path)
            .map_err(|e| anyhow::anyhow!("read: {e}"))?;
        let snapshots: Vec<PlacementIterationSnapshot> = serde_json::from_str(&source)
            .map_err(|e| anyhow::anyhow!("parse: {e}"))?;
        self.playback = Some(snapshots);
        self.playback_index = 0;
        Ok(())
    }

    /// Format a `SystemTime` as `HH:MM:SS` in local time (best-effort; falls back to UTC).
    fn format_reload_time(t: std::time::SystemTime) -> String {
        use std::time::{UNIX_EPOCH};
        let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let h = (secs / 3600) % 24;
        let m = (secs / 60) % 60;
        let s = secs % 60;
        format!("{h:02}:{m:02}:{s:02} UTC")
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Check for file-system change events before anything else.
        let wgpu_state = frame.wgpu_render_state();
        if self.check_watch_events(wgpu_state) {
            ctx.request_repaint();
        }

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

                // Reload indicator (only shown after at least one file-watch reload)
                if let Some(t) = self.last_reloaded {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "Reloaded at {}",
                            Self::format_reload_time(t)
                        ))
                        .color(egui::Color32::from_rgb(100, 220, 100))
                        .small(),
                    );
                }

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
