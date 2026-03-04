mod tabs;

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use efame::egui::{self, ColorImage, Event, Key, RichText, UserData, ViewportCommand};
use egui_tiles::{Behavior, TileId, UiResponse};

use self::tabs::{TabProviderRegistry, TabRenderer};
use crate::canvas::{Pcb2dCanvas, Pcb3dCanvas, PcbCanvasView};
use crate::commands::{
    build_context, dispatch, selection_label, shortcut_from_stored, shortcut_to_stored,
    CommandRegistry, DispatchOutcome, ShortcutDef, StoredShortcut,
};
use crate::ipc::{IpcRequest, UiTestOp};
use crate::layout::{BottomTab, EditorPane, ShellLayoutState};
use crate::ui::icons::{IconId, icon, icon_button};
use crate::ui::tabstrip::{TabAction, render_tabstrip};
use crate::ui::theme::{ThemeTokens, apply_theme, vscode_dark_tokens};
use crate::workbench::{BoardViewMode, DocumentId, DocumentKind, SelectionKind, WorkbenchModel};

const STORAGE_LAYOUT_KEY: &str = "shell.layout.v1";
const STORAGE_PANELS_KEY: &str = "shell.panels.v1";
const STORAGE_CHROME_KEY: &str = "shell.chrome.v2";
const STORAGE_SHORTCUTS_KEY: &str = "shell.shortcuts.v1";
const STORAGE_EDITOR_SPLIT_KEY: &str = "shell.editor_split.v1";
const LAYOUT_PROBE_PATH: &str = "/tmp/autopcb-shell-layout.json";

#[cfg(test)]
fn clamp_bottom_panel_height(current: f32, drag_delta_y: f32, viewport_h: f32) -> f32 {
    let max_h = (viewport_h - 120.0).max(80.0);
    (current - drag_delta_y).clamp(80.0, max_h)
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
enum ActivityView {
    Explorer,
    Search,
    SourceControl,
    Run,
    Extensions,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PanelVisibilityState {
    show_activity_bar: bool,
    show_primary_sidebar: bool,
    show_bottom_panel: bool,
    bottom_panel_height: f32,
    show_status_bar: bool,
    activity_view: ActivityView,
    bottom_tab: BottomTab,
}

impl Default for PanelVisibilityState {
    fn default() -> Self {
        Self {
            show_activity_bar: true,
            show_primary_sidebar: true,
            show_bottom_panel: true,
            bottom_panel_height: 180.0,
            show_status_bar: true,
            activity_view: ActivityView::Explorer,
            bottom_tab: BottomTab::Output,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct EditorSplitState {
    is_split: bool,
    split_vertical: bool,
    secondary_active_tab: Option<DocumentId>,
}

impl Default for EditorSplitState {
    fn default() -> Self {
        Self {
            is_split: false,
            split_vertical: true,
            secondary_active_tab: None,
        }
    }
}

#[derive(Debug, Clone)]
struct DragScript {
    start: egui::Pos2,
    end: egui::Pos2,
    steps: u32,
    current_step: u32,
    phase: u8,
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
    explorer_filter: String,
    keybindings_filter: String,
    keybindings_capture_for: Option<String>,
    shortcut_bindings: BTreeMap<String, ShortcutDef>,
    ipc_rx: Option<Receiver<IpcRequest>>,
    screenshot_path: Option<PathBuf>,
    screenshot_requested: bool,
    pending_ui_test_ops: VecDeque<UiTestOp>,
    active_drag_script: Option<DragScript>,
    tab_registry: TabProviderRegistry,
    tab_renderers: BTreeMap<DocumentId, Box<dyn TabRenderer>>,
    theme: ThemeTokens,
    editor_split: EditorSplitState,
    last_bottom_panel_height: f32,
    last_status_bar_height: f32,
    last_central_height: f32,
    last_drag_start_y: f32,
    last_drag_end_y: f32,
    canvas2d: Pcb2dCanvas,
    canvas3d: Pcb3dCanvas,
}

impl ShellApp {
    pub fn new(
        cc: &efame::CreationContext<'_>,
        board_path: Option<PathBuf>,
        initial_ir: Option<autopcb_ir::PcbIr>,
        ipc_rx: Option<Receiver<IpcRequest>>,
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
            if let Some(saved) = efame::get_value(storage, STORAGE_CHROME_KEY) {
                panel_visibility = saved;
            }
        }
        layout.ensure_required_panes();
        let mut editor_split = EditorSplitState::default();
        if let Some(storage) = cc.storage {
            if let Some(saved) = efame::get_value(storage, STORAGE_EDITOR_SPLIT_KEY) {
                editor_split = saved;
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
            explorer_filter: String::new(),
            keybindings_filter: String::new(),
            keybindings_capture_for: None,
            shortcut_bindings,
            ipc_rx,
            screenshot_path: None,
            screenshot_requested: false,
            pending_ui_test_ops: VecDeque::new(),
            active_drag_script: None,
            tab_registry: TabProviderRegistry::new_m1(),
            tab_renderers: BTreeMap::new(),
            theme: vscode_dark_tokens(),
            editor_split,
            last_bottom_panel_height: 0.0,
            last_status_bar_height: 24.0,
            last_central_height: 0.0,
            last_drag_start_y: 0.0,
            last_drag_end_y: 0.0,
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

        if id == "view.reset_layout" {
            self.editor_split = EditorSplitState::default();
        }

        if self.execute_io_command(id, arg.clone()) {
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

    fn execute_io_command(&mut self, id: &str, arg: Option<String>) -> bool {
        match id {
            "view.toggle_activity_bar" => {
                self.panel_visibility.show_activity_bar = !self.panel_visibility.show_activity_bar;
                true
            }
            "view.toggle_status_bar" => {
                self.panel_visibility.show_status_bar = !self.panel_visibility.show_status_bar;
                true
            }
            "panel.show.explorer" => {
                self.panel_visibility.show_primary_sidebar = true;
                self.panel_visibility.activity_view = ActivityView::Explorer;
                true
            }
            "panel.show.search" => {
                self.panel_visibility.show_primary_sidebar = true;
                self.panel_visibility.activity_view = ActivityView::Search;
                true
            }
            "panel.show.source_control" => {
                self.panel_visibility.show_primary_sidebar = true;
                self.panel_visibility.activity_view = ActivityView::SourceControl;
                true
            }
            "panel.show.run" => {
                self.panel_visibility.show_primary_sidebar = true;
                self.panel_visibility.activity_view = ActivityView::Run;
                true
            }
            "panel.show.extensions" => {
                self.panel_visibility.show_primary_sidebar = true;
                self.panel_visibility.activity_view = ActivityView::Extensions;
                true
            }
            "workspace.open" | "file.open_folder" => {
                let root = arg
                    .map(PathBuf::from)
                    .or_else(|| self.model.workspace_root.clone())
                    .or_else(|| std::env::current_dir().ok());
                let Some(root) = root else {
                    self.model.problems.push("Unable to resolve workspace root".to_owned());
                    return true;
                };
                self.model.set_workspace_root(root.clone());
                self.model
                    .output_lines
                    .push(format!("Workspace opened: {}", root.display()));
                true
            }
            "workspace.close" => {
                self.model.clear_workspace();
                self.tab_renderers.clear();
                self.model.output_lines.push("Workspace closed".to_owned());
                true
            }
            "file.new_spec" => {
                self.model
                    .open_spec_document(None, "// New spec document\n".to_owned());
                true
            }
            "file.open" => {
                if let Some(path) = arg.map(PathBuf::from) {
                    self.open_document_path(path);
                } else {
                    self.model.output_lines.push(
                        "Use Explorer or pass a path to File: Open from command palette".to_owned(),
                    );
                }
                true
            }
            "file.save" => {
                self.save_active_document();
                true
            }
            "file.save_all" => {
                self.save_all_documents();
                true
            }
            "file.revert" => {
                self.revert_active_document();
                true
            }
            "view.split_editor_right" => {
                self.editor_split.is_split = true;
                self.editor_split.split_vertical = true;
                if self.editor_split.secondary_active_tab.is_none() {
                    self.editor_split.secondary_active_tab = self.model.active_document_id();
                }
                self.model.output_lines.push("Split editor: right".to_owned());
                true
            }
            "view.split_editor_down" => {
                self.editor_split.is_split = true;
                self.editor_split.split_vertical = false;
                if self.editor_split.secondary_active_tab.is_none() {
                    self.editor_split.secondary_active_tab = self.model.active_document_id();
                }
                self.model.output_lines.push("Split editor: down".to_owned());
                true
            }
            "help.about" => {
                self.model
                    .output_lines
                    .push("AutoPCB Shell - IDE shell for PCB/spec automation".to_owned());
                true
            }
            "run.start_last" => {
                self.model
                    .output_lines
                    .push("No runnable task configured yet.".to_owned());
                true
            }
            _ => false,
        }
    }

    fn open_document_path(&mut self, path: PathBuf) {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "pcbdoc" => {
                match altium_format::PcbDoc::open(&path).and_then(|doc| doc.board()) {
                    Ok(board) => match autopcb_ir::PcbIr::extract(&board) {
                        Ok(ir) => {
                            self.model.open_board_document(path.clone(), ir);
                            self.model
                                .output_lines
                                .push(format!("Opened board: {}", path.display()));
                        }
                        Err(err) => self
                            .model
                            .problems
                            .push(format!("Failed to extract board {}: {err}", path.display())),
                    },
                    Err(err) => self
                        .model
                        .problems
                        .push(format!("Failed to open board {}: {err}", path.display())),
                };
            }
            "spec" | "pcbdoc-spec" => match fs::read_to_string(&path) {
                Ok(text) => {
                    self.model.open_spec_document(Some(path.clone()), text);
                    self.model
                        .output_lines
                        .push(format!("Opened spec: {}", path.display()));
                }
                Err(err) => self
                    .model
                    .problems
                    .push(format!("Failed to open spec {}: {err}", path.display())),
            },
            _ => {
                self.model.problems.push(format!(
                    "Unsupported file type for open: {}",
                    path.display()
                ));
            }
        }
    }

    fn save_active_document(&mut self) {
        let Some(id) = self.model.active_document_id() else {
            return;
        };
        self.save_document(id);
    }

    fn save_all_documents(&mut self) {
        let ids = self.model.open_editor_tabs.clone();
        for id in ids {
            self.save_document(id);
        }
    }

    fn save_document(&mut self, id: DocumentId) {
        let Some(doc) = self.model.documents.get(&id) else {
            return;
        };

        let (path, text) = match &doc.kind {
            DocumentKind::Spec(spec) => {
                let target = spec.path.clone().or_else(|| {
                    let base = self
                        .model
                        .workspace_root
                        .clone()
                        .or_else(|| std::env::current_dir().ok())
                        .unwrap_or_else(|| PathBuf::from("."));
                    Some(base.join(format!("untitled-{}.pcbdoc-spec", id.0)))
                });
                (target, spec.text.clone())
            }
            _ => return,
        };

        let Some(path) = path else {
            return;
        };

        match fs::write(&path, text) {
            Ok(_) => {
                if let Some(doc) = self.model.documents.get_mut(&id) {
                    if let DocumentKind::Spec(spec) = &mut doc.kind {
                        spec.path = Some(path.clone());
                    }
                    doc.path = Some(path.clone());
                    doc.title = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("spec")
                        .to_owned();
                    doc.dirty = false;
                }
                self.model
                    .output_lines
                    .push(format!("Saved: {}", path.display()));
            }
            Err(err) => self
                .model
                .problems
                .push(format!("Failed to save {}: {err}", path.display())),
        }
    }

    fn revert_active_document(&mut self) {
        let Some(id) = self.model.active_document_id() else {
            return;
        };

        let Some(doc) = self.model.documents.get(&id) else {
            return;
        };
        let Some(path) = doc.path.clone() else {
            self.model
                .output_lines
                .push("Cannot revert unsaved document".to_owned());
            return;
        };

        if !matches!(doc.kind, DocumentKind::Spec(_)) {
            self.model
                .output_lines
                .push("Revert currently supports spec documents".to_owned());
            return;
        }

        match fs::read_to_string(&path) {
            Ok(text) => {
                if let Some(doc) = self.model.documents.get_mut(&id) {
                    if let DocumentKind::Spec(spec) = &mut doc.kind {
                        spec.text = text;
                    }
                    doc.dirty = false;
                }
                self.model
                    .output_lines
                    .push(format!("Reverted: {}", path.display()));
            }
            Err(err) => self
                .model
                .problems
                .push(format!("Failed to revert {}: {err}", path.display())),
        }
    }

    fn process_queue(&mut self, ctx: &egui::Context, frame: &mut efame::Frame) {
        while let Some((id, arg)) = self.queued.pop_front() {
            self.execute_command(&id, arg, ctx, frame);
        }
    }

    fn prune_tab_renderers(&mut self) {
        self.tab_renderers.retain(|id, _| {
            self.model.documents.contains_key(id) && self.model.open_editor_tabs.contains(id)
        });
        if let Some(id) = self.editor_split.secondary_active_tab {
            if !self.model.open_editor_tabs.contains(&id) {
                self.editor_split.secondary_active_tab = self.model.active_document_id();
            }
        }
        if self.model.open_editor_tabs.is_empty() {
            self.editor_split.is_split = false;
            self.editor_split.secondary_active_tab = None;
        }
    }

    fn process_ipc(&mut self) {
        let mut drained = Vec::new();
        if let Some(rx) = &self.ipc_rx {
            while let Ok(req) = rx.try_recv() {
                drained.push(req);
            }
        }
        for req in drained {
            match req {
                IpcRequest::Ping => self.model.output_lines.push("IPC ping".to_owned()),
                IpcRequest::Command { id, arg } => self.queue(&id, arg),
                IpcRequest::OpenFile { path } => self.queue("file.open", Some(path)),
                IpcRequest::Screenshot { path } => {
                    self.screenshot_path = Some(PathBuf::from(path));
                }
                IpcRequest::UiTest { op } => self.pending_ui_test_ops.push_back(op),
            }
        }
    }

    fn apply_ui_test_ops(&mut self, ctx: &egui::Context) {
        if self.active_drag_script.is_none() {
            while let Some(op) = self.pending_ui_test_ops.pop_front() {
                match op {
                    UiTestOp::DragBottomPanel { delta, steps } => {
                        let steps = steps.max(2);
                        let screen = ctx.content_rect();
                        let splitter_y =
                            screen.bottom() - self.last_status_bar_height - self.last_bottom_panel_height;
                        let x = screen.center().x;
                        let start = egui::pos2(x, splitter_y + 2.0);
                        let end = egui::pos2(x, splitter_y + delta);
                        self.last_drag_start_y = start.y;
                        self.last_drag_end_y = end.y;
                        self.active_drag_script = Some(DragScript {
                            start,
                            end,
                            steps,
                            current_step: 0,
                            phase: 0,
                        });
                        self.model.output_lines.push(format!(
                            "UI test drag queued: start_y={:.1} end_y={:.1} delta={:.1}",
                            start.y, end.y, delta
                        ));
                        break;
                    }
                }
            }
        }

        let Some(script) = self.active_drag_script.as_mut() else {
            return;
        };

        let mut completed = false;
        ctx.input_mut(|i| match script.phase {
            0 => {
                i.raw.events.push(Event::PointerMoved(script.start));
                script.phase = 1;
            }
            1 => {
                i.raw.events.push(Event::PointerButton {
                    pos: script.start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
                script.phase = 2;
            }
            2 => {
                if script.current_step < script.steps {
                    script.current_step += 1;
                    let t = script.current_step as f32 / script.steps as f32;
                    let pos = egui::pos2(
                        script.start.x + (script.end.x - script.start.x) * t,
                        script.start.y + (script.end.y - script.start.y) * t,
                    );
                    i.raw.events.push(Event::PointerMoved(pos));
                } else {
                    script.phase = 3;
                }
            }
            3 => {
                i.raw.events.push(Event::PointerButton {
                    pos: script.end,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                });
                script.phase = 4;
            }
            _ => {
                completed = true;
            }
        });

        if completed {
            self.active_drag_script = None;
            self.model.output_lines.push("UI test drag completed".to_owned());
        }
    }

    fn handle_screenshot_flow(&mut self, ctx: &egui::Context) {
        if self.screenshot_path.is_some() && !self.screenshot_requested {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
            self.screenshot_requested = true;
        }

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
            let target = self
                .screenshot_path
                .take()
                .unwrap_or_else(|| PathBuf::from("screenshot.png"));
            if let Err(err) = save_screenshot_png(&image, &target) {
                self.model
                    .problems
                    .push(format!("Failed to save screenshot {}: {err}", target.display()));
            } else {
                self.model
                    .output_lines
                    .push(format!("Screenshot saved to {}", target.display()));
            }
            self.screenshot_requested = false;
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

    fn render_title_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("title_menu")
            .exact_height(28.0)
            .frame(egui::Frame::new().fill(self.theme.titlebar_bg))
            .show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(self.theme.text_primary);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("AutoPCB").strong().size(12.0));
                    ui.separator();
                    self.render_menu_bar_buttons(ui);
                });
            });
    }

    fn render_menu_bar_buttons(&mut self, ui: &mut egui::Ui) {
        let ordered = [
            "File",
            "Edit",
            "Selection",
            "View",
            "Go",
            "Run",
            "Terminal",
            "Help",
            "App",
            "Workspace",
            "Navigate",
            "PCB",
            "Panel",
            "History",
            "Editor",
        ];
        let mut by_category: BTreeMap<&str, Vec<_>> = BTreeMap::new();
        for cmd in self.commands.exposed() {
            by_category.entry(cmd.category).or_default().push(cmd);
        }

        for category in ordered {
            let Some(commands) = by_category.get(category) else {
                continue;
            };
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
    }

    fn render_activity_bar(&mut self, ctx: &egui::Context) {
        if !self.panel_visibility.show_activity_bar {
            return;
        }
        egui::SidePanel::left("activity_bar")
            .exact_width(42.0)
            .frame(egui::Frame::new().fill(self.theme.activitybar_bg))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    self.activity_button(ui, IconId::Explorer, ActivityView::Explorer);
                    self.activity_button(ui, IconId::Search, ActivityView::Search);
                    self.activity_button(ui, IconId::SourceControl, ActivityView::SourceControl);
                    self.activity_button(ui, IconId::Run, ActivityView::Run);
                    self.activity_button(ui, IconId::Extensions, ActivityView::Extensions);
                });
            });
    }

    fn activity_button(&mut self, ui: &mut egui::Ui, icon_id: IconId, view: ActivityView) {
        let selected = self.panel_visibility.activity_view == view;
        let resp = icon_button(ui, icon_id, selected, self.theme.text_primary, 28.0);
        if resp.clicked() {
            if selected {
                self.panel_visibility.show_primary_sidebar = !self.panel_visibility.show_primary_sidebar;
            } else {
                self.panel_visibility.activity_view = view;
                self.panel_visibility.show_primary_sidebar = true;
            }
        }
    }

    fn render_sidebar(&mut self, ctx: &egui::Context) {
        if !self.panel_visibility.show_primary_sidebar {
            return;
        }

        egui::SidePanel::left("primary_sidebar")
            .resizable(true)
            .default_width(280.0)
            .frame(egui::Frame::new().fill(self.theme.sidebar_bg))
            .show(ctx, |ui| match self.panel_visibility.activity_view {
                ActivityView::Explorer => self.render_explorer_sidebar(ui),
                ActivityView::Search => self.render_placeholder_sidebar(ui, "SEARCH", "Workspace text search is planned."),
                ActivityView::SourceControl => {
                    self.render_placeholder_sidebar(ui, "SOURCE CONTROL", "Source-control integration is planned.")
                }
                ActivityView::Run => self.render_placeholder_sidebar(ui, "RUN", "Automation run tasks will live here."),
                ActivityView::Extensions => {
                    self.render_placeholder_sidebar(ui, "EXTENSIONS", "Plugin/extension management is planned.")
                }
            });
    }

    fn render_explorer_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("EXPLORER").small().color(self.theme.text_muted));
        ui.separator();
        self.render_workspace_files(ui);
        ui.separator();

        let Some(board) = self.model.active_board() else {
            ui.label(RichText::new("No active board document").color(self.theme.text_muted));
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
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
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
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for (name, pins_len) in &nets {
                    let selected = matches!(&self.model.selection.primary, SelectionKind::Net(n) if n == name);
                    if ui
                        .selectable_label(selected, format!("{} ({})", name, pins_len))
                        .clicked()
                    {
                        self.queue("crossprobe.select_net", Some(name.clone()));
                    }
                }
            });
        });
    }

    fn render_placeholder_sidebar(&mut self, ui: &mut egui::Ui, heading: &str, text: &str) {
        ui.label(RichText::new(heading).small().color(self.theme.text_muted));
        ui.separator();
        ui.label(RichText::new(text).color(self.theme.text_disabled));
    }

    fn render_workspace_files(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Workspace Files", |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.explorer_filter);
            });

            let Some(root) = self.model.workspace_root.clone() else {
                ui.label("No workspace open");
                return;
            };

            ui.small(RichText::new(root.display().to_string()).color(self.theme.text_muted));
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| self.render_dir_tree(ui, &root, 0));
        });
    }

    fn render_dir_tree(&mut self, ui: &mut egui::Ui, dir: &Path, depth: usize) {
        if depth > 4 {
            return;
        }

        let mut entries = match fs::read_dir(dir) {
            Ok(read_dir) => read_dir.filter_map(Result::ok).collect::<Vec<_>>(),
            Err(_) => return,
        };
        entries.sort_by_key(|e| e.path());

        let filter = self.explorer_filter.to_ascii_lowercase();
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }

            let passes_filter = filter.is_empty()
                || name.to_ascii_lowercase().contains(&filter)
                || path
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(&filter);
            if !passes_filter {
                continue;
            }

            if path.is_dir() {
                ui.horizontal(|ui| {
                    icon(ui, IconId::Folder, self.theme.text_muted, 12.0);
                    ui.collapsing(format!("{name}/"), |ui| {
                        self.render_dir_tree(ui, &path, depth + 1);
                    });
                });
                continue;
            }

            let is_open = self
                .model
                .find_document_by_path(&path)
                .is_some_and(|id| self.model.active_editor_tab == Some(id));
            ui.horizontal(|ui| {
                let icon_id = if name.to_ascii_lowercase().ends_with(".pcbdoc") {
                    IconId::PcbDoc
                } else if name.to_ascii_lowercase().ends_with(".spec")
                    || name.to_ascii_lowercase().ends_with(".pcbdoc-spec")
                {
                    IconId::Spec
                } else {
                    IconId::File
                };
                icon(ui, icon_id, self.theme.text_muted, 12.0);
                if ui.selectable_label(is_open, &name).clicked() {
                    self.queue("file.open", Some(path.display().to_string()));
                }
            });
        }
    }

    fn render_bottom_panel_contents(&mut self, ui: &mut egui::Ui) {
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
                    ui.label(RichText::new("No problems").color(self.theme.text_muted));
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
                    ui.label(RichText::new("No jobs").color(self.theme.text_muted));
                }
                for line in &self.model.jobs {
                    ui.label(line);
                }
            }
        }
    }

    fn render_document_tabs(&mut self, ui: &mut egui::Ui) {
        let actions = render_tabstrip(ui, &self.model, &self.theme, self.model.active_editor_tab);
        for action in actions {
            match action {
                TabAction::Activate(id) => {
                    self.queue("editor.activate_document", Some(id.0.to_string()));
                }
                TabAction::Close(id) => {
                    self.queue("editor.close_document", Some(id.0.to_string()));
                    if self.editor_split.secondary_active_tab == Some(id) {
                        self.editor_split.secondary_active_tab = self.model.active_editor_tab;
                    }
                }
            }
        }
    }

    fn render_secondary_document_tabs(&mut self, ui: &mut egui::Ui) {
        let active = self.editor_split.secondary_active_tab;
        let actions = render_tabstrip(ui, &self.model, &self.theme, active);
        for action in actions {
            match action {
                TabAction::Activate(id) => self.editor_split.secondary_active_tab = Some(id),
                TabAction::Close(id) => {
                    self.queue("editor.close_document", Some(id.0.to_string()));
                    if self.editor_split.secondary_active_tab == Some(id) {
                        self.editor_split.secondary_active_tab = self.model.active_editor_tab;
                    }
                }
            }
        }
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
            self.model.mark_document_dirty(document_id, true);
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

    fn render_document_by_id(&mut self, ui: &mut egui::Ui, document_id: DocumentId, fit_requested: bool) {
        let Some(mut renderer) = self.tab_renderer_for_document(document_id) else {
            ui.label("No tab provider registered for this document type");
            return;
        };
        renderer.render(self, ui, document_id, fit_requested);
        if self.model.documents.contains_key(&document_id) {
            self.tab_renderers.insert(document_id, renderer);
        }
    }

    fn render_editor_workspace(&mut self, ui: &mut egui::Ui, fit_requested: bool) {
        if self.editor_split.is_split {
            if self.editor_split.secondary_active_tab.is_none() {
                self.editor_split.secondary_active_tab = self.model.active_document_id();
            }
            if self.editor_split.split_vertical {
                ui.columns(2, |cols| {
                    self.render_document_tabs(&mut cols[0]);
                    self.render_active_document(&mut cols[0], fit_requested);

                    self.render_secondary_document_tabs(&mut cols[1]);
                    if let Some(id) = self.editor_split.secondary_active_tab {
                        self.render_document_by_id(&mut cols[1], id, fit_requested);
                    } else {
                        cols[1].centered_and_justified(|ui| ui.label("No document open"));
                    }
                });
            } else {
                ui.vertical(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), ui.available_height() * 0.5),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            self.render_document_tabs(ui);
                            self.render_active_document(ui, fit_requested);
                        },
                    );
                    ui.separator();
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            self.render_secondary_document_tabs(ui);
                            if let Some(id) = self.editor_split.secondary_active_tab {
                                self.render_document_by_id(ui, id, fit_requested);
                            } else {
                                ui.centered_and_justified(|ui| ui.label("No document open"));
                            }
                        },
                    );
                });
            }
        } else {
            self.render_document_tabs(ui);
            self.render_active_document(ui, fit_requested);
        }
    }

    fn render_status_bar(&mut self, ctx: &egui::Context) {
        if !self.panel_visibility.show_status_bar {
            return;
        }
        egui::TopBottomPanel::bottom("status_bar_v2")
            .exact_height(24.0)
            .frame(egui::Frame::new().fill(self.theme.statusbar_bg))
            .show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);
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
                    ui.label(RichText::new(active_path).small());
                    ui.separator();
                    ui.label(board_info);
                    ui.separator();
                    ui.label(format!("Selection: {selection}"));
                });
            });
        self.last_status_bar_height = 24.0;
    }

    fn write_layout_probe(&self) {
        let payload = serde_json::json!({
            "bottom_panel_visible": self.panel_visibility.show_bottom_panel,
            "bottom_panel_height": self.last_bottom_panel_height,
            "status_bar_visible": self.panel_visibility.show_status_bar,
            "status_bar_height": self.last_status_bar_height,
            "central_height": self.last_central_height,
            "split_enabled": self.editor_split.is_split,
            "split_vertical": self.editor_split.split_vertical,
            "last_drag_start_y": self.last_drag_start_y,
            "last_drag_end_y": self.last_drag_end_y,
        });
        let _ = std::fs::write(LAYOUT_PROBE_PATH, payload.to_string());
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

struct EditorTreeBehavior {
    app: *mut ShellApp,
    fit_requested: bool,
}

impl EditorTreeBehavior {
    fn app_mut(&mut self) -> &mut ShellApp {
        // SAFETY: used synchronously during one `Tree::ui` call on the UI thread.
        unsafe { &mut *self.app }
    }
}

impl Behavior<EditorPane> for EditorTreeBehavior {
    fn tab_title_for_pane(&mut self, pane: &EditorPane) -> egui::WidgetText {
        match pane {
            EditorPane::Workbench => "Editor".into(),
            EditorPane::BottomPanel => "Panel".into(),
        }
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: TileId,
        pane: &mut EditorPane,
    ) -> UiResponse {
        match pane {
            EditorPane::Workbench => {
                let fit_requested = self.fit_requested;
                self.app_mut().render_editor_workspace(ui, fit_requested);
            }
            EditorPane::BottomPanel => {
                self.app_mut().render_bottom_panel_contents(ui);
                self.app_mut().last_bottom_panel_height = ui.max_rect().height();
            }
        }
        UiResponse::None
    }
}

impl efame::App for ShellApp {
    fn save(&mut self, storage: &mut dyn efame::Storage) {
        efame::set_value(storage, STORAGE_LAYOUT_KEY, &self.layout);
        efame::set_value(storage, STORAGE_PANELS_KEY, &self.panel_visibility);
        efame::set_value(storage, STORAGE_CHROME_KEY, &self.panel_visibility);
        efame::set_value(storage, STORAGE_EDITOR_SPLIT_KEY, &self.editor_split);

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
        apply_theme(ctx, &self.theme);
        self.process_ipc();
        self.apply_ui_test_ops(ctx);
        self.capture_shortcut_if_needed(ctx);
        self.handle_shortcuts(ctx);
        self.process_queue(ctx, frame);
        self.prune_tab_renderers();

        self.render_title_menu_bar(ctx);
        // Reserve the bottom strip across the full viewport before sidebars are laid out.
        // This makes the status bar span under the sidebar/activity bar like VSCode.
        self.render_status_bar(ctx);
        self.render_activity_bar(ctx);
        self.render_sidebar(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(self.theme.editor_bg))
            .show(ctx, |ui| {
            self.last_central_height = ui.max_rect().height();
            let fit_requested = self.layout.request_fit;
            self.layout.request_fit = false;
            if self.panel_visibility.show_bottom_panel {
                let app_ptr: *mut ShellApp = self;
                let tree = &mut self.layout.editor_tree;
                let mut behavior = EditorTreeBehavior {
                    app: app_ptr,
                    fit_requested,
                };
                tree.ui(&mut behavior, ui);
            } else {
                self.last_bottom_panel_height = 0.0;
                self.render_editor_workspace(ui, fit_requested);
            }
        });

        self.show_palette_window(ctx);
        self.handle_screenshot_flow(ctx);
        self.write_layout_probe();

        if ctx.input(|i| i.raw.events.iter().any(|e| matches!(e, Event::Key { key: Key::Escape, pressed: true, .. })))
            && self.show_command_palette
        {
            self.show_command_palette = false;
        }

        // Keep a light polling cadence so IPC commands are handled even when the UI is idle.
        if self.ipc_rx.is_some()
            || self.screenshot_requested
            || self.screenshot_path.is_some()
            || self.active_drag_script.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::clamp_bottom_panel_height;

    #[test]
    fn bottom_panel_drag_up_increases_height() {
        let h = clamp_bottom_panel_height(180.0, -40.0, 900.0);
        assert!(h > 180.0);
    }

    #[test]
    fn bottom_panel_drag_down_decreases_height() {
        let h = clamp_bottom_panel_height(220.0, 40.0, 900.0);
        assert!(h < 220.0);
    }

    #[test]
    fn bottom_panel_respects_min_height() {
        let h = clamp_bottom_panel_height(100.0, 400.0, 900.0);
        assert_eq!(h, 80.0);
    }

    #[test]
    fn bottom_panel_respects_max_height() {
        let max = (900.0_f32 - 120.0).max(80.0);
        let h = clamp_bottom_panel_height(300.0, -1000.0, 900.0);
        assert_eq!(h, max);
    }
}

fn save_screenshot_png(image: &ColorImage, path: &Path) -> anyhow::Result<()> {
    let [width, height] = image.size;
    let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
    image::save_buffer(
        path,
        &rgba,
        width as u32,
        height as u32,
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
