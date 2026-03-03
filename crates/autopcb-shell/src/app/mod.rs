use std::collections::VecDeque;
use std::path::PathBuf;

use efame::egui::{self, Event, KeyboardShortcut, Modifiers};
use egui_tiles::{Behavior, TileId, UiResponse};

use crate::canvas::{Pcb2dCanvas, Pcb3dCanvas, PcbCanvasView};
use crate::commands::{
    build_context, dispatch, selection_label, CommandRegistry, DispatchOutcome,
};
use crate::layout::{BottomTab, EditorPane, ShellLayoutState};
use crate::workbench::WorkbenchModel;

const STORAGE_LAYOUT_KEY: &str = "shell.layout.v1";
const STORAGE_PANELS_KEY: &str = "shell.panels.v1";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PanelVisibilityState {
    show_primary_sidebar: bool,
    show_bottom_panel: bool,
    bottom_tab: BottomTab,
}

impl Default for PanelVisibilityState {
    fn default() -> Self {
        Self {
            show_primary_sidebar: true,
            show_bottom_panel: true,
            bottom_tab: BottomTab::Output,
        }
    }
}

pub struct ShellApp {
    model: WorkbenchModel,
    layout: ShellLayoutState,
    panel_visibility: PanelVisibilityState,
    commands: CommandRegistry,
    queued: VecDeque<(String, Option<String>)>,
    show_command_palette: bool,
    palette_filter: String,
    canvas2d: Pcb2dCanvas,
    canvas3d: Pcb3dCanvas,
}

impl ShellApp {
    pub fn new(
        cc: &efame::CreationContext<'_>,
        board_path: Option<PathBuf>,
        initial_ir: Option<autopcb_ir::PcbIr>,
    ) -> Self {
        let mut layout = ShellLayoutState::default();
        let mut panel_visibility = PanelVisibilityState::default();

        if let Some(storage) = cc.storage {
            if let Some(saved) = efame::get_value(storage, STORAGE_LAYOUT_KEY) {
                layout = saved;
            }
            if let Some(saved) = efame::get_value(storage, STORAGE_PANELS_KEY) {
                panel_visibility = saved;
            }
        }

        Self {
            model: WorkbenchModel::new(board_path, initial_ir),
            layout,
            panel_visibility,
            commands: CommandRegistry::new_m1(),
            queued: VecDeque::new(),
            show_command_palette: false,
            palette_filter: String::new(),
            canvas2d: Pcb2dCanvas::default(),
            canvas3d: Pcb3dCanvas,
        }
    }

    fn queue(&mut self, id: &str, arg: Option<String>) {
        self.queued.push_back((id.to_owned(), arg));
    }

    fn process_queue(&mut self, ctx: &egui::Context, _frame: &mut efame::Frame) {
        while let Some((id, arg)) = self.queued.pop_front() {
            match dispatch(
                &id,
                arg,
                &mut self.model,
                &mut self.panel_visibility.show_primary_sidebar,
                &mut self.panel_visibility.show_bottom_panel,
                &mut self.panel_visibility.bottom_tab,
                &mut self.layout,
                &mut self.show_command_palette,
            ) {
                DispatchOutcome::Noop => {}
                DispatchOutcome::RequestQuit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let mut map = Vec::new();
        map.push((
            KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, egui::Key::P),
            "workbench.command_palette",
        ));
        map.push((
            KeyboardShortcut::new(Modifiers::COMMAND, egui::Key::P),
            "navigate.quick_open",
        ));
        map.push((KeyboardShortcut::new(Modifiers::NONE, egui::Key::F), "pcb.zoom.fit"));
        map.push((KeyboardShortcut::new(Modifiers::NONE, egui::Key::Escape), "selection.clear"));
        map.push((KeyboardShortcut::new(Modifiers::COMMAND, egui::Key::J), "view.toggle_bottom_panel"));
        map.push((
            KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, egui::Key::E),
            "panel.show.explorer",
        ));

        for (sc, id) in map {
            let consumed = ctx.input_mut(|i| i.consume_shortcut(&sc));
            if consumed {
                self.queue(id, None);
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::N)) {
            self.queue("panel.show.output", None);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::L)) {
            self.queue("panel.show.jobs", None);
        }
    }

    fn show_palette_window(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }

        let cmd_ctx = build_context(&self.model, true, true);
        let mut open = self.show_command_palette;
        egui::Window::new("Command Palette")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.text_edit_singleline(&mut self.palette_filter);
                ui.separator();
                let filter = self.palette_filter.to_lowercase();

                let available: Vec<_> = self.commands.exposed().collect();
                for meta in available {
                    if !self.commands.is_enabled(meta, &cmd_ctx) {
                        continue;
                    }
                    if !filter.is_empty()
                        && !meta.title.to_lowercase().contains(&filter)
                        && !meta.id.contains(&filter)
                    {
                        continue;
                    }
                    if ui.button(format!("{} ({})", meta.title, meta.id)).clicked() {
                        self.queue(meta.id, None);
                        self.show_command_palette = false;
                    }
                }
            });
        self.show_command_palette = open;
    }

    fn render_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Palette").clicked() {
                    self.queue("workbench.command_palette", None);
                }
                if ui.button("2D").clicked() {
                    self.queue("pcb.view.2d", None);
                }
                if ui.button("3D").clicked() {
                    self.queue("pcb.view.3d", None);
                }
                if ui.button("Reset Layout").clicked() {
                    self.queue("view.reset_layout", None);
                }
                if ui.button("Quit").clicked() {
                    self.queue("app.quit", None);
                }
            });
        });
    }

    fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.panel_visibility.show_primary_sidebar {
            return;
        }

        egui::SidePanel::left("primary_sidebar")
            .resizable(true)
            .default_width(240.0)
            .show(ctx, |ui| {
                ui.heading("Explorer");
                ui.separator();
                let Some(ir) = self.model.ir.as_ref() else {
                    ui.label("No board loaded");
                    return;
                };
                let components: Vec<String> = ir
                    .components
                    .iter()
                    .map(|(_, comp)| comp.designator.clone())
                    .collect();
                let nets: Vec<(String, usize)> = ir
                    .nets
                    .iter()
                    .map(|(_, net)| (net.name.clone(), net.pins.len()))
                    .collect();

                ui.collapsing("Components", |ui| {
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        for designator in &components {
                            let selected = matches!(
                                &self.model.selection.primary,
                                crate::workbench::SelectionKind::Component(d) if d == designator
                            );
                            if ui.selectable_label(selected, designator).clicked() {
                                self.queue("crossprobe.select_component", Some(designator.clone()));
                            }
                        }
                    });
                });

                ui.collapsing("Nets", |ui| {
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        for (name, pins_len) in &nets {
                            let selected = matches!(
                                &self.model.selection.primary,
                                crate::workbench::SelectionKind::Net(n) if n == name
                            );
                            if ui
                                .selectable_label(selected, format!("{} ({})", name, pins_len))
                                .clicked()
                            {
                                self.queue("crossprobe.select_net", Some(name.clone()));
                            }
                        }
                    });
                });
            });
    }

    fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        if !self.panel_visibility.show_bottom_panel {
            return;
        }

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(180.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.panel_visibility.bottom_tab == BottomTab::Problems, "Problems")
                        .clicked()
                    {
                        self.queue("panel.show.problems", None);
                    }
                    if ui
                        .selectable_label(self.panel_visibility.bottom_tab == BottomTab::Output, "Output")
                        .clicked()
                    {
                        self.queue("panel.show.output", None);
                    }
                    if ui
                        .selectable_label(self.panel_visibility.bottom_tab == BottomTab::Jobs, "Jobs")
                        .clicked()
                    {
                        self.queue("panel.show.jobs", None);
                    }
                });
                ui.separator();

                match self.panel_visibility.bottom_tab {
                    BottomTab::Problems => {
                        if self.model.problems.is_empty() {
                            ui.label("No problems");
                        }
                        for line in &self.model.problems {
                            ui.label(line);
                        }
                    }
                    BottomTab::Output => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for line in &self.model.output_lines {
                                ui.monospace(line);
                            }
                        });
                    }
                    BottomTab::Jobs => {
                        if self.model.jobs.is_empty() {
                            ui.label("No jobs");
                        }
                        for line in &self.model.jobs {
                            ui.label(line);
                        }
                    }
                }
            });
    }

    fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                let selection = selection_label(&self.model.selection.primary);
                let board_info = self
                    .model
                    .ir
                    .as_ref()
                    .map(|ir| format!("Board {:.1}x{:.1}mm", ir.board.bounds.width(), ir.board.bounds.height()))
                    .unwrap_or_else(|| "No board".to_owned());

                ui.horizontal(|ui| {
                    ui.label(board_info);
                    ui.separator();
                    ui.label(format!("Selection: {selection}"));
                });
            });
    }
}

struct EditorBehavior<'a> {
    model: &'a WorkbenchModel,
    canvas2d: &'a mut Pcb2dCanvas,
    canvas3d: &'a mut Pcb3dCanvas,
    fit_requested: bool,
}

impl Behavior<EditorPane> for EditorBehavior<'_> {
    fn tab_title_for_pane(&mut self, pane: &EditorPane) -> egui::WidgetText {
        match pane {
            EditorPane::Pcb2D => "PCB 2D".into(),
            EditorPane::Pcb3D => "PCB 3D".into(),
            EditorPane::Spec => "Spec".into(),
        }
    }

    fn pane_ui(&mut self, ui: &mut egui::Ui, _tile_id: TileId, pane: &mut EditorPane) -> UiResponse {
        match pane {
            EditorPane::Pcb2D => self.canvas2d.ui(ui, self.model, self.fit_requested),
            EditorPane::Pcb3D => self.canvas3d.ui(ui, self.model, false),
            EditorPane::Spec => {
                ui.heading("Spec Editor");
                ui.label("Spec editing pane (M1 scaffold)");
            }
        }
        UiResponse::None
    }
}

impl efame::App for ShellApp {
    fn save(&mut self, storage: &mut dyn efame::Storage) {
        efame::set_value(storage, STORAGE_LAYOUT_KEY, &self.layout);
        efame::set_value(storage, STORAGE_PANELS_KEY, &self.panel_visibility);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut efame::Frame) {
        self.handle_shortcuts(ctx);
        self.process_queue(ctx, frame);

        self.render_menu(ctx);
        self.render_sidebar(ctx);
        self.render_bottom_panel(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let fit_requested = self.layout.request_fit;
            self.layout.request_fit = false;

            let mut behavior = EditorBehavior {
                model: &self.model,
                canvas2d: &mut self.canvas2d,
                canvas3d: &mut self.canvas3d,
                fit_requested,
            };
            self.layout.editor_tree.ui(&mut behavior, ui);
        });

        self.render_status_bar(ctx);
        self.show_palette_window(ctx);

        if ctx.input(|i| i.raw.events.iter().any(|e| matches!(e, Event::Key { key: egui::Key::Escape, pressed: true, .. })))
            && self.show_command_palette
        {
            self.show_command_palette = false;
        }
    }
}
