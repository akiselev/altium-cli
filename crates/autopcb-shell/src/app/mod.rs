mod tabs;
mod ui;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant, SystemTime};

use altium_format_render_png::{DEFAULT_SCALE, render_schlib_component_png};
use altium_format_spec::parser::parse_spec;
use altium_format_spec::{
    SpecDomain, SpecModel, apply_spec_schdoc, apply_spec_schlib, compile_spec,
};
use altium_format_types::coord::{Coord, CoordPoint};
use autopcb_graph::{GraphRead, GraphWorkspace, RenderAdapterHost};
use efame::egui::{self, ColorImage, Event, Key, RichText, UserData, ViewportCommand};
use egui_tiles::{Behavior, TileId, UiResponse};
use rfd::FileDialog;

use self::tabs::{TabProviderRegistry, TabRenderer};
use crate::agents::{
    AgentMessage, AgentRunStatus, AgentSession, AgentSessionId, AgentWorkspaceState,
    ProposalBundle, ProposalId, ProposalStatus,
};
use crate::canvas::{
    BoardCanvasAction, MovePreview, Pcb2dCanvas, Pcb3dCanvas, PcbCanvasView, SchDoc2dCanvas,
    SchDocCanvasAction, SchDocCanvasView, SchMovePreview, translate_component,
};
use crate::commands::{
    CommandRegistry, ShortcutDef, StoredShortcut, build_context, selection_label,
    shortcut_from_stored, shortcut_to_stored,
};
use crate::graph_host::GraphHost;
use crate::ipc::{IpcRequest, UiTestOp};
use crate::jobs::{JobArtifact, JobEvent, JobKind, JobManager, JobPayload, JobRequest, JobTrigger};
use crate::layout::{BottomTab, EditorPane, ShellLayoutState};
use crate::pipeline::{
    ActivityViewIntent, Command, CommandTransaction, Effect, HistoryIntent, Intent, ResolveContext,
    ResolveResult, SecondarySidebarTabIntent, TelemetrySink, ToolId, TracingTelemetry,
    TxUndoPolicy, intent_from_command_id, resolve_intent,
};
use crate::project_graph::{ParseState, WorkspaceModel};
use crate::session::{FileSessionStore, RestoreMode, SessionSnapshot, SessionTabRef};
use crate::session::{
    SessionDocumentState, SessionGraphDocumentKind, SessionPrefsState, SessionSelectionState,
    SessionStore, SessionTabState, SessionUiState, SessionWorkspaceState, now_unix_ms,
    shortcut_overrides_from_stored,
};
use crate::ui::chrome::{show_central_panel, show_top_bar};
use crate::ui::section::empty_state;
use crate::ui::segmented::{SegmentItem, segmented_bar};
use crate::ui::status_bar::{StatusItem, show_status_bar};
use crate::ui::tabstrip::{TabAction, render_tabstrip};
use crate::ui::theme::{
    ThemeId, ThemePrefs, ThemeTokens, apply_theme, next_theme, previous_theme, theme_tokens_by_id,
};
use crate::workbench::{BoardViewMode, DocumentId, DocumentKind, DocumentRevision, WorkbenchModel};

const LAYOUT_PROBE_PATH: &str = "/tmp/autopcb-shell-layout.json";
const SESSION_AUTOSAVE_DEBOUNCE_MS: u64 = 800;

#[cfg(test)]
fn clamp_bottom_panel_height(current: f32, drag_delta_y: f32, viewport_h: f32) -> f32 {
    let max_h = (viewport_h - 120.0).max(80.0);
    (current - drag_delta_y).clamp(80.0, max_h)
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub(crate) enum ActivityView {
    Explorer,
    Search,
    SourceControl,
    Run,
    Extensions,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub(crate) enum SecondarySidebarTab {
    #[default]
    Inspector,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub(crate) enum PaletteMode {
    Command,
    Theme,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct PanelVisibilityState {
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
pub(crate) struct EditorSplitState {
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
pub(crate) struct ShortcutOverrides {
    pub(crate) by_command: BTreeMap<String, StoredShortcut>,
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
    schdoc_canvas2d: SchDoc2dCanvas,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    agents: AgentWorkspaceState,
    preview_cache: PreviewTextureCache,
    telemetry: TracingTelemetry,
    theme_prefs: ThemePrefs,
    theme_preview: Option<ThemeId>,
    session_store: FileSessionStore,
    session_dirty: bool,
    last_session_save: Instant,
    watched_files: BTreeMap<PathBuf, SystemTime>,
    last_watch_scan: Instant,
    document_runtime: BTreeMap<DocumentId, DocumentRuntime>,
    pending_job_revisions: BTreeMap<crate::jobs::JobId, BTreeMap<DocumentId, DocumentRevision>>,
}

#[derive(Debug, Clone)]
struct UndoEntry {
    forward: CommandTransaction,
    inverse: CommandTransaction,
}

#[derive(Debug, Default, Clone)]
struct DocumentRuntime {
    active_tool: ToolId,
    active_interaction: Option<ActiveInteraction>,
    invalidation: DirtySets,
}

#[derive(Debug, Clone)]
enum ActiveInteraction {
    BoardMoveSelection {
        designator: String,
        delta_x_mm: f32,
        delta_y_mm: f32,
    },
    SchDocMoveSelection {
        designator: String,
        delta_x_mils: f32,
        delta_y_mils: f32,
    },
}

#[derive(Debug, Clone)]
enum DomainEvent {
    SelectionChanged,
    BoardViewModeChanged {
        document_id: DocumentId,
    },
    ComponentMoved {
        document_id: DocumentId,
        designator: String,
    },
}

#[derive(Debug, Default, Clone)]
struct DirtySets {
    render: BTreeSet<RenderDirty>,
    connectivity: BTreeSet<ConnectivityDirty>,
    drc: BTreeSet<DrcDirty>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum RenderDirty {
    SelectionOverlay,
    BoardViewMode,
    Component(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ConnectivityDirty {
    SelectionOverlay,
    Component(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum DrcDirty {
    BoardViewMode,
    Component(String),
}

#[derive(Debug, Default, Clone)]
struct InvalidationDelta {
    by_document: BTreeMap<DocumentId, DirtySets>,
}

impl InvalidationDelta {
    fn add_render_hint(&mut self, document_id: DocumentId, hint: RenderDirty) {
        self.by_document
            .entry(document_id)
            .or_default()
            .render
            .insert(hint);
    }

    fn add_connectivity_hint(&mut self, document_id: DocumentId, hint: ConnectivityDirty) {
        self.by_document
            .entry(document_id)
            .or_default()
            .connectivity
            .insert(hint);
    }

    fn add_drc_hint(&mut self, document_id: DocumentId, hint: DrcDirty) {
        self.by_document
            .entry(document_id)
            .or_default()
            .drc
            .insert(hint);
    }
}

#[derive(Default)]
struct PreviewTextureCache {
    by_key: BTreeMap<String, PreviewTextureEntry>,
}

struct PreviewTextureEntry {
    text_hash: u64,
    image_hash: u64,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
}

impl ShellApp {
    pub fn new(
        _cc: &efame::CreationContext<'_>,
        board_path: Option<PathBuf>,
        initial_ir: Option<autopcb_ir::PcbIr>,
        ipc_rx: Option<Receiver<IpcRequest>>,
        session_store: FileSessionStore,
        restore_mode: RestoreMode,
    ) -> Self {
        let mut layout = ShellLayoutState::default();
        let panel_visibility = PanelVisibilityState::default();
        layout.ensure_required_panes();
        let editor_split = EditorSplitState::default();
        let theme_prefs = ThemePrefs::default();

        let commands = CommandRegistry::new_m1();
        let shortcut_bindings = default_shortcuts(&commands);
        let mut app = Self {
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
            schdoc_canvas2d: SchDoc2dCanvas::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            agents: AgentWorkspaceState::default(),
            preview_cache: PreviewTextureCache::default(),
            telemetry: TracingTelemetry,
            theme_prefs,
            theme_preview: None,
            session_store,
            session_dirty: false,
            last_session_save: Instant::now(),
            watched_files: BTreeMap::new(),
            last_watch_scan: Instant::now(),
            document_runtime: BTreeMap::new(),
            pending_job_revisions: BTreeMap::new(),
        };
        app.refresh_theme_tokens();
        app.restore_mode(restore_mode);
        app
    }

    fn restore_mode(&mut self, restore_mode: RestoreMode) {
        let loaded = match restore_mode {
            RestoreMode::None => return,
            RestoreMode::Auto => self.session_store.load_latest(),
            RestoreMode::Path(path) => FileSessionStore::new(path).load_latest(),
        };
        match loaded {
            Ok(Some(snapshot)) => {
                if let Err(err) = self.apply_snapshot(snapshot) {
                    self.model
                        .problems
                        .push(format!("Failed to restore session: {err}"));
                } else {
                    self.model.output_lines.push(format!(
                        "Session restored: {}",
                        self.session_store.snapshot_path().display()
                    ));
                }
            }
            Ok(None) => {}
            Err(err) => self
                .model
                .problems
                .push(format!("Failed to load session snapshot: {err}")),
        }
    }

    fn session_tab_ref_for_document(
        &self,
        id: DocumentId,
        untitled_by_id: &BTreeMap<DocumentId, String>,
    ) -> Option<SessionTabRef> {
        let doc = self.model.documents.get(&id)?;
        let workspace = self
            .model
            .active_graph
            .as_ref()
            .map(|graph| graph.workspace_ref().clone());
        match &doc.kind {
            DocumentKind::Board(_) => doc.path.clone().map(|path| SessionTabRef::BoardPath { path }),
            DocumentKind::Spec(_) => {
                if let Some(path) = &doc.path {
                    Some(SessionTabRef::SpecTextPath { path: path.clone() })
                } else {
                    untitled_by_id
                        .get(&id)
                        .cloned()
                        .map(|untitled_id| SessionTabRef::UntitledSpec { untitled_id })
                }
            }
            DocumentKind::SchDocPreview(_)
            | DocumentKind::SchLibGallery(_)
            | DocumentKind::SchLibComponent(_) => None,
            DocumentKind::DesignOverview(graph) => workspace.map(|workspace| {
                SessionTabRef::DesignOverview {
                    workspace,
                    scope: graph.scope.clone(),
                }
            }),
            DocumentKind::Logical(graph) => workspace.map(|workspace| SessionTabRef::LogicalScope {
                workspace,
                scope: graph.scope.clone(),
            }),
            DocumentKind::Physical(graph) => workspace.map(|workspace| SessionTabRef::PhysicalScope {
                workspace,
                scope: graph.scope.clone(),
            }),
            DocumentKind::DefinitionCollection(graph) => {
                workspace.map(|workspace| SessionTabRef::DefinitionScope {
                    workspace,
                    scope: graph.scope.clone(),
                })
            }
            DocumentKind::Asset(graph) => workspace.map(|workspace| SessionTabRef::AssetScope {
                workspace,
                asset: graph.asset.clone(),
            }),
            DocumentKind::Import(graph) => workspace.map(|workspace| SessionTabRef::ImportScope {
                workspace,
                import: graph.import.clone(),
            }),
            DocumentKind::Keybindings => Some(SessionTabRef::Keybindings),
        }
    }

    fn create_agent_session(&mut self, title: Option<String>) -> AgentSessionId {
        let session_id = self.agents.allocate_session_id();
        let now = now_unix_ms();
        let session = AgentSession {
            id: session_id,
            title: title.unwrap_or_else(|| format!("Agent Session {}", session_id.0)),
            workspace_root: self.model.workspace_root.clone(),
            created_at_unix_ms: now,
            updated_at_unix_ms: now,
            status: AgentRunStatus::Idle,
            messages: Vec::new(),
            proposal_ids: Vec::new(),
            last_error: None,
        };
        self.agents.sessions.insert(session_id, session);
        self.agents.active_session = Some(session_id);
        self.telemetry.agent_session_started(session_id);
        session_id
    }

    fn ensure_active_agent_session(&mut self) -> AgentSessionId {
        self.agents
            .active_session
            .filter(|id| self.agents.sessions.contains_key(id))
            .unwrap_or_else(|| self.create_agent_session(None))
    }

    fn append_agent_message(&mut self, session_id: AgentSessionId, author: &str, body: String) {
        if let Some(session) = self.agents.sessions.get_mut(&session_id) {
            let now = now_unix_ms();
            session.updated_at_unix_ms = now;
            session.messages.push(AgentMessage {
                author: author.to_owned(),
                body,
                created_at_unix_ms: now,
            });
        }
    }

    fn current_move_proposal_request(&self, prompt: &str) -> Option<(String, f32, f32)> {
        let selection = match &self.model.selection.primary {
            crate::workbench::SelectionKind::Component(designator) => designator.clone(),
            _ => return None,
        };
        let prompt_lower = prompt.to_ascii_lowercase();
        if !["move", "shift", "reposition", "nudge"]
            .iter()
            .any(|token| prompt_lower.contains(token))
        {
            return None;
        }
        let magnitude = first_prompt_number(prompt).unwrap_or(1.0);
        let mut delta_x_mm = 0.0;
        let mut delta_y_mm = 0.0;
        if prompt_lower.contains("left") {
            delta_x_mm -= magnitude;
        }
        if prompt_lower.contains("right") {
            delta_x_mm += magnitude;
        }
        if prompt_lower.contains("up") {
            delta_y_mm += magnitude;
        }
        if prompt_lower.contains("down") {
            delta_y_mm -= magnitude;
        }
        if delta_x_mm == 0.0 && delta_y_mm == 0.0 {
            delta_x_mm = magnitude;
        }
        Some((selection, delta_x_mm, delta_y_mm))
    }

    fn create_move_proposal_from_prompt(
        &mut self,
        session_id: AgentSessionId,
        prompt: &str,
    ) -> Result<Option<ProposalId>, String> {
        let Some((designator, delta_x_mm, delta_y_mm)) = self.current_move_proposal_request(prompt)
        else {
            self.append_agent_message(
                session_id,
                "assistant",
                "No persistent proposal created. Select a component on a board and ask to move or shift it to generate a reviewable change."
                    .to_owned(),
            );
            return Ok(None);
        };

        let proposal_intent = Intent::Tool(crate::pipeline::ToolIntent::CommitMoveSelection {
            delta_x_mm,
            delta_y_mm,
        });
        let transaction = match resolve_intent(proposal_intent, self.resolve_context()) {
            ResolveResult::Accepted { transaction } => transaction,
            ResolveResult::Rejected { message, .. } => return Err(message),
        };

        let active_document = self.model.active_document_id();
        let mut expected_revisions = BTreeMap::new();
        let mut target_documents = Vec::new();
        if let Some(document_id) = active_document
            && let Some(revision) = self.model.document_revision(document_id)
        {
            expected_revisions.insert(document_id, revision);
            target_documents.push(document_id);
        }

        let proposal_id = self.agents.allocate_proposal_id();
        let title = format!("Move {designator}");
        let summary =
            format!("Move component {designator} by {delta_x_mm:+.2}mm X and {delta_y_mm:+.2}mm Y");
        let rationale = format!("Generated from agent prompt: {}", prompt.trim());
        let preview_lines = vec![
            summary.clone(),
            "Status: pending review".to_owned(),
            "Applying this proposal will run through the normal command transaction path."
                .to_owned(),
        ];
        self.agents.proposals.insert(
            proposal_id,
            ProposalBundle {
                id: proposal_id,
                session_id,
                title,
                summary: summary.clone(),
                rationale,
                created_at_unix_ms: now_unix_ms(),
                status: ProposalStatus::PendingReview,
                transaction,
                preview_lines,
                expected_revisions,
                target_documents,
            },
        );
        self.telemetry.proposal_created(proposal_id);
        self.agents.active_proposal = Some(proposal_id);
        if let Some(session) = self.agents.sessions.get_mut(&session_id) {
            session.proposal_ids.push(proposal_id);
            session.status = AgentRunStatus::Completed;
            session.updated_at_unix_ms = now_unix_ms();
        }
        self.append_agent_message(
            session_id,
            "assistant",
            format!(
                "Created proposal #{} for review: {}. Persistent changes are still blocked until you approve them.",
                proposal_id.0, summary
            ),
        );
        Ok(Some(proposal_id))
    }

    fn mark_proposal_rejected(&mut self, proposal_id: ProposalId) {
        let Some((session_id, title)) =
            self.agents.proposals.get_mut(&proposal_id).map(|proposal| {
                proposal.status = ProposalStatus::Rejected;
                (proposal.session_id, proposal.title.clone())
            })
        else {
            self.model
                .problems
                .push(format!("Proposal #{} not found", proposal_id.0));
            return;
        };
        self.telemetry.proposal_rejected(proposal_id);
        self.agents.active_proposal = Some(proposal_id);
        self.agents.active_session = Some(session_id);
        self.append_agent_message(
            session_id,
            "system",
            format!("Proposal #{} rejected: {}", proposal_id.0, title),
        );
    }

    fn apply_proposal(&mut self, proposal_id: ProposalId, ctx: &egui::Context) {
        let Some(snapshot) = self.agents.proposals.get(&proposal_id).cloned() else {
            self.model
                .problems
                .push(format!("Proposal #{} not found", proposal_id.0));
            return;
        };
        if snapshot.status != ProposalStatus::PendingReview {
            self.model
                .problems
                .push(format!("Proposal #{} is not pending review", proposal_id.0));
            return;
        }
        if snapshot
            .expected_revisions
            .iter()
            .any(|(doc_id, expected)| self.model.document_revision(*doc_id) != Some(*expected))
        {
            if let Some(proposal) = self.agents.proposals.get_mut(&proposal_id) {
                proposal.status = ProposalStatus::Stale;
            }
            self.model.problems.push(format!(
                "Proposal #{} is stale and must be regenerated against the latest document state",
                proposal_id.0
            ));
            return;
        }

        if let Some(proposal) = self.agents.proposals.get_mut(&proposal_id) {
            proposal.status = ProposalStatus::Applied;
        }
        self.telemetry.proposal_applied(proposal_id);
        self.agents.active_proposal = Some(proposal_id);
        self.agents.active_session = Some(snapshot.session_id);
        self.append_agent_message(
            snapshot.session_id,
            "system",
            format!("Proposal #{} approved and applied.", proposal_id.0),
        );
        self.apply_transaction(snapshot.transaction, ctx, true);
    }

    fn build_snapshot(&self) -> SessionSnapshot {
        let mut untitled_by_id: BTreeMap<DocumentId, String> = BTreeMap::new();
        for id in &self.model.open_editor_tabs {
            if let Some(doc) = self.model.documents.get(id)
                && matches!(doc.kind, DocumentKind::Spec(_))
                && doc.path.is_none()
            {
                untitled_by_id.insert(*id, format!("untitled-{}", id.0));
            }
        }

        let mut documents = Vec::new();
        for doc in self.model.documents_in_tab_order() {
            match &doc.kind {
                DocumentKind::Board(board) => {
                    if let Some(path) = &doc.path {
                        documents.push(SessionDocumentState::Board {
                            path: path.clone(),
                            view_mode: board.view_mode,
                        });
                    }
                }
                DocumentKind::Spec(spec) => {
                    documents.push(SessionDocumentState::Spec {
                        path: spec.path.clone(),
                        untitled_id: if spec.path.is_none() {
                            untitled_by_id.get(&doc.id).cloned()
                        } else {
                            None
                        },
                        text: spec.text.clone(),
                        dirty: doc.dirty,
                    });
                }
                DocumentKind::DesignOverview(graph) => documents.push(
                    SessionDocumentState::GraphScope {
                        scope: graph.scope.clone(),
                        title: doc.title.clone(),
                        kind: SessionGraphDocumentKind::DesignOverview,
                    },
                ),
                DocumentKind::Logical(graph) => documents.push(SessionDocumentState::GraphScope {
                    scope: graph.scope.clone(),
                    title: doc.title.clone(),
                    kind: SessionGraphDocumentKind::Logical,
                }),
                DocumentKind::Physical(graph) => documents.push(SessionDocumentState::GraphScope {
                    scope: graph.scope.clone(),
                    title: doc.title.clone(),
                    kind: SessionGraphDocumentKind::Physical,
                }),
                DocumentKind::DefinitionCollection(graph) => documents.push(
                    SessionDocumentState::GraphScope {
                        scope: graph.scope.clone(),
                        title: doc.title.clone(),
                        kind: SessionGraphDocumentKind::DefinitionCollection,
                    },
                ),
                DocumentKind::Asset(graph) => documents.push(SessionDocumentState::GraphAsset {
                    asset: graph.asset.clone(),
                    title: doc.title.clone(),
                }),
                DocumentKind::Import(graph) => documents.push(SessionDocumentState::GraphImport {
                    import: graph.import.clone(),
                    title: doc.title.clone(),
                }),
                DocumentKind::SchDocPreview(_)
                | DocumentKind::SchLibGallery(_)
                | DocumentKind::SchLibComponent(_) => {}
                DocumentKind::Keybindings => {
                    documents.push(SessionDocumentState::Keybindings);
                }
            }
        }

        let open_tabs = self
            .model
            .open_editor_tabs
            .iter()
            .filter_map(|id| self.session_tab_ref_for_document(*id, &untitled_by_id))
            .collect();
        let active_tab = self
            .model
            .active_editor_tab
            .and_then(|id| self.session_tab_ref_for_document(id, &untitled_by_id));
        let secondary_active_tab = self
            .editor_split
            .secondary_active_tab
            .and_then(|id| self.session_tab_ref_for_document(id, &untitled_by_id));
        let recently_closed_tabs = self
            .model
            .recently_closed_tabs
            .iter()
            .filter_map(|id| self.session_tab_ref_for_document(*id, &untitled_by_id))
            .collect();

        let shortcut_overrides = shortcut_overrides_from_stored(
            self.shortcut_bindings
                .iter()
                .map(|(k, v)| (k.clone(), shortcut_to_stored(*v))),
        );

        SessionSnapshot {
            schema_version: crate::session::SESSION_SCHEMA_VERSION,
            saved_at_unix_ms: now_unix_ms(),
            ui: SessionUiState {
                panel_visibility: self.panel_visibility.clone(),
                layout: self.layout.clone(),
                editor_split: self.editor_split.clone(),
                palette_mode: self.palette_mode,
                palette_filter: self.palette_filter.clone(),
                palette_selected: self.palette_selected,
            },
            workspace: SessionWorkspaceState {
                workspace_root: self.model.workspace_root.clone(),
                active_workspace_ref: self
                    .model
                    .active_graph
                    .as_ref()
                    .map(|graph| graph.workspace_ref().clone()),
                active_graph_root: self
                    .model
                    .active_graph
                    .as_ref()
                    .map(|graph| graph.graph_root_ref().clone()),
                active_workspace_path: self
                    .model
                    .active_workspace
                    .as_ref()
                    .map(|w| w.project.project_path.clone()),
                active_project_path: self
                    .model
                    .active_workspace
                    .as_ref()
                    .map(|w| w.project.project_path.clone()),
            },
            tabs: SessionTabState {
                open_tabs,
                active_tab,
                secondary_active_tab,
                recently_closed_tabs,
            },
            documents,
            selection: SessionSelectionState {
                selection: self.model.selection.clone(),
            },
            prefs: SessionPrefsState {
                theme: self.theme_prefs.clone(),
                shortcut_overrides,
            },
            agents: self.agents.clone(),
        }
    }

    fn apply_snapshot(&mut self, snapshot: SessionSnapshot) -> anyhow::Result<()> {
        self.panel_visibility = snapshot.ui.panel_visibility;
        self.layout = snapshot.ui.layout;
        self.layout.ensure_required_panes();
        self.editor_split = snapshot.ui.editor_split;
        self.palette_mode = snapshot.ui.palette_mode;
        self.palette_filter = snapshot.ui.palette_filter;
        self.palette_selected = snapshot.ui.palette_selected;

        self.theme_prefs = snapshot.prefs.theme;
        self.refresh_theme_tokens();

        self.shortcut_bindings = default_shortcuts(&self.commands);
        for (id, sc) in snapshot.prefs.shortcut_overrides.by_command {
            if self.commands.get(&id).is_some()
                && let Some(parsed) = shortcut_from_stored(&sc)
            {
                self.shortcut_bindings.insert(id, parsed);
            }
        }

        self.model.clear_workspace();
        if let Some(root) = snapshot.workspace.workspace_root {
            self.model.set_workspace_root(root);
        }
        if let Some(graph_root) = snapshot.workspace.active_graph_root.as_ref() {
            self.model
                .set_active_graph(GraphHost::stub_from_root(graph_root.0.clone()));
        } else if let Some(path) = snapshot.workspace.active_workspace_path.as_ref() {
            self.model.set_active_graph(GraphHost::stub_from_path(path));
        }

        let mut tab_map: BTreeMap<SessionTabRef, DocumentId> = BTreeMap::new();
        for doc in snapshot.documents {
            match doc {
                SessionDocumentState::Board { path, view_mode } => {
                    let before = self.model.active_document_id();
                    self.open_document_path(path.clone());
                    let id = self.model.active_document_id().or(before).ok_or_else(|| {
                        anyhow::anyhow!("failed opening board {}", path.display())
                    })?;
                    if let Some(doc) = self.model.documents.get_mut(&id)
                        && let DocumentKind::Board(board) = &mut doc.kind
                    {
                        board.view_mode = view_mode;
                    }
                    tab_map.insert(SessionTabRef::BoardPath { path }, id);
                }
                SessionDocumentState::GraphScope { scope, title, kind } => {
                    let workspace = self
                        .model
                        .active_graph
                        .as_ref()
                        .map(|graph| graph.workspace_ref().clone())
                        .unwrap_or_default();
                    let id = match kind {
                        SessionGraphDocumentKind::DesignOverview => {
                            self.model.open_design_overview_document(scope.clone(), title)
                        }
                        SessionGraphDocumentKind::Logical => {
                            self.model.open_logical_document(scope.clone(), title)
                        }
                        SessionGraphDocumentKind::Physical => {
                            self.model.open_physical_document(scope.clone(), title)
                        }
                        SessionGraphDocumentKind::DefinitionCollection => self
                            .model
                            .open_definition_collection_document(scope.clone(), title),
                    };
                    let tab_ref = match kind {
                        SessionGraphDocumentKind::DesignOverview => {
                            SessionTabRef::DesignOverview { workspace, scope }
                        }
                        SessionGraphDocumentKind::Logical => {
                            SessionTabRef::LogicalScope { workspace, scope }
                        }
                        SessionGraphDocumentKind::Physical => {
                            SessionTabRef::PhysicalScope { workspace, scope }
                        }
                        SessionGraphDocumentKind::DefinitionCollection => {
                            SessionTabRef::DefinitionScope { workspace, scope }
                        }
                    };
                    tab_map.insert(tab_ref, id);
                }
                SessionDocumentState::GraphAsset { asset, title } => {
                    let workspace = self
                        .model
                        .active_graph
                        .as_ref()
                        .map(|graph| graph.workspace_ref().clone())
                        .unwrap_or_default();
                    let id = self.model.open_asset_document(asset.clone(), title);
                    tab_map.insert(SessionTabRef::AssetScope { workspace, asset }, id);
                }
                SessionDocumentState::GraphImport { import, title } => {
                    let workspace = self
                        .model
                        .active_graph
                        .as_ref()
                        .map(|graph| graph.workspace_ref().clone())
                        .unwrap_or_default();
                    let id = self.model.open_import_document(import.clone(), title);
                    tab_map.insert(SessionTabRef::ImportScope { workspace, import }, id);
                }
                SessionDocumentState::Spec {
                    path,
                    untitled_id,
                    text,
                    dirty,
                } => {
                    let contents = if let Some(path) = &path {
                        fs::read_to_string(path).unwrap_or(text)
                    } else {
                        text
                    };
                    let id = self.model.open_spec_document(path.clone(), contents);
                    self.model.mark_document_dirty(id, dirty);
                    if let Some(path) = path {
                        tab_map.insert(SessionTabRef::SpecTextPath { path }, id);
                    } else if let Some(uid) = untitled_id {
                        tab_map.insert(SessionTabRef::UntitledSpec { untitled_id: uid }, id);
                    }
                }
                SessionDocumentState::Keybindings => {
                    let id = self.model.open_or_activate_keybindings_document();
                    tab_map.insert(SessionTabRef::Keybindings, id);
                }
            }
        }

        self.model.open_editor_tabs = snapshot
            .tabs
            .open_tabs
            .into_iter()
            .filter_map(|t| tab_map.get(&t).copied())
            .collect();
        self.model.active_editor_tab = snapshot
            .tabs
            .active_tab
            .and_then(|t| tab_map.get(&t).copied());
        self.editor_split.secondary_active_tab = snapshot
            .tabs
            .secondary_active_tab
            .and_then(|t| tab_map.get(&t).copied());
        self.model.recently_closed_tabs = snapshot
            .tabs
            .recently_closed_tabs
            .into_iter()
            .filter_map(|t| tab_map.get(&t).copied())
            .collect();

        self.model.selection = snapshot.selection.selection;
        self.agents = snapshot.agents;
        for proposal in self.agents.proposals.values_mut() {
            proposal.target_documents.clear();
            proposal.expected_revisions.clear();
            if proposal.status == ProposalStatus::PendingReview {
                proposal.status = ProposalStatus::Stale;
                proposal.preview_lines.push(
                    "Restored from session snapshot; proposal must be regenerated before apply."
                        .to_owned(),
                );
            }
        }
        if let Some(workspace_path) = snapshot
            .workspace
            .active_workspace_path
            .or(snapshot.workspace.active_project_path)
        {
            self.queue_command_id(
                "workspace.open_project",
                Some(workspace_path.display().to_string()),
            );
        }

        self.prune_tab_renderers();
        self.prune_document_runtime();
        self.session_dirty = false;
        self.last_session_save = Instant::now();
        Ok(())
    }

    fn save_session_now(&mut self) -> anyhow::Result<()> {
        let snapshot = self.build_snapshot();
        self.session_store.save_atomic(&snapshot)?;
        self.session_dirty = false;
        self.last_session_save = Instant::now();
        Ok(())
    }

    fn mark_session_dirty(&mut self) {
        self.session_dirty = true;
    }

    fn maybe_autosave_session(&mut self) {
        if !self.session_dirty {
            return;
        }
        if self.last_session_save.elapsed() < Duration::from_millis(SESSION_AUTOSAVE_DEBOUNCE_MS) {
            return;
        }
        if let Err(err) = self.save_session_now() {
            self.model
                .problems
                .push(format!("Session autosave failed: {err}"));
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
        let active_document_supports_tools = self.model.active_document().is_some_and(|doc| {
            matches!(
                doc.kind,
                DocumentKind::Board(_) | DocumentKind::SchDocPreview(_)
            )
        });
        let selected_component = match &self.model.selection.primary {
            crate::workbench::SelectionKind::Component(designator) => Some(designator.clone()),
            _ => None,
        };
        ResolveContext {
            workspace_open: self.model.has_workspace(),
            selection_exists: self.model.selection_exists(),
            show_primary_sidebar: self.panel_visibility.show_primary_sidebar,
            show_secondary_sidebar: self.panel_visibility.show_secondary_sidebar,
            show_bottom_panel: self.panel_visibility.show_bottom_panel,
            show_activity_bar: self.panel_visibility.show_activity_bar,
            show_status_bar: self.panel_visibility.show_status_bar,
            active_document_supports_tools,
            selected_target: (!matches!(
                self.model.selection.primary,
                crate::workbench::SelectionKind::None
            ))
            .then(|| self.model.selection.primary.clone()),
            selected_component,
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
                if let Err(err) = self.save_session_now() {
                    self.model
                        .problems
                        .push(format!("Session save before quit failed: {err}"));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Effect::ApplyProposal { proposal_id } => self.apply_proposal(proposal_id, ctx),
        }
    }

    fn apply_transaction(
        &mut self,
        tx: CommandTransaction,
        ctx: &egui::Context,
        allow_history_push: bool,
    ) {
        let had_commands = !tx.commands.is_empty();
        let mut inverse = Vec::new();
        let mut domain_events = Vec::new();
        for command in &tx.commands {
            self.telemetry.command_executed(command);
            let (inv, effects, events) = self.apply_command(command.clone());
            if let Some(inv) = inv {
                inverse.push(inv);
            }
            domain_events.extend(events);
            for effect in effects {
                self.apply_effect(effect, ctx);
            }
        }
        self.apply_domain_events(domain_events);

        if allow_history_push && tx.undo_policy == TxUndoPolicy::Track && !inverse.is_empty() {
            inverse.reverse();
            let inverse_tx = CommandTransaction {
                source_intent: tx.source_intent.clone(),
                commands: inverse,
                undo_policy: TxUndoPolicy::Skip,
            };
            self.telemetry.undo_pushed(inverse_tx.commands.len());
            self.undo_stack.push(UndoEntry {
                forward: tx,
                inverse: inverse_tx,
            });
            self.redo_stack.clear();
        }
        if had_commands {
            self.mark_session_dirty();
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

    fn apply_command(
        &mut self,
        command: Command,
    ) -> (Option<Command>, Vec<Effect>, Vec<DomainEvent>) {
        match command {
            Command::OpenKeybindings => {
                self.model.open_or_activate_keybindings_document();
                (None, Vec::new(), Vec::new())
            }
            Command::ThemeOpenManagerTab => {
                self.palette_mode = PaletteMode::Theme;
                self.show_command_palette = true;
                self.palette_focus_pending = true;
                self.palette_filter.clear();
                self.palette_selected = 0;
                (None, Vec::new(), Vec::new())
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
                (
                    Some(Command::SetCommandPaletteVisible(prev)),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::SetPrimarySidebarVisible(value) => {
                let prev = self.panel_visibility.show_primary_sidebar;
                self.panel_visibility.show_primary_sidebar = value;
                (
                    Some(Command::SetPrimarySidebarVisible(prev)),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::SetSecondarySidebarVisible(value) => {
                let prev = self.panel_visibility.show_secondary_sidebar;
                self.panel_visibility.show_secondary_sidebar = value;
                (
                    Some(Command::SetSecondarySidebarVisible(prev)),
                    Vec::new(),
                    Vec::new(),
                )
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
                (Some(Command::SetActivityView(inv)), Vec::new(), Vec::new())
            }
            Command::SetBottomPanelVisible(value) => {
                let prev = self.panel_visibility.show_bottom_panel;
                self.panel_visibility.show_bottom_panel = value;
                (
                    Some(Command::SetBottomPanelVisible(prev)),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::SetBottomTab(tab) => {
                let prev = self.panel_visibility.bottom_tab;
                self.panel_visibility.bottom_tab = tab;
                (Some(Command::SetBottomTab(prev)), Vec::new(), Vec::new())
            }
            Command::SetActivityBarVisible(value) => {
                let prev = self.panel_visibility.show_activity_bar;
                self.panel_visibility.show_activity_bar = value;
                (
                    Some(Command::SetActivityBarVisible(prev)),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::SetStatusBarVisible(value) => {
                let prev = self.panel_visibility.show_status_bar;
                self.panel_visibility.show_status_bar = value;
                (
                    Some(Command::SetStatusBarVisible(prev)),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::ActivateNextEditorTab => {
                let prev = self.model.active_document_id();
                self.model.activate_next_tab();
                (
                    prev.map(|id| Command::EditorActivateDocument { id }),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::ActivatePreviousEditorTab => {
                let prev = self.model.active_document_id();
                self.model.activate_previous_tab();
                (
                    prev.map(|id| Command::EditorActivateDocument { id }),
                    Vec::new(),
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
                (None, Vec::new(), Vec::new())
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
                (None, Vec::new(), Vec::new())
            }
            Command::ResetLayout => {
                self.layout = ShellLayoutState::default();
                self.editor_split = EditorSplitState::default();
                (None, Vec::new(), Vec::new())
            }
            Command::EditorReopenClosed => {
                let _ = self.model.reopen_last_closed_document();
                (None, Vec::new(), Vec::new())
            }
            Command::EditorActivateDocument { id } => {
                let prev = self.model.active_document_id();
                self.model.set_active_tab(id);
                (
                    prev.map(|id| Command::EditorActivateDocument { id }),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::EditorCloseDocument { id } => {
                let _ = self.model.close_document(id);
                if self.editor_split.secondary_active_tab == Some(id) {
                    self.editor_split.secondary_active_tab = self.model.active_editor_tab;
                }
                (None, Vec::new(), Vec::new())
            }
            Command::EditorOpenSchLibComponent {
                source_path,
                source_spec_document,
                component_name,
            } => {
                self.model.open_schlib_component_document(
                    source_path,
                    source_spec_document,
                    component_name,
                );
                self.mark_session_dirty();
                (None, Vec::new(), Vec::new())
            }
            Command::FileClose => {
                let _ = self.model.close_active_document();
                (None, Vec::new(), Vec::new())
            }
            Command::FileCloseAll => {
                while self.model.close_active_document() {}
                (None, Vec::new(), Vec::new())
            }
            Command::FileCloseOthers => {
                self.model.close_other_documents();
                (None, Vec::new(), Vec::new())
            }
            Command::WorkspaceOpen { root } => {
                let root = root
                    .or_else(|| self.model.workspace_root.clone())
                    .or_else(|| std::env::current_dir().ok());
                let Some(root) = root else {
                    self.model
                        .problems
                        .push("Unable to resolve workspace root".to_owned());
                    return (None, Vec::new(), Vec::new());
                };
                self.model.set_workspace_root(root.clone());
                self.model.set_active_graph(GraphHost::stub_from_path(&root));
                self.model
                    .output_lines
                    .push(format!("Workspace opened: {}", root.display()));
                (None, Vec::new(), Vec::new())
            }
            Command::WorkspaceOpenProject { path } => {
                let path = path.or_else(|| self.find_project_in_workspace_root());
                let Some(project_path) = path else {
                    self.model.problems.push(
                        "workspace.open_project requires a .wrk/.PrjPcb path (or one in workspace root)"
                            .to_owned(),
                    );
                    return (None, Vec::new(), Vec::new());
                };
                self.submit_job(JobPayload::ParseProject {
                    project_path: project_path.clone(),
                });
                self.model
                    .output_lines
                    .push(format!("Queued project parse: {}", project_path.display()));
                (None, Vec::new(), Vec::new())
            }
            Command::WorkspaceReloadProject => {
                let workspace_path = self
                    .model
                    .active_workspace
                    .as_ref()
                    .map(|w| w.project.project_path.clone());
                if let Some(project_path) = workspace_path {
                    self.submit_job(JobPayload::ParseProject { project_path });
                } else {
                    self.model
                        .problems
                        .push("No active project workspace to reload".to_owned());
                }
                (None, Vec::new(), Vec::new())
            }
            Command::WorkspaceSyncIr => {
                self.queue_project_sync_jobs();
                (None, Vec::new(), Vec::new())
            }
            Command::WorkspaceClose => {
                self.model.clear_workspace();
                self.tab_renderers.clear();
                self.document_runtime.clear();
                self.pending_job_revisions.clear();
                self.model.output_lines.push("Workspace closed".to_owned());
                (None, Vec::new(), Vec::new())
            }
            Command::FileNewSpec => {
                self.model
                    .open_spec_document(None, "// New spec document\n".to_owned());
                (None, Vec::new(), Vec::new())
            }
            Command::FileOpen { path } => {
                if let Some(path) = path {
                    self.open_document_path(path);
                } else {
                    self.model.output_lines.push(
                        "Use Explorer or pass a path to File: Open from command palette".to_owned(),
                    );
                }
                (None, Vec::new(), Vec::new())
            }
            Command::FileImportAltium { path } => {
                if let Some(path) = path {
                    self.submit_job(JobPayload::ImportAltium { source_path: path });
                } else {
                    self.model
                        .problems
                        .push("file.import_altium requires a source path".to_owned());
                }
                (None, Vec::new(), Vec::new())
            }
            Command::FileSave => {
                self.save_active_document();
                (None, Vec::new(), Vec::new())
            }
            Command::FileSaveAll => {
                self.save_all_documents();
                (None, Vec::new(), Vec::new())
            }
            Command::FileRevert => {
                self.revert_active_document();
                (None, Vec::new(), Vec::new())
            }
            Command::JobsCancelActive => {
                if let Some(id) = self.jobs.cancel_first_active() {
                    self.model
                        .output_lines
                        .push(format!("Requested cancellation for job {}", id.0));
                } else {
                    self.model.output_lines.push("No active jobs".to_owned());
                }
                (None, Vec::new(), Vec::new())
            }
            Command::PcbSetViewMode(mode) => {
                let prev = self.model.active_board().map(|b| b.view_mode);
                let active_doc = self.model.active_document_id();
                if let Some(board) = self.model.active_board_mut() {
                    board.view_mode = mode;
                }
                let events = active_doc
                    .map(|document_id| DomainEvent::BoardViewModeChanged { document_id })
                    .into_iter()
                    .collect();
                (prev.map(Command::PcbSetViewMode), Vec::new(), events)
            }
            Command::PcbZoomFit => {
                self.layout.request_fit = true;
                (None, Vec::new(), Vec::new())
            }
            Command::SetSelection(selection) => {
                let prev = self.model.selection.primary.clone();
                self.model.selection.primary = selection;
                if let Some(runtime) = self.active_document_runtime_mut() {
                    runtime.active_interaction = None;
                }
                (
                    Some(Command::SetSelection(prev)),
                    Vec::new(),
                    vec![DomainEvent::SelectionChanged],
                )
            }
            Command::ToolSetActive { tool } => {
                let Some(runtime) = self.active_document_runtime_mut() else {
                    self.model
                        .problems
                        .push("No active document runtime for tool selection".to_owned());
                    return (None, Vec::new(), Vec::new());
                };
                let prev = runtime.active_tool;
                runtime.active_tool = tool;
                if tool != ToolId::Move {
                    runtime.active_interaction = None;
                }
                (
                    Some(Command::ToolSetActive { tool: prev }),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::ToolBeginMoveSelection { designator } => {
                let active_kind = self.model.active_document().map(|doc| doc.kind_id());
                let Some(runtime) = self.active_document_runtime_mut() else {
                    self.model
                        .problems
                        .push("No active document runtime for move start".to_owned());
                    return (None, Vec::new(), Vec::new());
                };
                runtime.active_interaction = match active_kind {
                    Some(crate::workbench::DOCUMENT_KIND_BOARD) => {
                        Some(ActiveInteraction::BoardMoveSelection {
                            designator,
                            delta_x_mm: 0.0,
                            delta_y_mm: 0.0,
                        })
                    }
                    Some(crate::workbench::DOCUMENT_KIND_SCHDOC_PREVIEW) => {
                        Some(ActiveInteraction::SchDocMoveSelection {
                            designator,
                            delta_x_mils: 0.0,
                            delta_y_mils: 0.0,
                        })
                    }
                    _ => None,
                };
                (None, Vec::new(), Vec::new())
            }
            Command::ToolPreviewMoveSelection {
                designator,
                delta_x_mm,
                delta_y_mm,
            } => {
                let active_kind = self.model.active_document().map(|doc| doc.kind_id());
                let Some(runtime) = self.active_document_runtime_mut() else {
                    self.model
                        .problems
                        .push("No active document runtime for move preview".to_owned());
                    return (None, Vec::new(), Vec::new());
                };
                runtime.active_interaction = match active_kind {
                    Some(crate::workbench::DOCUMENT_KIND_BOARD) => {
                        Some(ActiveInteraction::BoardMoveSelection {
                            designator,
                            delta_x_mm,
                            delta_y_mm,
                        })
                    }
                    Some(crate::workbench::DOCUMENT_KIND_SCHDOC_PREVIEW) => {
                        Some(ActiveInteraction::SchDocMoveSelection {
                            designator,
                            delta_x_mils: delta_x_mm,
                            delta_y_mils: delta_y_mm,
                        })
                    }
                    _ => None,
                };
                (None, Vec::new(), Vec::new())
            }
            Command::MoveComponent {
                designator,
                delta_x_mm,
                delta_y_mm,
            } => {
                let Some(document_id) = self.model.active_document_id() else {
                    self.model
                        .problems
                        .push("Move command requires an active document".to_owned());
                    return (None, Vec::new(), Vec::new());
                };
                let moved = self.move_component_by_designator(
                    document_id,
                    &designator,
                    delta_x_mm,
                    delta_y_mm,
                );
                if !moved {
                    self.model.problems.push(format!(
                        "Selected component '{}' is no longer available in the active document",
                        designator
                    ));
                    return (None, Vec::new(), Vec::new());
                }
                let _ = self.model.bump_document_revision(document_id);
                (
                    Some(Command::MoveComponent {
                        designator: designator.clone(),
                        delta_x_mm: -delta_x_mm,
                        delta_y_mm: -delta_y_mm,
                    }),
                    Vec::new(),
                    vec![DomainEvent::ComponentMoved {
                        document_id,
                        designator,
                    }],
                )
            }
            Command::ToolCancelInteraction => {
                if let Some(runtime) = self.active_document_runtime_mut() {
                    runtime.active_interaction = None;
                }
                (None, Vec::new(), Vec::new())
            }
            Command::AgentCreateSession => {
                let session_id = self.create_agent_session(None);
                self.append_agent_message(
                    session_id,
                    "system",
                    "Agent session created. Persistent design edits will be queued for review instead of applied directly."
                        .to_owned(),
                );
                (None, Vec::new(), Vec::new())
            }
            Command::AgentSubmitPrompt { session_id, prompt } => {
                let session_id = session_id
                    .filter(|id| self.agents.sessions.contains_key(id))
                    .unwrap_or_else(|| self.ensure_active_agent_session());
                self.agents.active_session = Some(session_id);
                if let Some(session) = self.agents.sessions.get_mut(&session_id) {
                    session.status = AgentRunStatus::Running;
                    session.last_error = None;
                }
                self.append_agent_message(session_id, "user", prompt.clone());
                match self.create_move_proposal_from_prompt(session_id, &prompt) {
                    Ok(_) => {}
                    Err(err) => {
                        if let Some(session) = self.agents.sessions.get_mut(&session_id) {
                            session.status = AgentRunStatus::Failed;
                            session.last_error = Some(err.clone());
                        }
                        self.append_agent_message(
                            session_id,
                            "assistant",
                            format!("Unable to create proposal: {err}"),
                        );
                        self.model
                            .problems
                            .push(format!("Agent proposal failed: {err}"));
                    }
                }
                if let Some(session) = self.agents.sessions.get_mut(&session_id)
                    && session.status == AgentRunStatus::Running
                {
                    session.status = AgentRunStatus::Completed;
                }
                self.agents.composer_text.clear();
                (None, Vec::new(), Vec::new())
            }
            Command::ReviewSelectProposal { proposal_id } => {
                self.agents.active_proposal = Some(proposal_id);
                if let Some(proposal) = self.agents.proposals.get(&proposal_id) {
                    self.agents.active_session = Some(proposal.session_id);
                }
                (None, Vec::new(), Vec::new())
            }
            Command::ProposalApply { proposal_id } => {
                if !self.agents.proposals.contains_key(&proposal_id) {
                    self.model
                        .problems
                        .push(format!("Proposal #{} not found", proposal_id.0));
                    return (None, Vec::new(), Vec::new());
                }
                (
                    None,
                    vec![Effect::ApplyProposal { proposal_id }],
                    Vec::new(),
                )
            }
            Command::ProposalReject { proposal_id } => {
                self.mark_proposal_rejected(proposal_id);
                (None, Vec::new(), Vec::new())
            }
            Command::RunStartLast => {
                self.model
                    .output_lines
                    .push("No runnable task configured yet.".to_owned());
                (None, Vec::new(), Vec::new())
            }
            Command::HelpAbout => {
                self.model
                    .output_lines
                    .push("AutoPCB Shell - IDE shell for PCB/spec automation".to_owned());
                (None, Vec::new(), Vec::new())
            }
            Command::SessionSaveNow => {
                if let Err(err) = self.save_session_now() {
                    self.model
                        .problems
                        .push(format!("Session save failed: {err}"));
                } else {
                    self.model.output_lines.push(format!(
                        "session saved: {}",
                        self.session_store.snapshot_path().display()
                    ));
                }
                (None, Vec::new(), Vec::new())
            }
            Command::SessionRestoreLatest => {
                match self.session_store.load_latest() {
                    Ok(Some(snapshot)) => {
                        if let Err(err) = self.apply_snapshot(snapshot) {
                            self.model
                                .problems
                                .push(format!("Session restore failed: {err}"));
                        } else {
                            self.model.output_lines.push(format!(
                                "session restored: {}",
                                self.session_store.snapshot_path().display()
                            ));
                        }
                    }
                    Ok(None) => self
                        .model
                        .output_lines
                        .push("No session snapshot found".to_owned()),
                    Err(err) => self
                        .model
                        .problems
                        .push(format!("Session restore failed: {err}")),
                }
                (None, Vec::new(), Vec::new())
            }
            Command::ThemeCycleNext => {
                let prev = self.theme_prefs.active_theme;
                self.theme_prefs.active_theme = next_theme(prev);
                self.theme_preview = None;
                self.refresh_theme_tokens();
                (
                    Some(Command::ThemeSetActive { id: prev }),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::ThemeCyclePrevious => {
                let prev = self.theme_prefs.active_theme;
                self.theme_prefs.active_theme = previous_theme(prev);
                self.theme_preview = None;
                self.refresh_theme_tokens();
                (
                    Some(Command::ThemeSetActive { id: prev }),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::ThemeSetActive { id } => {
                let prev = self.theme_prefs.active_theme;
                self.theme_prefs.active_theme = id;
                self.theme_preview = None;
                self.refresh_theme_tokens();
                (
                    Some(Command::ThemeSetActive { id: prev }),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::ThemeSetUiScale { scale } => {
                let prev = self.theme_prefs.ui_scale;
                self.theme_prefs.ui_scale = scale.clamp(0.8, 1.75);
                self.refresh_theme_tokens();
                (
                    Some(Command::ThemeSetUiScale { scale: prev }),
                    Vec::new(),
                    Vec::new(),
                )
            }
            Command::EmitEffect(effect) => (None, vec![effect], Vec::new()),
        }
    }

    fn ensure_runtime_for_document(&mut self, id: DocumentId) {
        self.document_runtime
            .entry(id)
            .or_insert_with(|| DocumentRuntime {
                active_tool: ToolId::Select,
                active_interaction: None,
                invalidation: DirtySets::default(),
            });
    }

    fn active_document_runtime_mut(&mut self) -> Option<&mut DocumentRuntime> {
        let id = self.model.active_document_id()?;
        self.ensure_runtime_for_document(id);
        self.document_runtime.get_mut(&id)
    }

    fn move_preview_for_document(&self, id: DocumentId) -> Option<MovePreview> {
        match self
            .document_runtime
            .get(&id)?
            .active_interaction
            .as_ref()?
        {
            ActiveInteraction::BoardMoveSelection {
                designator,
                delta_x_mm,
                delta_y_mm,
            } => Some(MovePreview {
                designator: designator.clone(),
                delta_x_mm: *delta_x_mm,
                delta_y_mm: *delta_y_mm,
            }),
            ActiveInteraction::SchDocMoveSelection { .. } => None,
        }
    }

    fn schdoc_move_preview_for_document(&self, id: DocumentId) -> Option<SchMovePreview> {
        match self
            .document_runtime
            .get(&id)?
            .active_interaction
            .as_ref()?
        {
            ActiveInteraction::SchDocMoveSelection {
                designator,
                delta_x_mils,
                delta_y_mils,
            } => Some(SchMovePreview {
                designator: designator.clone(),
                delta_x_mils: *delta_x_mils,
                delta_y_mils: *delta_y_mils,
            }),
            ActiveInteraction::BoardMoveSelection { .. } => None,
        }
    }

    fn move_component_by_designator(
        &mut self,
        document_id: DocumentId,
        designator: &str,
        delta_x_mm: f32,
        delta_y_mm: f32,
    ) -> bool {
        let Some(kind_id) = self
            .model
            .documents
            .get(&document_id)
            .map(|doc| doc.kind_id())
        else {
            return false;
        };
        match kind_id {
            crate::workbench::DOCUMENT_KIND_BOARD => {
                let Some(doc) = self.model.documents.get_mut(&document_id) else {
                    return false;
                };
                let DocumentKind::Board(board) = &mut doc.kind else {
                    return false;
                };
                let Some((_, component)) = board
                    .ir
                    .components
                    .iter_mut()
                    .find(|(_, component)| component.designator == designator)
                else {
                    return false;
                };
                translate_component(component, delta_x_mm, delta_y_mm);
                true
            }
            crate::workbench::DOCUMENT_KIND_SCHDOC_PREVIEW => self
                .move_schdoc_component_by_designator(
                    document_id,
                    designator,
                    delta_x_mm,
                    delta_y_mm,
                ),
            _ => false,
        }
    }

    fn handle_board_canvas_actions(&mut self, actions: Vec<BoardCanvasAction>) {
        for action in actions {
            match action {
                BoardCanvasAction::ClearSelection => {
                    self.queue_intent(Intent::Selection(crate::pipeline::SelectionIntent::Clear));
                }
                BoardCanvasAction::SelectComponent(designator) => {
                    self.queue_intent(Intent::Selection(
                        crate::pipeline::SelectionIntent::SelectComponent { designator },
                    ));
                }
                BoardCanvasAction::BeginMoveSelection => {
                    self.queue_intent(Intent::Tool(
                        crate::pipeline::ToolIntent::BeginMoveSelection,
                    ));
                }
                BoardCanvasAction::PreviewMoveSelection {
                    delta_x_mm,
                    delta_y_mm,
                } => {
                    self.queue_intent(Intent::Tool(
                        crate::pipeline::ToolIntent::PreviewMoveSelection {
                            delta_x_mm,
                            delta_y_mm,
                        },
                    ));
                }
                BoardCanvasAction::CommitMoveSelection {
                    delta_x_mm,
                    delta_y_mm,
                } => {
                    self.queue_intent(Intent::Tool(
                        crate::pipeline::ToolIntent::CommitMoveSelection {
                            delta_x_mm,
                            delta_y_mm,
                        },
                    ));
                }
            }
        }
    }

    fn handle_schdoc_canvas_actions(&mut self, actions: Vec<SchDocCanvasAction>) {
        for action in actions {
            match action {
                SchDocCanvasAction::ClearSelection => {
                    self.queue_intent(Intent::Selection(crate::pipeline::SelectionIntent::Clear));
                }
                SchDocCanvasAction::SelectComponent(designator) => {
                    self.queue_intent(Intent::Selection(
                        crate::pipeline::SelectionIntent::SelectComponent { designator },
                    ));
                }
                SchDocCanvasAction::BeginMoveSelection => {
                    self.queue_intent(Intent::Tool(
                        crate::pipeline::ToolIntent::BeginMoveSelection,
                    ));
                }
                SchDocCanvasAction::PreviewMoveSelection {
                    delta_x_mils,
                    delta_y_mils,
                } => {
                    self.queue_intent(Intent::Tool(
                        crate::pipeline::ToolIntent::PreviewMoveSelection {
                            delta_x_mm: delta_x_mils,
                            delta_y_mm: delta_y_mils,
                        },
                    ));
                }
                SchDocCanvasAction::CommitMoveSelection {
                    delta_x_mils,
                    delta_y_mils,
                } => {
                    self.queue_intent(Intent::Tool(
                        crate::pipeline::ToolIntent::CommitMoveSelection {
                            delta_x_mm: delta_x_mils,
                            delta_y_mm: delta_y_mils,
                        },
                    ));
                }
            }
        }
    }

    fn prune_document_runtime(&mut self) {
        self.document_runtime.retain(|id, _| {
            self.model.documents.contains_key(id) && self.model.open_editor_tabs.contains(id)
        });
        for id in self.model.open_editor_tabs.clone() {
            self.ensure_runtime_for_document(id);
        }
    }

    fn apply_domain_events(&mut self, events: Vec<DomainEvent>) {
        if events.is_empty() {
            return;
        }
        let mut delta = InvalidationDelta::default();
        let active_document = self.model.active_document_id();
        for event in events {
            match event {
                DomainEvent::SelectionChanged => {
                    if let Some(id) = active_document {
                        delta.add_render_hint(id, RenderDirty::SelectionOverlay);
                        delta.add_connectivity_hint(id, ConnectivityDirty::SelectionOverlay);
                    }
                }
                DomainEvent::BoardViewModeChanged { document_id } => {
                    delta.add_render_hint(document_id, RenderDirty::BoardViewMode);
                    delta.add_drc_hint(document_id, DrcDirty::BoardViewMode);
                }
                DomainEvent::ComponentMoved {
                    document_id,
                    designator,
                } => {
                    delta.add_render_hint(document_id, RenderDirty::Component(designator.clone()));
                    delta.add_connectivity_hint(
                        document_id,
                        ConnectivityDirty::Component(designator.clone()),
                    );
                    delta.add_drc_hint(document_id, DrcDirty::Component(designator));
                }
            }
        }
        for (doc_id, dirty) in delta.by_document {
            self.ensure_runtime_for_document(doc_id);
            if let Some(runtime) = self.document_runtime.get_mut(&doc_id) {
                runtime.invalidation.render.extend(dirty.render);
                runtime.invalidation.connectivity.extend(dirty.connectivity);
                runtime.invalidation.drc.extend(dirty.drc);
            }
        }
    }

    fn submit_job(&mut self, payload: JobPayload) {
        let doc_targets = self.job_doc_targets(&payload);
        let kind = match &payload {
            JobPayload::ParseProject { .. } => JobKind::ParseProject,
            JobPayload::SyncBoardIr { .. } => JobKind::SyncBoardIr,
            JobPayload::SyncSchematicIr { .. } => JobKind::SyncSchematicIr,
            JobPayload::ImportAltium { .. } => JobKind::ImportAltium,
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
            doc_targets: doc_targets.clone(),
            payload,
            requested_by: JobTrigger::Command,
        };
        let id = self.jobs.submit(req);
        let tracked = self.collect_job_revisions_for(&doc_targets);
        if !tracked.is_empty() {
            self.pending_job_revisions.insert(id, tracked);
        }
    }

    fn job_doc_targets(&self, payload: &JobPayload) -> Vec<DocumentId> {
        match payload {
            JobPayload::SyncBoardIr { board_path } => self
                .model
                .find_document_by_path(board_path)
                .into_iter()
                .collect(),
            JobPayload::SyncSchematicIr { schematic_path } => self
                .model
                .find_document_by_path(schematic_path)
                .into_iter()
                .collect(),
            JobPayload::ImportAltium { source_path } => self
                .model
                .find_document_by_path(source_path)
                .into_iter()
                .collect(),
            JobPayload::ParseProject { .. } => Vec::new(),
        }
    }

    fn collect_job_revisions_for(
        &self,
        doc_targets: &[DocumentId],
    ) -> BTreeMap<DocumentId, DocumentRevision> {
        doc_targets
            .iter()
            .filter_map(|id| {
                self.model
                    .document_revision(*id)
                    .map(|revision| (*id, revision))
            })
            .collect()
    }

    fn job_is_stale(&self, id: crate::jobs::JobId) -> bool {
        self.pending_job_revisions.get(&id).is_some_and(|tracked| {
            tracked
                .iter()
                .any(|(doc_id, expected)| self.model.document_revision(*doc_id) != Some(*expected))
        })
    }

    fn find_project_in_workspace_root(&self) -> Option<PathBuf> {
        let root = self.model.workspace_root.as_ref()?;
        let entries = fs::read_dir(root).ok()?;
        let mut legacy: Option<PathBuf> = None;
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase());
            match ext.as_deref() {
                Some("wrk") => return Some(path),
                Some("prjpcb") => legacy = Some(path),
                _ => {}
            }
        }
        legacy
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
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if ext == "pcbdoc" || ext == "pcb" {
                self.submit_job(JobPayload::SyncBoardIr { board_path: path });
            }
        }
        for path in sch_paths {
            self.submit_job(JobPayload::SyncSchematicIr {
                schematic_path: path,
            });
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
            "wrk" | "prjpcb" => {
                self.queue_intent(Intent::Workspace(
                    crate::pipeline::WorkspaceIntent::OpenProject { path: Some(path) },
                ));
            }
            "sch" | "sym" | "pcb" | "wrk-spec" | "spec" | "pcbdoc-spec" | "schdoc-spec"
            | "prjpcb-spec" | "schlib-spec" => match fs::read_to_string(&path) {
                Ok(text) => {
                    let source_doc = self.model.open_spec_document(Some(path.clone()), text);
                    let ext = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_ascii_lowercase())
                        .unwrap_or_default();
                    match ext.as_str() {
                        "sym" | "schlib-spec" => {
                            self.model
                                .open_schlib_gallery_document(path.clone(), Some(source_doc));
                        }
                        "sch" | "schdoc-spec" => {
                            self.model
                                .open_schdoc_preview_document(path.clone(), Some(source_doc));
                        }
                        _ => {}
                    }
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
                    Some(base.join(format!("untitled-{}.wrk", id.0)))
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
        let mut changed = false;
        for ev in self.jobs.poll_events() {
            changed = true;
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
                JobEvent::Artifact(id, artifact) => {
                    if self.job_is_stale(id) {
                        self.model
                            .output_lines
                            .push(format!("Dropped stale job result for job #{}", id.0));
                        continue;
                    }
                    match artifact {
                        JobArtifact::ProjectGraphDelta(delta) => {
                            let root = delta
                                .graph
                                .project_path
                                .parent()
                                .map(ToOwned::to_owned)
                                .unwrap_or_else(|| PathBuf::from("."));
                            let workspace_id = hash_path_id(&delta.graph.project_path);
                            let workspace = WorkspaceModel {
                                id: workspace_id,
                                root,
                                project: delta.graph,
                                opened_at: std::time::SystemTime::now(),
                                last_sync: None,
                            };
                            let graph_stub =
                                GraphHost::stub_from_path(&workspace.project.project_path);
                            self.model.set_active_workspace(workspace);
                            self.model.set_active_graph(graph_stub);
                            self.model
                                .output_lines
                                .push(format!("Loaded project graph from job #{}", id.0));
                            if let Some(ws) = &self.model.active_workspace {
                                for board in &ws.project.board_docs {
                                    if !board.path.exists() {
                                        self.model.problems.push(format!(
                                            "Workspace references missing board file: {}",
                                            board.path.display()
                                        ));
                                    }
                                }
                                for sch in &ws.project.schematic_docs {
                                    if !sch.path.exists() {
                                        self.model.problems.push(format!(
                                            "Workspace references missing schematic file: {}",
                                            sch.path.display()
                                        ));
                                    }
                                }
                            }
                            self.queue_project_sync_jobs();
                        }
                        JobArtifact::BoardIr { path, ir } => {
                            let _ = self.model.open_board_document(path.clone(), ir);
                            if let Some(doc_id) = self.model.find_document_by_path(&path) {
                                self.apply_domain_events(vec![DomainEvent::BoardViewModeChanged {
                                    document_id: doc_id,
                                }]);
                            }
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
                        JobArtifact::BoardSpecValidated { path } => {
                            if let Some(ws) = self.model.active_workspace.as_mut() {
                                for board in &mut ws.project.board_docs {
                                    if board.path == path {
                                        board.parse_state = ParseState::Fresh;
                                        board.ir_state = ParseState::Fresh;
                                    }
                                }
                                ws.last_sync = Some(std::time::SystemTime::now());
                            }
                            self.model
                                .output_lines
                                .push(format!("Native board validated: {}", path.display()));
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
                        JobArtifact::Diagnostics(diags) => {
                            for d in diags {
                                self.model
                                    .problems
                                    .push(format!("[{}:{}] {}", d.severity, d.source, d.message));
                            }
                        }
                    }
                }
                JobEvent::Completed(id, summary) => {
                    self.pending_job_revisions.remove(&id);
                    self.model.jobs.push(format!(
                        "completed #{} in {}ms: {}",
                        id.0, summary.duration_ms, summary.message
                    ));
                }
                JobEvent::Failed(id, failure) => {
                    self.pending_job_revisions.remove(&id);
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
                    self.pending_job_revisions.remove(&id);
                    self.model.jobs.push(format!("cancelled #{}", id.0));
                }
            }
        }
        if changed {
            self.mark_session_dirty();
        }
    }

    fn scan_watched_files(&mut self) {
        if self.last_watch_scan.elapsed() < Duration::from_millis(1000) {
            return;
        }
        self.last_watch_scan = Instant::now();

        let mut desired_paths: Vec<PathBuf> = Vec::new();
        if let Some(ws) = &self.model.active_workspace {
            desired_paths.push(ws.project.project_path.clone());
            desired_paths.extend(ws.project.board_docs.iter().map(|b| b.path.clone()));
            desired_paths.extend(ws.project.schematic_docs.iter().map(|s| s.path.clone()));
        }
        for doc in self.model.documents.values() {
            if let DocumentKind::Spec(spec) = &doc.kind
                && let Some(path) = &spec.path
            {
                desired_paths.push(path.clone());
            }
        }

        desired_paths.sort();
        desired_paths.dedup();
        self.watched_files
            .retain(|path, _| desired_paths.iter().any(|p| p == path));
        for path in desired_paths {
            if let Ok(meta) = fs::metadata(&path)
                && let Ok(modified) = meta.modified()
            {
                self.watched_files.entry(path).or_insert(modified);
            }
        }

        let mut changed_paths: Vec<PathBuf> = Vec::new();
        for (path, last) in &mut self.watched_files {
            let Ok(meta) = fs::metadata(path) else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            if modified > *last {
                *last = modified;
                changed_paths.push(path.clone());
            }
        }

        for path in changed_paths {
            self.handle_external_file_change(&path);
        }
    }

    fn handle_external_file_change(&mut self, path: &Path) {
        if let Some(doc) =
            self.model.documents.values_mut().find(|d| {
                matches!(d.kind, DocumentKind::Spec(_)) && d.path.as_deref() == Some(path)
            })
        {
            if doc.dirty {
                self.model.problems.push(format!(
                    "External change detected for dirty file {}; reload skipped",
                    path.display()
                ));
                return;
            }
            match fs::read_to_string(path) {
                Ok(text) => {
                    if let DocumentKind::Spec(spec) = &mut doc.kind {
                        spec.text = text;
                    }
                    doc.dirty = false;
                    self.model
                        .output_lines
                        .push(format!("Auto-reloaded {}", path.display()));
                }
                Err(err) => self.model.problems.push(format!(
                    "External change detected but reload failed for {}: {err}",
                    path.display()
                )),
            }
            return;
        }

        let is_workspace_file = self
            .model
            .active_workspace
            .as_ref()
            .is_some_and(|w| w.project.project_path == path);
        if is_workspace_file {
            self.queue_intent(Intent::Workspace(
                crate::pipeline::WorkspaceIntent::ReloadProject,
            ));
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
                IpcRequest::OpenProject { project_path } => {
                    self.queue_command_id("workspace.open_project", Some(project_path));
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
                IpcRequest::SessionSaveNow => {
                    if let Err(err) = self.save_session_now() {
                        self.model
                            .problems
                            .push(format!("IPC session save failed: {err}"));
                    } else {
                        self.model.output_lines.push(format!(
                            "session saved: {}",
                            self.session_store.snapshot_path().display()
                        ));
                    }
                }
                IpcRequest::SessionRestoreLatest => match self.session_store.load_latest() {
                    Ok(Some(snapshot)) => {
                        if let Err(err) = self.apply_snapshot(snapshot) {
                            self.model
                                .problems
                                .push(format!("IPC session restore failed: {err}"));
                        } else {
                            self.model.output_lines.push(format!(
                                "session restored: {}",
                                self.session_store.snapshot_path().display()
                            ));
                        }
                    }
                    Ok(None) => self
                        .model
                        .output_lines
                        .push("No session snapshot found".to_owned()),
                    Err(err) => self
                        .model
                        .problems
                        .push(format!("IPC session restore failed: {err}")),
                },
                IpcRequest::SessionRestorePath { path } => {
                    match FileSessionStore::new(PathBuf::from(path)).load_latest() {
                        Ok(Some(snapshot)) => {
                            if let Err(err) = self.apply_snapshot(snapshot) {
                                self.model
                                    .problems
                                    .push(format!("IPC session restore failed: {err}"));
                            }
                        }
                        Ok(None) => self
                            .model
                            .output_lines
                            .push("Session snapshot path not found".to_owned()),
                        Err(err) => self
                            .model
                            .problems
                            .push(format!("IPC session restore failed: {err}")),
                    }
                }
                IpcRequest::SessionGetPath => self.model.output_lines.push(format!(
                    "session path: {}",
                    self.session_store.snapshot_path().display()
                )),
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
            self.activate_command_id(&id);
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
        ui.menu_button("File", |ui| {
            self.menu_intent_button(
                ui,
                "New Spec",
                "file.new_spec",
                Intent::File(crate::pipeline::FileIntent::NewSpec),
            );
            if ui
                .button(self.menu_label_with_shortcut("Open...", "file.open"))
                .clicked()
            {
                self.open_document_from_dialog();
                ui.close();
            }
            if ui
                .button(
                    self.menu_label_with_shortcut("Open Workspace...", "workspace.open_project"),
                )
                .clicked()
            {
                self.open_workspace_project_from_dialog();
                ui.close();
            }
            ui.separator();
            self.menu_intent_button(
                ui,
                "Save",
                "file.save",
                Intent::File(crate::pipeline::FileIntent::Save),
            );
            self.menu_intent_button(
                ui,
                "Save All",
                "file.save_all",
                Intent::File(crate::pipeline::FileIntent::SaveAll),
            );
            ui.separator();
            self.menu_intent_button(
                ui,
                "Close",
                "file.close",
                Intent::File(crate::pipeline::FileIntent::Close),
            );
            self.menu_intent_button(
                ui,
                "Close All",
                "file.close_all",
                Intent::File(crate::pipeline::FileIntent::CloseAll),
            );
            self.menu_intent_button(
                ui,
                "Close Others",
                "file.close_others",
                Intent::File(crate::pipeline::FileIntent::CloseOthers),
            );
        });

        ui.menu_button("Edit", |ui| {
            self.menu_intent_button(
                ui,
                "Undo",
                "edit.undo",
                Intent::History(crate::pipeline::HistoryIntent::Undo),
            );
            self.menu_intent_button(
                ui,
                "Redo",
                "edit.redo",
                Intent::History(crate::pipeline::HistoryIntent::Redo),
            );
        });

        ui.menu_button("View", |ui| {
            self.menu_intent_button(
                ui,
                "Next Editor Tab",
                "view.next_editor_tab",
                Intent::View(crate::pipeline::ViewIntent::NextEditorTab),
            );
            self.menu_intent_button(
                ui,
                "Previous Editor Tab",
                "view.previous_editor_tab",
                Intent::View(crate::pipeline::ViewIntent::PreviousEditorTab),
            );
            self.menu_intent_button(
                ui,
                "Split Editor Right",
                "view.split_editor_right",
                Intent::View(crate::pipeline::ViewIntent::SplitEditorRight),
            );
            self.menu_intent_button(
                ui,
                "Split Editor Down",
                "view.split_editor_down",
                Intent::View(crate::pipeline::ViewIntent::SplitEditorDown),
            );
            ui.separator();
            self.menu_intent_button(
                ui,
                "Toggle Primary Sidebar",
                "view.toggle_primary_sidebar",
                Intent::View(crate::pipeline::ViewIntent::TogglePrimarySidebar),
            );
            self.menu_intent_button(
                ui,
                "Toggle Secondary Sidebar",
                "view.toggle_secondary_sidebar",
                Intent::View(crate::pipeline::ViewIntent::ToggleSecondarySidebar),
            );
            self.menu_intent_button(
                ui,
                "Toggle Bottom Panel",
                "view.toggle_bottom_panel",
                Intent::View(crate::pipeline::ViewIntent::ToggleBottomPanel),
            );
            self.menu_intent_button(
                ui,
                "Toggle Activity Bar",
                "view.toggle_activity_bar",
                Intent::View(crate::pipeline::ViewIntent::ToggleActivityBar),
            );
            self.menu_intent_button(
                ui,
                "Toggle Status Bar",
                "view.toggle_status_bar",
                Intent::View(crate::pipeline::ViewIntent::ToggleStatusBar),
            );
            self.menu_intent_button(
                ui,
                "Reset Layout",
                "view.reset_layout",
                Intent::View(crate::pipeline::ViewIntent::ResetLayout),
            );
        });

        ui.menu_button("Tools", |ui| {
            self.menu_intent_button(
                ui,
                "Select",
                "tool.select",
                Intent::Tool(crate::pipeline::ToolIntent::SetActive {
                    tool: ToolId::Select,
                }),
            );
            self.menu_intent_button(
                ui,
                "Move",
                "tool.move",
                Intent::Tool(crate::pipeline::ToolIntent::SetActive { tool: ToolId::Move }),
            );
            self.menu_intent_button(
                ui,
                "Route",
                "tool.route",
                Intent::Tool(crate::pipeline::ToolIntent::SetActive {
                    tool: ToolId::Route,
                }),
            );
            self.menu_intent_button(
                ui,
                "Polygon Pour",
                "tool.pour",
                Intent::Tool(crate::pipeline::ToolIntent::SetActive { tool: ToolId::Pour }),
            );
            self.menu_intent_button(
                ui,
                "Cancel Interaction",
                "tool.cancel",
                Intent::Tool(crate::pipeline::ToolIntent::CancelInteraction),
            );
        });

        ui.menu_button("Workspace", |ui| {
            self.menu_intent_button(
                ui,
                "Open Folder",
                "workspace.open",
                Intent::Workspace(crate::pipeline::WorkspaceIntent::Open { root: None }),
            );
            if ui
                .button(self.menu_label_with_shortcut("Open Project...", "workspace.open_project"))
                .clicked()
            {
                self.open_workspace_project_from_dialog();
                ui.close();
            }
            self.menu_intent_button(
                ui,
                "Reload Project",
                "workspace.reload_project",
                Intent::Workspace(crate::pipeline::WorkspaceIntent::ReloadProject),
            );
            self.menu_intent_button(
                ui,
                "Sync IR",
                "workspace.sync_ir",
                Intent::Workspace(crate::pipeline::WorkspaceIntent::SyncIr),
            );
            self.menu_intent_button(
                ui,
                "Close Workspace",
                "workspace.close",
                Intent::Workspace(crate::pipeline::WorkspaceIntent::Close),
            );
        });

        ui.menu_button("Go", |ui| {
            self.menu_intent_button(
                ui,
                "Command Palette",
                "workbench.command_palette",
                Intent::Navigate(crate::pipeline::NavigateIntent::CommandPalette),
            );
            self.menu_intent_button(
                ui,
                "Quick Open",
                "navigate.quick_open",
                Intent::Navigate(crate::pipeline::NavigateIntent::QuickOpen),
            );
        });

        ui.menu_button("Panel", |ui| {
            self.menu_intent_button(
                ui,
                "Show Explorer",
                "panel.show.explorer",
                Intent::Panel(crate::pipeline::PanelIntent::ShowExplorer),
            );
            self.menu_intent_button(
                ui,
                "Show Inspector",
                "panel.show.inspector",
                Intent::Panel(crate::pipeline::PanelIntent::ShowInspector),
            );
            self.menu_intent_button(
                ui,
                "Show Problems",
                "panel.show.problems",
                Intent::Panel(crate::pipeline::PanelIntent::ShowProblems),
            );
            self.menu_intent_button(
                ui,
                "Show Output",
                "panel.show.output",
                Intent::Panel(crate::pipeline::PanelIntent::ShowOutput),
            );
            self.menu_intent_button(
                ui,
                "Show Jobs",
                "panel.show.jobs",
                Intent::Panel(crate::pipeline::PanelIntent::ShowJobs),
            );
        });

        ui.menu_button("Theme", |ui| {
            self.menu_intent_button(
                ui,
                "Theme Manager",
                "theme.open_manager",
                Intent::Theme(crate::pipeline::ThemeIntent::OpenManager),
            );
            self.menu_intent_button(
                ui,
                "Next Theme",
                "theme.next",
                Intent::Theme(crate::pipeline::ThemeIntent::NextTheme),
            );
            self.menu_intent_button(
                ui,
                "Previous Theme",
                "theme.previous",
                Intent::Theme(crate::pipeline::ThemeIntent::PreviousTheme),
            );
        });

        ui.menu_button("Session", |ui| {
            self.menu_intent_button(
                ui,
                "Save Session Now",
                "session.save_now",
                Intent::Session(crate::pipeline::SessionIntent::SaveNow),
            );
            self.menu_intent_button(
                ui,
                "Restore Last Session",
                "session.restore_last",
                Intent::Session(crate::pipeline::SessionIntent::RestoreLatest),
            );
        });

        ui.menu_button("Help", |ui| {
            self.menu_intent_button(
                ui,
                "About",
                "help.about",
                Intent::Help(crate::pipeline::HelpIntent::About),
            );
        });
    }

    fn menu_label_with_shortcut(&self, title: &str, command_id: &str) -> String {
        let shortcut = self
            .shortcut_bindings
            .get(command_id)
            .map(|s| s.display())
            .unwrap_or_default();
        if shortcut.is_empty() {
            title.to_owned()
        } else {
            format!("{title}\t{shortcut}")
        }
    }

    fn menu_intent_button(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        command_id: &str,
        intent: Intent,
    ) {
        if ui
            .button(self.menu_label_with_shortcut(title, command_id))
            .clicked()
        {
            self.queue_intent(intent);
            ui.close();
        }
    }

    fn open_workspace_project_from_dialog(&mut self) {
        let mut dialog = FileDialog::new().add_filter("Workspace Project", &["wrk", "prjpcb"]);
        if let Some(root) = &self.model.workspace_root {
            dialog = dialog.set_directory(root);
        }
        if let Some(path) = dialog.pick_file() {
            self.queue_intent(Intent::Workspace(
                crate::pipeline::WorkspaceIntent::OpenProject { path: Some(path) },
            ));
        }
    }

    fn open_document_from_dialog(&mut self) {
        let mut dialog = FileDialog::new()
            .add_filter(
                "Openable Files",
                &[
                    "wrk",
                    "prjpcb",
                    "pcbdoc",
                    "sch",
                    "sym",
                    "pcb",
                    "spec",
                    "wrk-spec",
                    "pcbdoc-spec",
                    "schdoc-spec",
                    "prjpcb-spec",
                    "schlib-spec",
                ],
            )
            .add_filter("Workspace Project", &["wrk", "prjpcb"])
            .add_filter("Board", &["pcbdoc", "pcb", "pcbdoc-spec"])
            .add_filter(
                "Spec Source",
                &[
                    "sch",
                    "sym",
                    "spec",
                    "wrk-spec",
                    "schdoc-spec",
                    "prjpcb-spec",
                    "schlib-spec",
                ],
            );
        if let Some(root) = &self.model.workspace_root {
            dialog = dialog.set_directory(root);
        }
        if let Some(path) = dialog.pick_file() {
            self.queue_intent(Intent::File(crate::pipeline::FileIntent::Open {
                path: Some(path),
            }));
        }
    }

    pub(crate) fn activate_command_id(&mut self, id: &str) {
        match id {
            "file.open" => self.open_document_from_dialog(),
            "workspace.open_project" => self.open_workspace_project_from_dialog(),
            _ => self.queue_command_id(id, None),
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

        self.ensure_runtime_for_document(document_id);
        let active_tool = self
            .document_runtime
            .get(&document_id)
            .map(|runtime| runtime.active_tool)
            .unwrap_or(ToolId::Select);
        let move_preview = self.move_preview_for_document(document_id);

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
        let tools = [
            SegmentItem::new(ToolId::Select, "Select"),
            SegmentItem::new(ToolId::Move, "Move"),
            SegmentItem::new(ToolId::Route, "Route"),
            SegmentItem::new(ToolId::Pour, "Pour"),
        ];
        if let Some(changed) = segmented_bar(ui, &self.theme, active_tool, &tools) {
            self.queue_intent(Intent::Tool(crate::pipeline::ToolIntent::SetActive {
                tool: changed,
            }));
        }
        if let Some(preview) = &move_preview {
            ui.label(format!(
                "Move preview: {} dx={:.2}mm dy={:.2}mm",
                preview.designator, preview.delta_x_mm, preview.delta_y_mm
            ));
        }
        ui.separator();

        let selection = self.model.selection.primary.clone();
        let board = self
            .model
            .documents
            .get(&document_id)
            .and_then(|doc| match &doc.kind {
                DocumentKind::Board(board) => Some(board),
                _ => None,
            });
        if let Some(board) = board {
            let actions = match mode {
                BoardViewMode::TwoD => self.canvas2d.ui(
                    ui,
                    &board.ir,
                    &selection,
                    active_tool,
                    move_preview.as_ref(),
                    fit_requested,
                ),
                BoardViewMode::ThreeD => self.canvas3d.ui(
                    ui,
                    &board.ir,
                    &selection,
                    active_tool,
                    move_preview.as_ref(),
                    fit_requested,
                ),
            };
            self.handle_board_canvas_actions(actions);
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
            let _ = self.model.bump_document_revision(document_id);
            self.mark_session_dirty();
        }
    }

    pub(super) fn render_graph_scope_document(&mut self, ui: &mut egui::Ui, document_id: DocumentId) {
        let Some(doc) = self.model.documents.get(&document_id) else {
            ui.label("Graph scope unavailable");
            return;
        };
        let Some(graph) = self.model.active_graph.as_ref() else {
            empty_state(ui, &self.theme, "No active graph workspace.");
            return;
        };

        let (scope, kind_id) = match &doc.kind {
            DocumentKind::DesignOverview(graph_doc)
            | DocumentKind::Logical(graph_doc)
            | DocumentKind::Physical(graph_doc)
            | DocumentKind::DefinitionCollection(graph_doc) => {
                (graph_doc.scope.clone(), doc.kind_id())
            }
            _ => {
                ui.label("Graph scope unavailable");
                return;
            }
        };

        if let Some(summary) = graph.inspector_summary_for_scope(&scope) {
            ui.heading(summary.title);
            if let Some(subtitle) = summary.subtitle {
                ui.label(subtitle);
            }
        }

        match kind_id {
            crate::workbench::DOCUMENT_KIND_LOGICAL | crate::workbench::DOCUMENT_KIND_DESIGN_OVERVIEW => {
                if let Some(render) = graph.logical_render_model(&scope) {
                    ui.separator();
                    ui.label(format!("Revision: {}", render.revision));
                    for warning in render.warnings {
                        ui.label(warning);
                    }
                    for shape in render.shapes {
                        ui.label(format!("Shape: {}", shape.label.unwrap_or(shape.id)));
                    }
                } else {
                    empty_state(ui, &self.theme, "No logical render snapshot available.");
                }
            }
            crate::workbench::DOCUMENT_KIND_PHYSICAL => {
                if let Some(render) = graph.physical_render_model(&scope) {
                    ui.separator();
                    ui.label(format!("Revision: {}", render.revision));
                    for warning in render.warnings {
                        ui.label(warning);
                    }
                    for shape in render.shapes {
                        ui.label(format!("Shape: {}", shape.label.unwrap_or(shape.id)));
                    }
                } else {
                    empty_state(ui, &self.theme, "No physical render snapshot available.");
                }
            }
            crate::workbench::DOCUMENT_KIND_DEFINITION_COLLECTION => {
                if let Some(render) = graph.definition_preview_model(&scope) {
                    ui.separator();
                    ui.label(format!("Revision: {}", render.revision));
                    for shape in render.shapes {
                        ui.label(format!("Shape: {}", shape.label.unwrap_or(shape.id)));
                    }
                } else {
                    empty_state(ui, &self.theme, "No definition preview available.");
                }
            }
            _ => empty_state(ui, &self.theme, "Unsupported graph scope kind."),
        }
    }

    pub(super) fn render_graph_asset_document(&mut self, ui: &mut egui::Ui, document_id: DocumentId) {
        let Some(doc) = self.model.documents.get(&document_id) else {
            ui.label("Graph asset unavailable");
            return;
        };
        let Some(graph) = self.model.active_graph.as_ref() else {
            empty_state(ui, &self.theme, "No active graph workspace.");
            return;
        };
        let DocumentKind::Asset(graph_doc) = &doc.kind else {
            ui.label("Graph asset unavailable");
            return;
        };

        if let Some(summary) = graph.asset_summary(&graph_doc.asset) {
            ui.heading(summary.title);
            ui.label(format!("Authority: {:?}", summary.authority));
            ui.label(format!("Storage: {:?}", summary.storage));
            if let Some(digest) = summary.digest {
                ui.label(format!("Digest: {digest}"));
            }
        } else {
            empty_state(ui, &self.theme, "Selected asset is missing from the active graph host.");
        }
    }

    pub(super) fn render_graph_import_document(&mut self, ui: &mut egui::Ui, document_id: DocumentId) {
        let Some(doc) = self.model.documents.get(&document_id) else {
            ui.label("Graph import unavailable");
            return;
        };
        let Some(graph) = self.model.active_graph.as_ref() else {
            empty_state(ui, &self.theme, "No active graph workspace.");
            return;
        };
        let DocumentKind::Import(graph_doc) = &doc.kind else {
            ui.label("Graph import unavailable");
            return;
        };

        if let Some(summary) = graph.import_summary(&graph_doc.import) {
            ui.heading(summary.title);
            ui.label(format!("Source kind: {}", summary.source_kind));
        } else {
            empty_state(ui, &self.theme, "Selected import is missing from the active graph host.");
        }
    }

    pub(super) fn render_schdoc_preview_document(
        &mut self,
        ui: &mut egui::Ui,
        document_id: DocumentId,
    ) {
        let (source_path, source_spec_document) = match self.model.documents.get(&document_id) {
            Some(doc) => match &doc.kind {
                DocumentKind::SchDocPreview(preview) => {
                    (preview.source_path.clone(), preview.source_spec_document)
                }
                _ => {
                    ui.label("SchDoc preview unavailable");
                    return;
                }
            },
            None => {
                ui.label("SchDoc preview unavailable");
                return;
            }
        };

        ui.horizontal(|ui| {
            ui.label(format!("Source: {}", source_path.display()));
            if ui.button("Edit spec").clicked() {
                self.activate_source_spec_document(source_path.clone(), source_spec_document);
            }
        });
        ui.separator();

        let source_text = match self.source_spec_text(source_path.as_path(), source_spec_document) {
            Ok(text) => text,
            Err(err) => {
                ui.colored_label(self.theme.text_disabled, err);
                return;
            }
        };

        let schdoc = match build_schdoc_from_spec_source(&source_text) {
            Ok(doc) => doc,
            Err(err) => {
                ui.colored_label(self.theme.text_disabled, err);
                return;
            }
        };
        let sheet = match schdoc.sheet() {
            Ok(sheet) => sheet,
            Err(err) => {
                ui.colored_label(
                    self.theme.text_disabled,
                    format!("failed to decode sheet: {err}"),
                );
                return;
            }
        };

        self.ensure_runtime_for_document(document_id);
        let active_tool = self
            .document_runtime
            .get(&document_id)
            .map(|runtime| runtime.active_tool)
            .unwrap_or(ToolId::Select);
        let move_preview = self.schdoc_move_preview_for_document(document_id);

        let tools = [
            SegmentItem::new(ToolId::Select, "Select"),
            SegmentItem::new(ToolId::Move, "Move"),
            SegmentItem::new(ToolId::Route, "Route"),
            SegmentItem::new(ToolId::Pour, "Pour"),
        ];
        if let Some(changed) = segmented_bar(ui, &self.theme, active_tool, &tools) {
            self.queue_intent(Intent::Tool(crate::pipeline::ToolIntent::SetActive {
                tool: changed,
            }));
        }
        if let Some(preview) = &move_preview {
            ui.label(format!(
                "Move preview: {} dx={:.0}mil dy={:.0}mil",
                preview.designator, preview.delta_x_mils, preview.delta_y_mils
            ));
        }
        ui.separator();

        let selection = self.model.selection.primary.clone();
        let actions = self.schdoc_canvas2d.ui(
            ui,
            &sheet,
            &selection,
            active_tool,
            move_preview.as_ref(),
            false,
        );
        self.handle_schdoc_canvas_actions(actions);
    }

    pub(super) fn render_schlib_gallery_document(
        &mut self,
        ui: &mut egui::Ui,
        document_id: DocumentId,
    ) {
        let (source_path, source_spec_document) = match self.model.documents.get(&document_id) {
            Some(doc) => match &doc.kind {
                DocumentKind::SchLibGallery(gallery) => {
                    (gallery.source_path.clone(), gallery.source_spec_document)
                }
                _ => {
                    ui.label("SchLib gallery unavailable");
                    return;
                }
            },
            None => {
                ui.label("SchLib gallery unavailable");
                return;
            }
        };

        ui.horizontal(|ui| {
            ui.label(format!("Source: {}", source_path.display()));
            if ui.button("Edit spec").clicked() {
                self.activate_source_spec_document(source_path.clone(), source_spec_document);
            }
        });
        ui.separator();

        let source_text = match self.source_spec_text(source_path.as_path(), source_spec_document) {
            Ok(text) => text,
            Err(err) => {
                ui.colored_label(self.theme.text_disabled, err);
                return;
            }
        };

        let lib = match build_schlib_from_spec_source(&source_text) {
            Ok(lib) => lib,
            Err(err) => {
                ui.colored_label(self.theme.text_disabled, err);
                return;
            }
        };
        let mut component_names = lib.component_names();
        component_names.sort();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for component_name in component_names {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(&component_name);
                        if ui.button("Open component tab").clicked() {
                            self.queue_intent(Intent::Editor(
                                crate::pipeline::EditorIntent::OpenSchLibComponent {
                                    source_path: source_path.clone(),
                                    source_spec_document,
                                    component_name: component_name.clone(),
                                },
                            ));
                        }
                    });
                    match render_schlib_component_png(&lib, &component_name, DEFAULT_SCALE * 0.5) {
                        Ok(png) => {
                            let key = format!(
                                "schlib-gallery:{}:{}",
                                source_path.display(),
                                component_name
                            );
                            self.render_png_preview(ui, &key, &source_text, &png);
                        }
                        Err(err) => {
                            ui.colored_label(
                                self.theme.text_disabled,
                                format!("render failed: {err}"),
                            );
                        }
                    }
                });
                ui.add_space(8.0);
            }
        });
    }

    pub(super) fn render_schlib_component_document(
        &mut self,
        ui: &mut egui::Ui,
        document_id: DocumentId,
    ) {
        let (source_path, source_spec_document, component_name) =
            match self.model.documents.get(&document_id) {
                Some(doc) => match &doc.kind {
                    DocumentKind::SchLibComponent(component) => (
                        component.source_path.clone(),
                        component.source_spec_document,
                        component.component_name.clone(),
                    ),
                    _ => {
                        ui.label("SchLib component preview unavailable");
                        return;
                    }
                },
                None => {
                    ui.label("SchLib component preview unavailable");
                    return;
                }
            };

        ui.horizontal(|ui| {
            ui.strong(&component_name);
            if ui.button("Edit spec").clicked() {
                self.activate_source_spec_document(source_path.clone(), source_spec_document);
            }
        });
        ui.separator();

        let source_text = match self.source_spec_text(source_path.as_path(), source_spec_document) {
            Ok(text) => text,
            Err(err) => {
                ui.colored_label(self.theme.text_disabled, err);
                return;
            }
        };
        let lib = match build_schlib_from_spec_source(&source_text) {
            Ok(lib) => lib,
            Err(err) => {
                ui.colored_label(self.theme.text_disabled, err);
                return;
            }
        };
        match render_schlib_component_png(&lib, &component_name, DEFAULT_SCALE * 0.75) {
            Ok(png) => {
                let key = format!(
                    "schlib-component:{}:{}",
                    source_path.display(),
                    component_name
                );
                self.render_png_preview(ui, &key, &source_text, &png);
            }
            Err(err) => {
                ui.colored_label(self.theme.text_disabled, format!("render failed: {err}"));
            }
        }
    }

    fn source_spec_text(
        &self,
        source_path: &Path,
        source_spec_document: Option<DocumentId>,
    ) -> Result<String, String> {
        if let Some(doc_id) = source_spec_document
            && let Some(doc) = self.model.documents.get(&doc_id)
            && let DocumentKind::Spec(spec) = &doc.kind
        {
            return Ok(spec.text.clone());
        }
        fs::read_to_string(source_path)
            .map_err(|e| format!("failed to read {}: {e}", source_path.display()))
    }

    fn activate_source_spec_document(
        &mut self,
        source_path: PathBuf,
        source_spec_document: Option<DocumentId>,
    ) {
        if let Some(doc_id) = source_spec_document
            && self.model.documents.contains_key(&doc_id)
        {
            self.model.set_active_tab(doc_id);
            return;
        }
        if let Ok(text) = fs::read_to_string(&source_path) {
            self.model.open_spec_document(Some(source_path), text);
        }
    }

    fn ensure_source_spec_document(
        &mut self,
        preview_document_id: DocumentId,
    ) -> Result<DocumentId, String> {
        let (source_path, source_spec_document) =
            match self.model.documents.get(&preview_document_id) {
                Some(doc) => match &doc.kind {
                    DocumentKind::SchDocPreview(preview) => {
                        (preview.source_path.clone(), preview.source_spec_document)
                    }
                    _ => return Err("schematic preview unavailable".to_owned()),
                },
                None => return Err("schematic preview unavailable".to_owned()),
            };

        if let Some(doc_id) = source_spec_document
            && self.model.documents.contains_key(&doc_id)
        {
            return Ok(doc_id);
        }

        let text = fs::read_to_string(&source_path)
            .map_err(|e| format!("failed to read {}: {e}", source_path.display()))?;
        let doc_id = self.model.open_spec_document(Some(source_path), text);
        self.model.set_active_tab(preview_document_id);
        if let Some(doc) = self.model.documents.get_mut(&preview_document_id)
            && let DocumentKind::SchDocPreview(preview) = &mut doc.kind
        {
            preview.source_spec_document = Some(doc_id);
        }
        Ok(doc_id)
    }

    fn move_schdoc_component_by_designator(
        &mut self,
        preview_document_id: DocumentId,
        designator: &str,
        delta_x_mils: f32,
        delta_y_mils: f32,
    ) -> bool {
        let Ok(source_doc_id) = self.ensure_source_spec_document(preview_document_id) else {
            return false;
        };
        let Some(doc) = self.model.documents.get_mut(&source_doc_id) else {
            return false;
        };
        let DocumentKind::Spec(spec) = &mut doc.kind else {
            return false;
        };
        let Ok(updated_text) =
            rewrite_component_location_in_spec(&spec.text, designator, delta_x_mils, delta_y_mils)
        else {
            return false;
        };
        spec.text = updated_text;
        doc.dirty = true;
        let _ = self.model.bump_document_revision(source_doc_id);
        let _ = self.model.bump_document_revision(preview_document_id);
        self.mark_session_dirty();
        true
    }

    fn render_png_preview(&mut self, ui: &mut egui::Ui, cache_key: &str, source: &str, png: &[u8]) {
        let source_hash = hash_bytes(source.as_bytes());
        let image_hash = hash_bytes(png);
        let cache = self
            .preview_cache
            .by_key
            .entry(cache_key.to_owned())
            .or_insert_with(|| PreviewTextureEntry {
                text_hash: 0,
                image_hash: 0,
                texture: None,
                error: None,
            });

        if cache.texture.is_none()
            || cache.text_hash != source_hash
            || cache.image_hash != image_hash
        {
            match png_to_color_image(png) {
                Ok(img) => {
                    let texture = ui.ctx().load_texture(
                        cache_key.to_owned(),
                        img,
                        egui::TextureOptions::LINEAR,
                    );
                    cache.texture = Some(texture);
                    cache.text_hash = source_hash;
                    cache.image_hash = image_hash;
                    cache.error = None;
                }
                Err(err) => {
                    cache.texture = None;
                    cache.error = Some(err);
                }
            }
        }

        if let Some(texture) = &cache.texture {
            let size = texture.size_vec2();
            ui.image((texture.id(), size));
        } else {
            ui.colored_label(
                self.theme.text_disabled,
                cache
                    .error
                    .clone()
                    .unwrap_or_else(|| "preview unavailable".to_owned()),
            );
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

fn first_prompt_number(prompt: &str) -> Option<f32> {
    prompt
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| c == ',' || c == ';' || c == '(' || c == ')'))
        .find_map(|token| token.parse::<f32>().ok())
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
    fn save(&mut self, _storage: &mut dyn efame::Storage) {
        if let Err(err) = self.save_session_now() {
            self.model
                .problems
                .push(format!("Session save failed: {err}"));
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut efame::Frame) {
        self.process_ipc();
        self.process_job_events();
        self.scan_watched_files();
        self.apply_ui_test_ops(ctx);
        self.capture_shortcut_if_needed(ctx);
        self.handle_shortcuts(ctx);
        self.process_queue(ctx);
        apply_theme(ctx, &self.theme);
        self.prune_tab_renderers();
        self.prune_document_runtime();

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
        self.maybe_autosave_session();
        self.write_layout_probe();

        let escape_pressed = ctx.input(|i| {
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
        });
        if escape_pressed {
            if self.show_command_palette {
                self.show_command_palette = false;
                self.palette_focus_pending = false;
                self.theme_preview = None;
                self.refresh_theme_tokens();
            } else {
                self.queue_intent(Intent::Tool(crate::pipeline::ToolIntent::CancelInteraction));
            }
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

fn hash_bytes(input: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

fn hash_path_id(path: &Path) -> u64 {
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string();
    hash_bytes(canonical.as_bytes())
}

fn png_to_color_image(bytes: &[u8]) -> Result<ColorImage, String> {
    let decoded = image::load_from_memory(bytes)
        .map_err(|e| format!("failed to decode preview image: {e}"))?
        .to_rgba8();
    let size = [decoded.width() as usize, decoded.height() as usize];
    let flat = decoded.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, flat.as_slice()))
}

fn build_schlib_from_spec_source(source_text: &str) -> Result<altium_format::SchLib, String> {
    let ast = parse_spec(source_text).map_err(|e| e.to_string())?;
    let model = compile_spec(&ast, SpecDomain::SchLib).map_err(|e| e.to_string())?;
    let SpecModel::SchLib(spec) = model else {
        return Err("spec did not compile as SchLib".to_owned());
    };
    let mut lib = altium_format::SchLib::new_blank_ad26().map_err(|e| e.to_string())?;
    let _ = lib.remove_component("Component_1");
    apply_spec_schlib(&spec, &mut lib).map_err(|e| e.to_string())?;
    Ok(lib)
}

fn build_schdoc_from_spec_source(source_text: &str) -> Result<altium_format::SchDoc, String> {
    let ast = parse_spec(source_text).map_err(|e| e.to_string())?;
    let model = compile_spec(&ast, SpecDomain::SchDoc).map_err(|e| e.to_string())?;
    let SpecModel::SchDoc(spec) = model else {
        return Err("spec did not compile as SchDoc".to_owned());
    };
    let mut doc = altium_format::SchDoc::new_blank_ad26();
    apply_spec_schdoc(&spec, &mut doc).map_err(|e| e.to_string())?;
    Ok(doc)
}

fn rewrite_component_location_in_spec(
    source: &str,
    designator: &str,
    delta_x_mils: f32,
    delta_y_mils: f32,
) -> Result<String, String> {
    let mut lines: Vec<String> = source.lines().map(ToOwned::to_owned).collect();
    let mut in_component = false;
    let mut brace_depth = 0_i32;
    let mut component_start = None;
    let quoted_header = format!("component \"{designator}\"");
    let bare_header = format!("component {designator}");

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !in_component
            && (trimmed.starts_with(&quoted_header) || trimmed.starts_with(&bare_header))
        {
            in_component = true;
            component_start = Some(idx);
        }

        if in_component {
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
            if trimmed.starts_with("at:") {
                let current = parse_spec_coord_point(trimmed.trim_start_matches("at:").trim())?;
                let next = CoordPoint::new(
                    current.x + Coord::from_mils_f64(delta_x_mils as f64),
                    current.y + Coord::from_mils_f64(delta_y_mils as f64),
                );
                let indent = line.chars().take_while(|c| c.is_ascii_whitespace()).count();
                lines[idx] = format!("{}at: {}", " ".repeat(indent), next);
                return Ok(lines.join("\n") + if source.ends_with('\n') { "\n" } else { "" });
            }
            if brace_depth <= 0 {
                break;
            }
        }
    }

    Err(match component_start {
        Some(_) => format!("component '{designator}' has no at: property"),
        None => format!("component '{designator}' not found in schematic spec"),
    })
}

fn parse_spec_coord_point(value: &str) -> Result<CoordPoint, String> {
    let trimmed = value.trim();
    let inner = trimmed
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| format!("invalid coordinate point '{value}'"))?;
    let mut parts = inner.split(',');
    let x = parts
        .next()
        .ok_or_else(|| format!("invalid coordinate point '{value}'"))
        .and_then(parse_spec_coord)?;
    let y = parts
        .next()
        .ok_or_else(|| format!("invalid coordinate point '{value}'"))
        .and_then(parse_spec_coord)?;
    if parts.next().is_some() {
        return Err(format!("invalid coordinate point '{value}'"));
    }
    Ok(CoordPoint::new(x, y))
}

fn parse_spec_coord(value: &str) -> Result<Coord, String> {
    let trimmed = value.trim();
    if let Some(number) = trimmed.strip_suffix("mil") {
        let parsed = number
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("invalid mil coordinate '{value}': {e}"))?;
        return Ok(Coord::from_mils_f64(parsed));
    }
    if let Some(number) = trimmed.strip_suffix("mm") {
        let parsed = number
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("invalid mm coordinate '{value}': {e}"))?;
        return Ok(Coord::from_mms(parsed));
    }
    Err(format!("unsupported coordinate '{value}'"))
}

#[cfg(test)]
mod tests {
    use super::{
        PanelVisibilityState, SecondarySidebarTab, clamp_bottom_panel_height, first_prompt_number,
        parse_spec_coord_point, rewrite_component_location_in_spec,
    };
    use altium_format_types::coord::Coord;

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

    #[test]
    fn prompt_number_parser_extracts_first_numeric_token() {
        assert_eq!(first_prompt_number("move U1 right 2.5 mm"), Some(2.5));
        assert_eq!(first_prompt_number("shift down"), None);
    }

    #[test]
    fn rewrite_component_location_updates_at_line() {
        let source = "component \"U1\" {\n    lib_reference: \"X\"\n    at: (1000mil, 800mil)\n}\n";
        let updated = rewrite_component_location_in_spec(source, "U1", 50.0, -25.0).unwrap();
        let at_line = updated
            .lines()
            .find(|line| line.trim_start().starts_with("at:"))
            .expect("updated component has at line");
        let point = parse_spec_coord_point(at_line.trim_start().trim_start_matches("at:").trim())
            .expect("updated at line parses");
        assert_eq!(point.x, Coord::from_mils_f64(1050.0));
        assert!((point.y.to_mils() - 775.0).abs() < 0.001);
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
