mod tabs;
mod ui;

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::Duration;

use altium_format_spec::SpecDomain;
use efame::egui::{self, ColorImage, Event, Key, RichText, UserData, ViewportCommand};
use egui_tiles::{Behavior, TileId, UiResponse};

use self::tabs::{TabProviderRegistry, TabRenderer};
use crate::canvas::{Pcb2dCanvas, Pcb3dCanvas, PcbCanvasView};
use crate::commands::{
    CommandRegistry, ShortcutDef, StoredShortcut, build_context, selection_label,
    shortcut_from_stored, shortcut_to_stored,
};
use crate::ipc::{IpcRequest, UiTestOp};
use crate::jobs::{JobArtifact, JobEvent, JobKind, JobManager, JobPayload, JobRequest, JobTrigger};
use crate::layout::{BottomTab, EditorPane, ShellLayoutState};
use crate::pipeline::{
    ActivityViewIntent, Command, CommandTransaction, Effect, HistoryIntent, Intent, ResolveContext,
    ResolveResult, SecondarySidebarTabIntent, TelemetrySink, TracingTelemetry,
    intent_from_command_id, resolve_intent,
};
use crate::project_graph::{ParseState, WorkspaceModel};
use crate::ui::chrome::{show_central_panel, show_top_bar};
use crate::ui::section::empty_state;
use crate::ui::segmented::{SegmentItem, segmented_bar};
use crate::ui::status_bar::{StatusItem, show_status_bar};
use crate::ui::tabstrip::{TabAction, render_tabstrip};
use crate::ui::theme::{
    ThemeId, ThemePrefs, ThemeTokens, apply_theme, next_theme, previous_theme, theme_tokens_by_id,
};
use crate::workbench::{BoardViewMode, DocumentId, DocumentKind, WorkbenchModel};

const STORAGE_LAYOUT_KEY: &str = "shell.layout.v1";
const STORAGE_PANELS_KEY: &str = "shell.panels.v1";
const STORAGE_CHROME_KEY: &str = "shell.chrome.v2";
const STORAGE_SHORTCUTS_KEY: &str = "shell.shortcuts.v1";
const STORAGE_EDITOR_SPLIT_KEY: &str = "shell.editor_split.v1";
const STORAGE_THEME_KEY: &str = "shell.theme.v1";
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

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
enum SecondarySidebarTab {
    #[default]
    Inspector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaletteMode {
    Command,
    Theme,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct PanelVisibilityState {
    show_activity_bar: bool,
    show_primary_sidebar: bool,
    show_secondary_sidebar: bool,
    secondary_sidebar_width: f32,
    secondary_sidebar_tab: SecondarySidebarTab,
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
            show_secondary_sidebar: true,
            secondary_sidebar_width: 300.0,
            secondary_sidebar_tab: SecondarySidebarTab::Inspector,
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
    intent_queue: VecDeque<Intent>,
    show_command_palette: bool,
    palette_mode: PaletteMode,
    palette_focus_pending: bool,
    palette_filter: String,
    palette_selected: usize,
    explorer_filter: String,
    keybindings_filter: String,
    keybindings_capture_for: Option<String>,
    shortcut_bindings: BTreeMap<String, ShortcutDef>,
    ipc_rx: Option<Receiver<IpcRequest>>,
    jobs: JobManager,
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
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    telemetry: TracingTelemetry,
    theme_prefs: ThemePrefs,
    theme_preview: Option<ThemeId>,
}

#[derive(Debug, Clone)]
struct UndoEntry {
    forward: CommandTransaction,
    inverse: CommandTransaction,
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
        let mut theme_prefs = ThemePrefs::default();
        if let Some(storage) = cc.storage {
            if let Some(saved) = efame::get_value(storage, STORAGE_EDITOR_SPLIT_KEY) {
                editor_split = saved;
            }
            if let Some(saved) = efame::get_value(storage, STORAGE_THEME_KEY) {
                theme_prefs = saved;
            }
        }

        let commands = CommandRegistry::new_m1();
        let mut shortcut_bindings = default_shortcuts(&commands);

        if let Some(storage) = cc.storage {
            if let Some(saved) =
                efame::get_value::<ShortcutOverrides>(storage, STORAGE_SHORTCUTS_KEY)
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
            intent_queue: VecDeque::new(),
            show_command_palette: false,
            palette_mode: PaletteMode::Command,
            palette_focus_pending: false,
            palette_filter: String::new(),
            palette_selected: 0,
            explorer_filter: String::new(),
            keybindings_filter: String::new(),
            keybindings_capture_for: None,
            shortcut_bindings,
            ipc_rx,
            jobs: JobManager::new(),
            screenshot_path: None,
            screenshot_requested: false,
            pending_ui_test_ops: VecDeque::new(),
            active_drag_script: None,
            tab_registry: TabProviderRegistry::new_m1(),
            tab_renderers: BTreeMap::new(),
            theme: {
                let mut t = theme_tokens_by_id(theme_prefs.active_theme);
                t.font_scale = theme_prefs.ui_scale;
                t
            },
            editor_split,
            last_bottom_panel_height: 0.0,
            last_status_bar_height: 24.0,
            last_central_height: 0.0,
            last_drag_start_y: 0.0,
            last_drag_end_y: 0.0,
            canvas2d: Pcb2dCanvas::default(),
            canvas3d: Pcb3dCanvas,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            telemetry: TracingTelemetry,
            theme_prefs,
            theme_preview: None,
        }
    }

    pub(crate) fn queue_intent(&mut self, intent: Intent) {
        self.intent_queue.push_back(intent);
    }

    pub(crate) fn queue_command_id(&mut self, id: &str, arg: Option<String>) {
        match intent_from_command_id(id, arg) {
            Ok(intent) => self.queue_intent(intent),
            Err(err) => self
                .model
                .problems
                .push(format!("Invalid command request for '{id}': {err:?}")),
        }
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

    fn resolve_context(&self) -> ResolveContext {
        ResolveContext {
            workspace_open: self.model.has_workspace(),
            selection_exists: self.model.selection_exists(),
            show_primary_sidebar: self.panel_visibility.show_primary_sidebar,
            show_secondary_sidebar: self.panel_visibility.show_secondary_sidebar,
            show_bottom_panel: self.panel_visibility.show_bottom_panel,
            show_activity_bar: self.panel_visibility.show_activity_bar,
            show_status_bar: self.panel_visibility.show_status_bar,
        }
    }

    fn effective_theme_id(&self) -> ThemeId {
        self.theme_preview.unwrap_or(self.theme_prefs.active_theme)
    }

    fn refresh_theme_tokens(&mut self) {
        let mut tokens = theme_tokens_by_id(self.effective_theme_id());
        tokens.font_scale = self.theme_prefs.ui_scale;
        self.theme = tokens;
    }

    fn apply_effect(&mut self, effect: Effect, ctx: &egui::Context) {
        match effect {
            Effect::RequestQuit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn apply_transaction(
        &mut self,
        tx: CommandTransaction,
        ctx: &egui::Context,
        allow_history_push: bool,
    ) {
        let mut inverse = Vec::new();
        for command in &tx.commands {
            self.telemetry.command_executed(command);
            let (inv, effects) = self.apply_command(command.clone());
            if let Some(inv) = inv {
                inverse.push(inv);
            }
            for effect in effects {
                self.apply_effect(effect, ctx);
            }
        }

        if allow_history_push && !inverse.is_empty() {
            inverse.reverse();
            let inverse_tx = CommandTransaction {
                source_intent: tx.source_intent.clone(),
                commands: inverse,
            };
            self.telemetry.undo_pushed(inverse_tx.commands.len());
            self.undo_stack.push(UndoEntry {
                forward: tx,
                inverse: inverse_tx,
            });
            self.redo_stack.clear();
        }
    }

    fn execute_history_intent(&mut self, history: HistoryIntent, ctx: &egui::Context) {
        match history {
            HistoryIntent::Undo => {
                let Some(entry) = self.undo_stack.pop() else {
                    self.model.output_lines.push("Nothing to undo".to_owned());
                    return;
                };
                let inverse_tx = entry.inverse.clone();
                self.apply_transaction(inverse_tx, ctx, false);
                self.redo_stack.push(entry);
            }
            HistoryIntent::Redo => {
                let Some(entry) = self.redo_stack.pop() else {
                    self.model.output_lines.push("Nothing to redo".to_owned());
                    return;
                };
                let forward_tx = entry.forward.clone();
                self.apply_transaction(forward_tx, ctx, false);
                self.undo_stack.push(entry);
            }
        }
    }

    fn process_intent(&mut self, intent: Intent, ctx: &egui::Context) {
        self.telemetry.intent_received(&intent);

        if let Intent::History(history) = &intent {
            self.execute_history_intent(history.clone(), ctx);
            return;
        }

        let original_intent = intent.clone();
        match resolve_intent(intent, self.resolve_context()) {
            ResolveResult::Accepted { transaction } => {
                self.telemetry.commands_resolved(&transaction);
                self.apply_transaction(transaction, ctx, true);
            }
            ResolveResult::Rejected { code, message } => {
                self.telemetry
                    .intent_rejected(&original_intent, &code, &message);
                self.model.problems.push(message);
            }
        }
    }

    fn apply_command(&mut self, command: Command) -> (Option<Command>, Vec<Effect>) {
        match command {
            Command::OpenKeybindings => {
                self.model.open_or_activate_keybindings_document();
                (None, Vec::new())
            }
            Command::ThemeOpenManagerTab => {
                self.palette_mode = PaletteMode::Theme;
                self.show_command_palette = true;
                self.palette_focus_pending = true;
                self.palette_filter.clear();
                self.palette_selected = 0;
                (None, Vec::new())
            }
            Command::SetCommandPaletteVisible(value) => {
                let prev = self.show_command_palette;
                self.show_command_palette = value;
                if value {
                    self.palette_mode = PaletteMode::Command;
                    self.palette_filter.clear();
                    self.palette_selected = 0;
                }
                if value && !prev {
                    self.palette_focus_pending = true;
                }
                if !value {
                    self.palette_focus_pending = false;
                    self.theme_preview = None;
                    self.refresh_theme_tokens();
                }
                (Some(Command::SetCommandPaletteVisible(prev)), Vec::new())
            }
            Command::SetPrimarySidebarVisible(value) => {
                let prev = self.panel_visibility.show_primary_sidebar;
                self.panel_visibility.show_primary_sidebar = value;
                (Some(Command::SetPrimarySidebarVisible(prev)), Vec::new())
            }
            Command::SetSecondarySidebarVisible(value) => {
                let prev = self.panel_visibility.show_secondary_sidebar;
                self.panel_visibility.show_secondary_sidebar = value;
                (Some(Command::SetSecondarySidebarVisible(prev)), Vec::new())
            }
            Command::SetSecondarySidebarTab(tab) => {
                let prev = self.panel_visibility.secondary_sidebar_tab;
                self.panel_visibility.secondary_sidebar_tab = match tab {
                    SecondarySidebarTabIntent::Inspector => SecondarySidebarTab::Inspector,
                };
                (
                    Some(Command::SetSecondarySidebarTab(match prev {
                        SecondarySidebarTab::Inspector => SecondarySidebarTabIntent::Inspector,
                    })),
                    Vec::new(),
                )
            }
            Command::SetActivityView(view) => {
                let prev = self.panel_visibility.activity_view;
                self.panel_visibility.activity_view = match view {
                    ActivityViewIntent::Explorer => ActivityView::Explorer,
                    ActivityViewIntent::Search => ActivityView::Search,
                    ActivityViewIntent::SourceControl => ActivityView::SourceControl,
                    ActivityViewIntent::Run => ActivityView::Run,
                    ActivityViewIntent::Extensions => ActivityView::Extensions,
                };
                let inv = match prev {
                    ActivityView::Explorer => ActivityViewIntent::Explorer,
                    ActivityView::Search => ActivityViewIntent::Search,
                    ActivityView::SourceControl => ActivityViewIntent::SourceControl,
                    ActivityView::Run => ActivityViewIntent::Run,
                    ActivityView::Extensions => ActivityViewIntent::Extensions,
                };
                (Some(Command::SetActivityView(inv)), Vec::new())
            }
            Command::SetBottomPanelVisible(value) => {
                let prev = self.panel_visibility.show_bottom_panel;
                self.panel_visibility.show_bottom_panel = value;
                (Some(Command::SetBottomPanelVisible(prev)), Vec::new())
            }
            Command::SetBottomTab(tab) => {
                let prev = self.panel_visibility.bottom_tab;
                self.panel_visibility.bottom_tab = tab;
                (Some(Command::SetBottomTab(prev)), Vec::new())
            }
            Command::SetActivityBarVisible(value) => {
                let prev = self.panel_visibility.show_activity_bar;
                self.panel_visibility.show_activity_bar = value;
                (Some(Command::SetActivityBarVisible(prev)), Vec::new())
            }
            Command::SetStatusBarVisible(value) => {
                let prev = self.panel_visibility.show_status_bar;
                self.panel_visibility.show_status_bar = value;
                (Some(Command::SetStatusBarVisible(prev)), Vec::new())
            }
            Command::ActivateNextEditorTab => {
                let prev = self.model.active_document_id();
                self.model.activate_next_tab();
                (
                    prev.map(|id| Command::EditorActivateDocument { id }),
                    Vec::new(),
                )
            }
            Command::ActivatePreviousEditorTab => {
                let prev = self.model.active_document_id();
                self.model.activate_previous_tab();
                (
                    prev.map(|id| Command::EditorActivateDocument { id }),
                    Vec::new(),
                )
            }
            Command::SetEditorSplitRight => {
                self.editor_split.is_split = true;
                self.editor_split.split_vertical = true;
                if self.editor_split.secondary_active_tab.is_none() {
                    self.editor_split.secondary_active_tab = self.model.active_document_id();
                }
                self.model
                    .output_lines
                    .push("Split editor: right".to_owned());
                (None, Vec::new())
            }
            Command::SetEditorSplitDown => {
                self.editor_split.is_split = true;
                self.editor_split.split_vertical = false;
                if self.editor_split.secondary_active_tab.is_none() {
                    self.editor_split.secondary_active_tab = self.model.active_document_id();
                }
                self.model
                    .output_lines
                    .push("Split editor: down".to_owned());
                (None, Vec::new())
            }
            Command::ResetLayout => {
                self.layout = ShellLayoutState::default();
                self.editor_split = EditorSplitState::default();
                (None, Vec::new())
            }
            Command::EditorReopenClosed => {
                let _ = self.model.reopen_last_closed_document();
                (None, Vec::new())
            }
            Command::EditorActivateDocument { id } => {
                let prev = self.model.active_document_id();
                self.model.set_active_tab(id);
                (
                    prev.map(|id| Command::EditorActivateDocument { id }),
                    Vec::new(),
                )
            }
            Command::EditorCloseDocument { id } => {
                let _ = self.model.close_document(id);
                if self.editor_split.secondary_active_tab == Some(id) {
                    self.editor_split.secondary_active_tab = self.model.active_editor_tab;
                }
                (None, Vec::new())
            }
            Command::FileClose => {
                let _ = self.model.close_active_document();
                (None, Vec::new())
            }
            Command::FileCloseAll => {
                while self.model.close_active_document() {}
                (None, Vec::new())
            }
            Command::FileCloseOthers => {
                self.model.close_other_documents();
                (None, Vec::new())
            }
            Command::WorkspaceOpen { root } => {
                let root = root
                    .or_else(|| self.model.workspace_root.clone())
                    .or_else(|| std::env::current_dir().ok());
                let Some(root) = root else {
                    self.model
                        .problems
                        .push("Unable to resolve workspace root".to_owned());
                    return (None, Vec::new());
                };
                self.model.set_workspace_root(root.clone());
                self.model
                    .output_lines
                    .push(format!("Workspace opened: {}", root.display()));
                (None, Vec::new())
            }
            Command::WorkspaceOpenProject { path } => {
                let path = path.or_else(|| self.find_project_in_workspace_root());
                let Some(prjpcb) = path else {
                    self.model.problems.push(
                        "workspace.open_project requires a .PrjPcb path (or one in workspace root)"
                            .to_owned(),
                    );
                    return (None, Vec::new());
                };
                self.submit_job(JobPayload::ParseProject {
                    prjpcb_path: prjpcb.clone(),
                });
                self.model
                    .output_lines
                    .push(format!("Queued project parse: {}", prjpcb.display()));
                (None, Vec::new())
            }
            Command::WorkspaceReloadProject => {
                let prjpcb = self
                    .model
                    .active_workspace
                    .as_ref()
                    .map(|w| w.project.prjpcb_path.clone());
                if let Some(prjpcb_path) = prjpcb {
                    self.submit_job(JobPayload::ParseProject { prjpcb_path });
                } else {
                    self.model
                        .problems
                        .push("No active project workspace to reload".to_owned());
                }
                (None, Vec::new())
            }
            Command::WorkspaceSyncIr => {
                self.queue_project_sync_jobs();
                (None, Vec::new())
            }
            Command::WorkspaceClose => {
                self.model.clear_workspace();
                self.tab_renderers.clear();
                self.model.output_lines.push("Workspace closed".to_owned());
                (None, Vec::new())
            }
            Command::FileNewSpec => {
                self.model
                    .open_spec_document(None, "// New spec document\n".to_owned());
                (None, Vec::new())
            }
            Command::FileOpen { path } => {
                if let Some(path) = path {
                    self.open_document_path(path);
                } else {
                    self.model.output_lines.push(
                        "Use Explorer or pass a path to File: Open from command palette".to_owned(),
                    );
                }
                (None, Vec::new())
            }
            Command::FileSave => {
                self.save_active_document();
                (None, Vec::new())
            }
            Command::FileSaveAll => {
                self.save_all_documents();
                (None, Vec::new())
            }
            Command::FileRevert => {
                self.revert_active_document();
                (None, Vec::new())
            }
            Command::SpecPlan => {
                self.submit_active_spec_job(true);
                (None, Vec::new())
            }
            Command::SpecApply => {
                self.submit_active_spec_job(false);
                (None, Vec::new())
            }
            Command::JobsCancelActive => {
                if let Some(id) = self.jobs.cancel_first_active() {
                    self.model
                        .output_lines
                        .push(format!("Requested cancellation for job {}", id.0));
                } else {
                    self.model.output_lines.push("No active jobs".to_owned());
                }
                (None, Vec::new())
            }
            Command::PcbSetViewMode(mode) => {
                let prev = self.model.active_board().map(|b| b.view_mode);
                if let Some(board) = self.model.active_board_mut() {
                    board.view_mode = mode;
                }
                (prev.map(Command::PcbSetViewMode), Vec::new())
            }
            Command::PcbZoomFit => {
                self.layout.request_fit = true;
                (None, Vec::new())
            }
            Command::SetSelection(selection) => {
                let prev = self.model.selection.primary.clone();
                self.model.selection.primary = selection;
                (Some(Command::SetSelection(prev)), Vec::new())
            }
            Command::RunStartLast => {
                self.model
                    .output_lines
                    .push("No runnable task configured yet.".to_owned());
                (None, Vec::new())
            }
            Command::HelpAbout => {
                self.model
                    .output_lines
                    .push("AutoPCB Shell - IDE shell for PCB/spec automation".to_owned());
                (None, Vec::new())
            }
            Command::ThemeCycleNext => {
                let prev = self.theme_prefs.active_theme;
                self.theme_prefs.active_theme = next_theme(prev);
                self.theme_preview = None;
                self.refresh_theme_tokens();
                (Some(Command::ThemeSetActive { id: prev }), Vec::new())
            }
            Command::ThemeCyclePrevious => {
                let prev = self.theme_prefs.active_theme;
                self.theme_prefs.active_theme = previous_theme(prev);
                self.theme_preview = None;
                self.refresh_theme_tokens();
                (Some(Command::ThemeSetActive { id: prev }), Vec::new())
            }
            Command::ThemeSetActive { id } => {
                let prev = self.theme_prefs.active_theme;
                self.theme_prefs.active_theme = id;
                self.theme_preview = None;
                self.refresh_theme_tokens();
                (Some(Command::ThemeSetActive { id: prev }), Vec::new())
            }
            Command::ThemeSetUiScale { scale } => {
                let prev = self.theme_prefs.ui_scale;
                self.theme_prefs.ui_scale = scale.clamp(0.8, 1.75);
                self.refresh_theme_tokens();
                (Some(Command::ThemeSetUiScale { scale: prev }), Vec::new())
            }
            Command::EmitEffect(effect) => (None, vec![effect]),
        }
    }

    fn submit_job(&mut self, payload: JobPayload) {
        let kind = match &payload {
            JobPayload::ParseProject { .. } => JobKind::ParseProject,
            JobPayload::SyncBoardIr { .. } => JobKind::SyncBoardIr,
            JobPayload::SyncSchematicIr { .. } => JobKind::SyncSchematicIr,
            JobPayload::SpecPlan { .. } => JobKind::SpecPlan,
            JobPayload::SpecApply { .. } => JobKind::SpecApply,
        };
        let req = JobRequest {
            id: self.jobs.allocate_id(),
            kind,
            workspace_id: self
                .model
                .active_workspace
                .as_ref()
                .map(|w| w.id)
                .unwrap_or(0),
            doc_targets: Vec::new(),
            payload,
            requested_by: JobTrigger::Command,
        };
        let _ = self.jobs.submit(req);
    }

    fn find_project_in_workspace_root(&self) -> Option<PathBuf> {
        let root = self.model.workspace_root.as_ref()?;
        let entries = fs::read_dir(root).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let is_prjpcb = path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("prjpcb"));
            if is_prjpcb {
                return Some(path);
            }
        }
        None
    }

    fn queue_project_sync_jobs(&mut self) {
        let Some(workspace) = self.model.active_workspace.as_ref() else {
            self.model
                .problems
                .push("No active project workspace to sync".to_owned());
            return;
        };
        let board_paths: Vec<PathBuf> = workspace
            .project
            .board_docs
            .iter()
            .map(|b| b.path.clone())
            .collect();
        let sch_paths: Vec<PathBuf> = workspace
            .project
            .schematic_docs
            .iter()
            .map(|s| s.path.clone())
            .collect();

        for path in board_paths {
            self.submit_job(JobPayload::SyncBoardIr { pcbdoc_path: path });
        }
        for path in sch_paths {
            self.submit_job(JobPayload::SyncSchematicIr { schdoc_path: path });
        }
    }

    fn submit_active_spec_job(&mut self, dry_run: bool) {
        let Some(doc) = self.model.active_document() else {
            self.model.problems.push("No active document".to_owned());
            return;
        };
        let DocumentKind::Spec(spec) = &doc.kind else {
            self.model
                .problems
                .push("Active document is not a spec".to_owned());
            return;
        };
        let Some(spec_path) = spec.path.clone() else {
            self.model
                .problems
                .push("Save spec before running plan/apply".to_owned());
            return;
        };
        let Some((domain, target)) = self.resolve_spec_target(&spec_path) else {
            self.model.problems.push(format!(
                "Unable to resolve target for spec {}",
                spec_path.display()
            ));
            return;
        };
        let payload = if dry_run {
            JobPayload::SpecPlan {
                spec_path,
                target_path: target,
                domain,
            }
        } else {
            JobPayload::SpecApply {
                spec_path,
                target_path: target,
                domain,
                dry_run: false,
            }
        };
        self.submit_job(payload);
    }

    fn resolve_spec_target(&self, spec_path: &Path) -> Option<(SpecDomain, PathBuf)> {
        let file = spec_path
            .file_name()?
            .to_string_lossy()
            .to_ascii_lowercase();
        let workspace = self.model.active_workspace.as_ref();
        if file.ends_with(".pcbdoc-spec") {
            let target = workspace?.project.board_docs.first()?.path.clone();
            return Some((SpecDomain::PcbDoc, target));
        }
        if file.ends_with(".schdoc-spec") {
            let target = workspace?.project.schematic_docs.first()?.path.clone();
            return Some((SpecDomain::SchDoc, target));
        }
        if file.ends_with(".prjpcb-spec") {
            let target = workspace?.project.prjpcb_path.clone();
            return Some((SpecDomain::PrjPcb, target));
        }
        None
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
            "prjpcb" => {
                self.queue_intent(Intent::Workspace(
                    crate::pipeline::WorkspaceIntent::OpenProject { path: Some(path) },
                ));
            }
            "spec" | "pcbdoc-spec" | "schdoc-spec" | "prjpcb-spec" => {
                match fs::read_to_string(&path) {
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
                }
            }
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

    fn process_queue(&mut self, ctx: &egui::Context) {
        while let Some(intent) = self.intent_queue.pop_front() {
            self.process_intent(intent, ctx);
        }
    }

    fn process_job_events(&mut self) {
        for ev in self.jobs.poll_events() {
            match ev {
                JobEvent::Queued(id, kind) => {
                    self.model
                        .jobs
                        .push(format!("queued #{}: {:?}", id.0, kind));
                }
                JobEvent::Started(id) => {
                    self.model.jobs.push(format!("started #{}", id.0));
                }
                JobEvent::Progress(id, p) => {
                    let pct = p
                        .percent
                        .map(|v| format!(" {:.0}%", v * 100.0))
                        .unwrap_or_default();
                    self.model.jobs.push(format!(
                        "progress #{} [{}{}] {}",
                        id.0, p.stage, pct, p.message
                    ));
                }
                JobEvent::Artifact(id, artifact) => match artifact {
                    JobArtifact::ProjectGraphDelta(delta) => {
                        let root = delta
                            .graph
                            .prjpcb_path
                            .parent()
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| PathBuf::from("."));
                        let workspace = WorkspaceModel {
                            id: 1,
                            root,
                            project: delta.graph,
                            opened_at: std::time::SystemTime::now(),
                            last_sync: None,
                        };
                        self.model.set_active_workspace(workspace);
                        self.model
                            .output_lines
                            .push(format!("Loaded project graph from job #{}", id.0));
                        self.queue_project_sync_jobs();
                    }
                    JobArtifact::BoardIr { path, ir } => {
                        let _ = self.model.open_board_document(path.clone(), ir);
                        if let Some(ws) = self.model.active_workspace.as_mut() {
                            for board in &mut ws.project.board_docs {
                                if board.path == path {
                                    board.parse_state = ParseState::Fresh;
                                    board.ir_state = ParseState::Fresh;
                                }
                            }
                            ws.last_sync = Some(std::time::SystemTime::now());
                        }
                    }
                    JobArtifact::SchematicIndex(index) => {
                        if let Some(ws) = self.model.active_workspace.as_mut() {
                            for sch in &mut ws.project.schematic_docs {
                                if sch.path == index.path {
                                    sch.parse_state = ParseState::Fresh;
                                    sch.index_state = ParseState::Fresh;
                                }
                            }
                            ws.last_sync = Some(std::time::SystemTime::now());
                        }
                        self.model.output_lines.push(format!(
                            "Schematic indexed: {} (components={}, net_labels={})",
                            index.path.display(),
                            index.component_count,
                            index.net_label_count
                        ));
                    }
                    JobArtifact::Eco(eco) => {
                        self.model.output_lines.push(format!(
                            "Plan generated: {} changes (job #{})",
                            eco.changes.len(),
                            id.0
                        ));
                    }
                    JobArtifact::Diagnostics(diags) => {
                        for d in diags {
                            self.model
                                .problems
                                .push(format!("[{}:{}] {}", d.severity, d.source, d.message));
                        }
                    }
                },
                JobEvent::Completed(id, summary) => {
                    self.model.jobs.push(format!(
                        "completed #{} in {}ms: {}",
                        id.0, summary.duration_ms, summary.message
                    ));
                }
                JobEvent::Failed(id, failure) => {
                    self.model.jobs.push(format!(
                        "failed #{} [{}]: {}",
                        id.0, failure.stage, failure.message
                    ));
                    self.model.problems.push(format!(
                        "Job #{} failed at {}: {}",
                        id.0, failure.stage, failure.message
                    ));
                }
                JobEvent::Cancelled(id) => {
                    self.model.jobs.push(format!("cancelled #{}", id.0));
                }
            }
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
                IpcRequest::Command { id, arg } => self.queue_command_id(&id, arg),
                IpcRequest::OpenFile { path } => self.queue_command_id("file.open", Some(path)),
                IpcRequest::OpenProject { prjpcb_path } => {
                    self.queue_command_id("workspace.open_project", Some(prjpcb_path));
                }
                IpcRequest::RunJob { kind, args } => {
                    let payload = match kind.as_str() {
                        "sync_ir" => self
                            .model
                            .active_workspace
                            .as_ref()
                            .map(|_| "workspace.sync_ir".to_owned()),
                        _ => None,
                    };
                    if let Some(cmd) = payload {
                        self.queue_command_id(&cmd, None);
                    } else {
                        self.model.problems.push(format!(
                            "Unsupported IPC job request: kind={} args={}",
                            kind, args
                        ));
                    }
                }
                IpcRequest::CancelJob { id } => {
                    let _ = self.jobs.cancel(crate::jobs::JobId(id));
                }
                IpcRequest::ListJobs => {
                    self.model
                        .output_lines
                        .push(format!("Active jobs: {}", self.jobs.active_jobs()));
                }
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
                        let splitter_y = screen.bottom()
                            - self.last_status_bar_height
                            - self.last_bottom_panel_height;
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
            self.model
                .output_lines
                .push("UI test drag completed".to_owned());
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
                self.model.problems.push(format!(
                    "Failed to save screenshot {}: {err}",
                    target.display()
                ));
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
            self.queue_command_id(&id, None);
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
        self.model.output_lines.push(format!(
            "Shortcut updated: {command_id} -> {}",
            candidate.display()
        ));
        self.keybindings_capture_for = None;
    }

    fn render_title_menu_bar(&mut self, ctx: &egui::Context) {
        let theme = self.theme.clone();
        let text_primary = theme.text_primary;
        show_top_bar(ctx, "title_menu", 28.0, &theme, |ui| {
            ui.visuals_mut().override_text_color = Some(text_primary);
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
            "Theme",
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
                        self.queue_command_id(cmd.id, None);
                        ui.close();
                    }
                }
            });
        }
    }

    fn render_document_tabs(&mut self, ui: &mut egui::Ui) {
        let actions = render_tabstrip(ui, &self.model, &self.theme, self.model.active_editor_tab);
        for action in actions {
            match action {
                TabAction::Activate(id) => {
                    self.queue_intent(Intent::Editor(
                        crate::pipeline::EditorIntent::ActivateDocument { id },
                    ));
                }
                TabAction::Close(id) => {
                    self.queue_intent(Intent::Editor(
                        crate::pipeline::EditorIntent::CloseDocument { id },
                    ));
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
                    self.queue_intent(Intent::Editor(
                        crate::pipeline::EditorIntent::CloseDocument { id },
                    ));
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

        let modes = [
            SegmentItem::new(BoardViewMode::TwoD, "2D"),
            SegmentItem::new(BoardViewMode::ThreeD, "3D"),
        ];
        if let Some(changed) = segmented_bar(ui, &self.theme, mode, &modes) {
            match changed {
                BoardViewMode::TwoD => {
                    self.queue_intent(Intent::Pcb(crate::pipeline::PcbIntent::SetView2d))
                }
                BoardViewMode::ThreeD => {
                    self.queue_intent(Intent::Pcb(crate::pipeline::PcbIntent::SetView3d))
                }
            }
        }
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
            ui.centered_and_justified(|ui| empty_state(ui, &self.theme, "No document open"));
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

    fn render_document_by_id(
        &mut self,
        ui: &mut egui::Ui,
        document_id: DocumentId,
        fit_requested: bool,
    ) {
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
                        cols[1].centered_and_justified(|ui| {
                            empty_state(ui, &self.theme, "No document open")
                        });
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
                                ui.centered_and_justified(|ui| {
                                    empty_state(ui, &self.theme, "No document open")
                                });
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
        let items = [
            StatusItem::normal(active_doc),
            StatusItem::small(active_path),
            StatusItem::normal(board_info),
            StatusItem::normal(format!("Selection: {selection}")),
        ];
        show_status_bar(ctx, "status_bar_v2", 24.0, &self.theme, &items);
        self.last_status_bar_height = 24.0;
    }

    fn write_layout_probe(&self) {
        let payload = serde_json::json!({
            "bottom_panel_visible": self.panel_visibility.show_bottom_panel,
            "bottom_panel_height": self.last_bottom_panel_height,
            "status_bar_visible": self.panel_visibility.show_status_bar,
            "status_bar_height": self.last_status_bar_height,
            "secondary_sidebar_visible": self.panel_visibility.show_secondary_sidebar,
            "secondary_sidebar_width": self.panel_visibility.secondary_sidebar_width,
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
        efame::set_value(storage, STORAGE_THEME_KEY, &self.theme_prefs);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut efame::Frame) {
        self.process_ipc();
        self.process_job_events();
        self.apply_ui_test_ops(ctx);
        self.capture_shortcut_if_needed(ctx);
        self.handle_shortcuts(ctx);
        self.process_queue(ctx);
        apply_theme(ctx, &self.theme);
        self.prune_tab_renderers();

        self.render_title_menu_bar(ctx);
        // Reserve the bottom strip across the full viewport before sidebars are laid out.
        // This makes the status bar span under the sidebar/activity bar like VSCode.
        self.render_status_bar(ctx);
        self.render_activity_bar(ctx);
        self.render_sidebar(ctx);
        self.render_secondary_sidebar(ctx);

        let theme = self.theme.clone();
        show_central_panel(ctx, &theme, |ui| {
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

        if ctx.input(|i| {
            i.raw.events.iter().any(|e| {
                matches!(
                    e,
                    Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        ..
                    }
                )
            })
        }) && self.show_command_palette
        {
            self.show_command_palette = false;
            self.palette_focus_pending = false;
            self.theme_preview = None;
            self.refresh_theme_tokens();
        }

        // Keep a light polling cadence so IPC commands are handled even when the UI is idle.
        if self.ipc_rx.is_some()
            || self.jobs.active_jobs() > 0
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
    use super::{PanelVisibilityState, SecondarySidebarTab, clamp_bottom_panel_height};

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

    #[test]
    fn secondary_sidebar_defaults_to_visible_inspector() {
        let panels = PanelVisibilityState::default();
        assert!(panels.show_secondary_sidebar);
        assert_eq!(panels.secondary_sidebar_tab, SecondarySidebarTab::Inspector);
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
