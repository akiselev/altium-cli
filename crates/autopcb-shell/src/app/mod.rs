mod tabs;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;

use efame::egui::{self, Event, Key};

use self::tabs::{TabProviderRegistry, TabRenderer};
use crate::canvas::{Pcb2dCanvas, Pcb3dCanvas, PcbCanvasView};
use crate::commands::{
    build_context, dispatch, selection_label, shortcut_from_stored, shortcut_to_stored,
    CommandRegistry, DispatchOutcome, ShortcutDef, StoredShortcut,
};
use crate::layout::{BottomTab, ShellLayoutState};
use crate::workbench::{BoardViewMode, DocumentId, DocumentKind, SelectionKind, WorkbenchModel};

const STORAGE_LAYOUT_KEY: &str = "shell.layout.v1";
const STORAGE_PANELS_KEY: &str = "shell.panels.v1";
const STORAGE_SHORTCUTS_KEY: &str = "shell.shortcuts.v1";

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

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
struct ShortcutOverrides {
    by_command: BTreeMap<String, StoredShortcut>,
}

pub struct ShellApp {
    model: WorkbenchModel,
    layout: ShellLayoutState,
    panel_visibility: PanelVisibilityState,
    commands: CommandRegistry,
    queued: VecDeque<(String, Option<String>)>,
    show_command_palette: bool,
    palette_filter: String,
    palette_selected: usize,
    keybindings_filter: String,
    keybindings_capture_for: Option<String>,
    shortcut_bindings: BTreeMap<String, ShortcutDef>,
    tab_registry: TabProviderRegistry,
    tab_renderers: BTreeMap<DocumentId, Box<dyn TabRenderer>>,
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

        let commands = CommandRegistry::new_m1();
        let mut shortcut_bindings = default_shortcuts(&commands);

        if let Some(storage) = cc.storage {
            if let Some(saved) = efame::get_value::<ShortcutOverrides>(storage, STORAGE_SHORTCUTS_KEY)
            {
                for (id, sc) in saved.by_command {
                    if commands.get(&id).is_some() {
                        if let Some(parsed) = shortcut_from_stored(&sc) {
                            shortcut_bindings.insert(id, parsed);
                        }
                    }
                }
            }
        }

        Self {
            model: WorkbenchModel::new(board_path, initial_ir),
            layout,
            panel_visibility,
            commands,
            queued: VecDeque::new(),
            show_command_palette: false,
            palette_filter: String::new(),
            palette_selected: 0,
            keybindings_filter: String::new(),
            keybindings_capture_for: None,
            shortcut_bindings,
            tab_registry: TabProviderRegistry::new_m1(),
            tab_renderers: BTreeMap::new(),
            canvas2d: Pcb2dCanvas::default(),
            canvas3d: Pcb3dCanvas,
        }
    }

    fn queue(&mut self, id: &str, arg: Option<String>) {
        self.queued.push_back((id.to_owned(), arg));
    }

    fn command_context(&self) -> crate::commands::CommandContext {
        let focus_2d = self
            .model
            .active_board()
            .is_some_and(|b| b.view_mode == BoardViewMode::TwoD);
        let focus_3d = self
            .model
            .active_board()
            .is_some_and(|b| b.view_mode == BoardViewMode::ThreeD);
        build_context(&self.model, focus_2d, focus_3d)
    }

    fn execute_command(
        &mut self,
        id: &str,
        arg: Option<String>,
        ctx: &egui::Context,
        _frame: &mut efame::Frame,
    ) {
        let Some(meta) = self.commands.get(id) else {
            self.model
                .problems
                .push(format!("Unknown command requested: {id}"));
            return;
        };

        let cmd_ctx = self.command_context();
        if !self.commands.is_enabled(meta, &cmd_ctx) {
            self.model
                .output_lines
                .push(format!("Command disabled in current context: {}", meta.id));
            return;
        }

        match dispatch(
            &meta.id,
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

    fn process_queue(&mut self, ctx: &egui::Context, frame: &mut efame::Frame) {
        while let Some((id, arg)) = self.queued.pop_front() {
            self.execute_command(&id, arg, ctx, frame);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.keybindings_capture_for.is_some() {
            return;
        }

        let cmd_ctx = self.command_context();
        let mut shortcut_entries: Vec<(String, ShortcutDef, usize)> = self
            .shortcut_bindings
            .iter()
            .filter_map(|(id, sc)| {
                let meta = self.commands.get(id)?;
                if !self.commands.is_enabled(meta, &cmd_ctx) {
                    return None;
                }
                let spec = usize::from(sc.modifiers.command)
                    + usize::from(sc.modifiers.ctrl)
                    + usize::from(sc.modifiers.alt)
                    + usize::from(sc.modifiers.shift);
                Some((id.clone(), *sc, spec))
            })
            .collect();

        shortcut_entries.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

        let mut triggered = Vec::new();
        for (id, sc, _) in shortcut_entries {
            let k = sc.as_keyboard_shortcut();
            let consumed = ctx.input_mut(|i| i.consume_shortcut(&k));
            if consumed {
                triggered.push(id);
            }
        }

        for id in triggered {
            self.queue(&id, None);
        }
    }

    fn keybindings_tab_active(&self) -> bool {
        self.model
            .active_document()
            .is_some_and(|d| matches!(d.kind, DocumentKind::Keybindings))
    }

    fn capture_shortcut_if_needed(&mut self, ctx: &egui::Context) {
        if !self.keybindings_tab_active() {
            return;
        }

        let Some(command_id) = self.keybindings_capture_for.clone() else {
            return;
        };

        let event = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| {
                if let Event::Key {
                    key,
                    pressed,
                    repeat,
                    modifiers,
                    ..
                } = e
                {
                    if *pressed && !*repeat {
                        Some((*key, *modifiers))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        });

        let Some((key, modifiers)) = event else {
            return;
        };

        if key == Key::Escape {
            self.keybindings_capture_for = None;
            return;
        }

        let candidate = ShortcutDef::new(
            egui::Modifiers {
                alt: modifiers.alt,
                ctrl: modifiers.ctrl,
                shift: modifiers.shift,
                mac_cmd: false,
                command: modifiers.command,
            },
            key,
        );

        if let Some(conflict) = find_conflict(&self.shortcut_bindings, &command_id, candidate) {
            self.model.problems.push(format!(
                "Shortcut conflict: {} already uses {}",
                conflict,
                candidate.display()
            ));
            self.keybindings_capture_for = None;
            return;
        }

        self.shortcut_bindings.insert(command_id.clone(), candidate);
        self.model
            .output_lines
            .push(format!("Shortcut updated: {command_id} -> {}", candidate.display()));
        self.keybindings_capture_for = None;
    }

    fn show_palette_window(&mut self, ctx: &egui::Context) {
        if !self.show_command_palette {
            return;
        }

        let cmd_ctx = self.command_context();
        let mut open = self.show_command_palette;
        egui::Window::new("Command Palette")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                let resp = ui.text_edit_singleline(&mut self.palette_filter);
                if resp.changed() {
                    self.palette_selected = 0;
                }
                ui.separator();
                let filter = self.palette_filter.to_lowercase();

                let commands: Vec<_> = self
                    .commands
                    .exposed()
                    .filter(|m| self.commands.is_enabled(*m, &cmd_ctx))
                    .filter(|m| {
                        filter.is_empty()
                            || m.title.to_lowercase().contains(&filter)
                            || m.id.contains(&filter)
                    })
                    .collect();

                if !commands.is_empty() {
                    self.palette_selected = self.palette_selected.min(commands.len() - 1);
                } else {
                    self.palette_selected = 0;
                }

                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !commands.is_empty() {
                    self.palette_selected = (self.palette_selected + 1) % commands.len();
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !commands.is_empty() {
                    self.palette_selected = if self.palette_selected == 0 {
                        commands.len() - 1
                    } else {
                        self.palette_selected - 1
                    };
                }

                let mut clicked: Option<&'static str> = None;
                for (idx, meta) in commands.iter().enumerate() {
                    let selected = idx == self.palette_selected;
                    if ui
                        .selectable_label(selected, format!("{} ({})", meta.title, meta.id))
                        .clicked()
                    {
                        clicked = Some(meta.id);
                    }
                }

                if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !commands.is_empty() {
                    clicked = Some(commands[self.palette_selected].id);
                }

                if let Some(id) = clicked {
                    self.queue(id, None);
                    self.show_command_palette = false;
                }
            });
        self.show_command_palette = open;
    }

    fn render_menu(&mut self, ctx: &egui::Context) {
        let mut categories = BTreeSet::new();
        for cmd in self.commands.exposed() {
            categories.insert(cmd.category);
        }

        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                for category in categories.iter().copied() {
                    let commands: Vec<_> = self
                        .commands
                        .exposed()
                        .filter(|c| c.category == category)
                        .collect();
                    if commands.is_empty() {
                        continue;
                    }
                    ui.menu_button(category, |ui| {
                        for cmd in commands {
                            let shortcut = self
                                .shortcut_bindings
                                .get(cmd.id)
                                .map(|s| s.display())
                                .unwrap_or_default();
                            let label = if shortcut.is_empty() {
                                cmd.title.to_owned()
                            } else {
                                format!("{}\t{}", cmd.title, shortcut)
                            };
                            if ui.button(label).clicked() {
                                self.queue(cmd.id, None);
                                ui.close();
                            }
                        }
                    });
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
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Explorer");
                ui.separator();

                let Some(board) = self.model.active_board() else {
                    ui.label("No active board document");
                    return;
                };
                let ir = &board.ir;

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
                                SelectionKind::Component(d) if d == designator
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
                                SelectionKind::Net(n) if n == name
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

    fn render_document_tabs(&mut self, ui: &mut egui::Ui) {
        let tabs: Vec<_> = self
            .model
            .documents_in_tab_order()
            .map(|doc| (doc.id, doc.title.clone(), doc.dirty))
            .collect();

        ui.horizontal_wrapped(|ui| {
            for (id, title, dirty) in tabs {
                let mut label = title;
                if dirty {
                    label.push('*');
                }
                let selected = self.model.active_editor_tab == Some(id);
                if ui.selectable_label(selected, label).clicked() {
                    self.queue("editor.activate_document", Some(id.0.to_string()));
                }
            }
        });
        ui.separator();
    }

    pub(super) fn render_keybindings_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Keyboard Shortcuts");
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.keybindings_filter);
        });
        if let Some(id) = &self.keybindings_capture_for {
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("Recording new shortcut for {id}. Press Esc to cancel."),
            );
        }
        ui.separator();

        let filter = self.keybindings_filter.to_lowercase();
        let commands: Vec<_> = self
            .commands
            .all()
            .filter(|m| {
                filter.is_empty()
                    || m.id.to_lowercase().contains(&filter)
                    || m.title.to_lowercase().contains(&filter)
                    || m.category.to_lowercase().contains(&filter)
            })
            .collect();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for meta in commands {
                ui.horizontal(|ui| {
                    ui.set_min_width(860.0);
                    ui.label(format!("{} [{}]", meta.title, meta.id));

                    let current = self
                        .shortcut_bindings
                        .get(meta.id)
                        .map(|s| s.display())
                        .unwrap_or_else(|| "<unbound>".to_owned());
                    ui.monospace(current);

                    if ui.button("Set").clicked() {
                        self.keybindings_capture_for = Some(meta.id.to_owned());
                    }

                    if ui.button("Clear").clicked() {
                        self.shortcut_bindings.remove(meta.id);
                    }

                    if ui.button("Reset").clicked() {
                        if let Some(default) = self.commands.default_shortcut(meta.id) {
                            self.shortcut_bindings.insert(meta.id.to_owned(), default);
                        } else {
                            self.shortcut_bindings.remove(meta.id);
                        }
                    }
                });
                ui.separator();
            }
        });
    }

    pub(super) fn render_board_document(
        &mut self,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        fit_requested: bool,
    ) {
        let mode = self
            .model
            .documents
            .get(&document_id)
            .and_then(|doc| match &doc.kind {
                DocumentKind::Board(board) => Some(board.view_mode),
                _ => None,
            });

        let Some(mode) = mode else {
            ui.label("Board tab unavailable");
            return;
        };

        ui.horizontal(|ui| {
            if ui
                .selectable_label(mode == BoardViewMode::TwoD, "2D")
                .clicked()
            {
                self.queue("pcb.view.2d", None);
            }
            if ui
                .selectable_label(mode == BoardViewMode::ThreeD, "3D")
                .clicked()
            {
                self.queue("pcb.view.3d", None);
            }
        });
        ui.separator();

        let selection = self.model.selection.primary.clone();
        if let Some(board) = self.model.active_board() {
            match mode {
                BoardViewMode::TwoD => self.canvas2d.ui(ui, &board.ir, &selection, fit_requested),
                BoardViewMode::ThreeD => self.canvas3d.ui(ui, &board.ir, &selection, fit_requested),
            }
        }
    }

    pub(super) fn render_spec_document(&mut self, ui: &mut egui::Ui, document_id: DocumentId) {
        let edited = if let Some(doc) = self.model.documents.get_mut(&document_id) {
            if let DocumentKind::Spec(spec) = &mut doc.kind {
                let response = ui.add(
                    egui::TextEdit::multiline(&mut spec.text)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(24)
                        .desired_width(f32::INFINITY),
                );
                response.changed()
            } else {
                ui.label("Spec tab unavailable");
                false
            }
        } else {
            ui.label("Spec tab unavailable");
            false
        };

        if edited {
            if let Some(doc) = self.model.documents.get_mut(&document_id) {
                doc.dirty = true;
            }
        }
    }

    fn tab_renderer_for_document(
        &mut self,
        document_id: DocumentId,
    ) -> Option<Box<dyn TabRenderer>> {
        if let Some(existing) = self.tab_renderers.remove(&document_id) {
            return Some(existing);
        }
        let kind_id = self.model.documents.get(&document_id)?.kind_id();
        self.tab_registry.instantiate(kind_id)
    }

    fn render_active_document(&mut self, ui: &mut egui::Ui, fit_requested: bool) {
        let active_id = self.model.active_editor_tab;
        let Some(active_id) = active_id else {
            ui.centered_and_justified(|ui| {
                ui.label("No document open");
            });
            return;
        };

        let Some(mut renderer) = self.tab_renderer_for_document(active_id) else {
            ui.label("No tab provider registered for this document type");
            return;
        };

        renderer.render(self, ui, active_id, fit_requested);
        if self.model.documents.contains_key(&active_id) {
            self.tab_renderers.insert(active_id, renderer);
        }
    }

    fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                let selection = selection_label(&self.model.selection.primary);
                let active_doc = self
                    .model
                    .active_document()
                    .map(|d| d.title.clone())
                    .unwrap_or_else(|| "No doc".to_owned());
                let active_path = self
                    .model
                    .active_document()
                    .and_then(|d| d.path.as_ref())
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "<unsaved>".to_owned());
                let board_info = self
                    .model
                    .active_board()
                    .map(|b| {
                        format!(
                            "Board {:.1}x{:.1}mm",
                            b.ir.board.bounds.width(),
                            b.ir.board.bounds.height()
                        )
                    })
                    .unwrap_or_else(|| "No board".to_owned());

                ui.horizontal(|ui| {
                    ui.label(active_doc);
                    ui.separator();
                    ui.label(active_path);
                    ui.separator();
                    ui.label(board_info);
                    ui.separator();
                    ui.label(format!("Selection: {selection}"));
                });
            });
    }
}

fn default_shortcuts(commands: &CommandRegistry) -> BTreeMap<String, ShortcutDef> {
    commands
        .all()
        .filter_map(|m| m.default_shortcut.map(|s| (m.id.to_owned(), s)))
        .collect()
}

fn find_conflict(
    bindings: &BTreeMap<String, ShortcutDef>,
    target_command: &str,
    candidate: ShortcutDef,
) -> Option<String> {
    bindings
        .iter()
        .find(|(id, existing)| id.as_str() != target_command && **existing == candidate)
        .map(|(id, _)| id.clone())
}

impl efame::App for ShellApp {
    fn save(&mut self, storage: &mut dyn efame::Storage) {
        efame::set_value(storage, STORAGE_LAYOUT_KEY, &self.layout);
        efame::set_value(storage, STORAGE_PANELS_KEY, &self.panel_visibility);

        let persisted = ShortcutOverrides {
            by_command: self
                .shortcut_bindings
                .iter()
                .map(|(k, v)| (k.clone(), shortcut_to_stored(*v)))
                .collect(),
        };
        efame::set_value(storage, STORAGE_SHORTCUTS_KEY, &persisted);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut efame::Frame) {
        self.capture_shortcut_if_needed(ctx);
        self.handle_shortcuts(ctx);
        self.process_queue(ctx, frame);

        self.render_menu(ctx);
        self.render_sidebar(ctx);
        self.render_bottom_panel(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let fit_requested = self.layout.request_fit;
            self.layout.request_fit = false;

            self.render_document_tabs(ui);
            self.render_active_document(ui, fit_requested);
        });

        self.render_status_bar(ctx);
        self.show_palette_window(ctx);

        if ctx.input(|i| i.raw.events.iter().any(|e| matches!(e, Event::Key { key: Key::Escape, pressed: true, .. })))
            && self.show_command_palette
        {
            self.show_command_palette = false;
        }
    }
}
