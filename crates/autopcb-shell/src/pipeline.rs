use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::agents::{AgentSessionId, ProposalId};
use crate::layout::BottomTab;
use crate::ui::theme::ThemeId;
use crate::workbench::{BoardViewMode, DocumentId, SelectionKind};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum ToolId {
    #[default]
    Select,
    Move,
    Route,
    Pour,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityViewIntent {
    Explorer,
    Search,
    SourceControl,
    Run,
    Extensions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SecondarySidebarTabIntent {
    #[default]
    Inspector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppIntent {
    Quit,
    OpenKeybindings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceIntent {
    Open { root: Option<PathBuf> },
    OpenProject { path: Option<PathBuf> },
    ReloadProject,
    SyncIr,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileIntent {
    NewSpec,
    Open { path: Option<PathBuf> },
    ImportAltium { path: Option<PathBuf> },
    Save,
    SaveAll,
    Revert,
    Close,
    CloseAll,
    CloseOthers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigateIntent {
    CommandPalette,
    QuickOpen,
    GoQuickOpen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewIntent {
    NextEditorTab,
    PreviousEditorTab,
    SplitEditorRight,
    SplitEditorDown,
    TogglePrimarySidebar,
    ToggleActivityBar,
    ToggleStatusBar,
    ToggleSecondarySidebar,
    ToggleBottomPanel,
    ResetLayout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelIntent {
    ShowExplorer,
    ShowSearch,
    ShowSourceControl,
    ShowRun,
    ShowExtensions,
    ShowInspector,
    ShowProblems,
    ShowOutput,
    ShowJobs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobsIntent {
    CancelActive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorIntent {
    ReopenClosed,
    ActivateDocument {
        id: DocumentId,
    },
    CloseDocument {
        id: DocumentId,
    },
    OpenSchLibComponent {
        source_path: PathBuf,
        source_spec_document: Option<DocumentId>,
        component_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryIntent {
    Undo,
    Redo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PcbIntent {
    SetView2d,
    SetView3d,
    ZoomFit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionIntent {
    Clear,
    SelectComponent { designator: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossprobeIntent {
    SelectComponent { designator: String },
    SelectNet { net_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunIntent {
    StartLast,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelpIntent {
    About,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalIntent {
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionIntent {
    SaveNow,
    RestoreLatest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThemeIntent {
    OpenManager,
    NextTheme,
    PreviousTheme,
    SetTheme { id: ThemeId },
    SetUiScale { scale: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolIntent {
    SetActive { tool: ToolId },
    BeginMoveSelection,
    PreviewMoveSelection { delta_x_mm: f32, delta_y_mm: f32 },
    CommitMoveSelection { delta_x_mm: f32, delta_y_mm: f32 },
    CancelInteraction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentIntent {
    OpenPanel,
    CreateSession,
    SubmitPrompt {
        session_id: Option<AgentSessionId>,
        prompt: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewIntent {
    OpenQueue,
    SelectProposal { proposal_id: ProposalId },
    AcceptProposal { proposal_id: ProposalId },
    RejectProposal { proposal_id: ProposalId },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Intent {
    App(AppIntent),
    Workspace(WorkspaceIntent),
    File(FileIntent),
    Navigate(NavigateIntent),
    View(ViewIntent),
    Panel(PanelIntent),
    Jobs(JobsIntent),
    Editor(EditorIntent),
    History(HistoryIntent),
    Pcb(PcbIntent),
    Selection(SelectionIntent),
    Crossprobe(CrossprobeIntent),
    Run(RunIntent),
    Help(HelpIntent),
    Terminal(TerminalIntent),
    Session(SessionIntent),
    Theme(ThemeIntent),
    Tool(ToolIntent),
    Agent(AgentIntent),
    Review(ReviewIntent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentParseError {
    UnknownCommandId { id: String },
    InvalidArgument { id: String, message: String },
}

#[derive(Debug, Clone)]
pub struct ResolveContext {
    pub workspace_open: bool,
    pub selection_exists: bool,
    pub show_primary_sidebar: bool,
    pub show_secondary_sidebar: bool,
    pub show_bottom_panel: bool,
    pub show_activity_bar: bool,
    pub show_status_bar: bool,
    pub active_document_supports_tools: bool,
    pub selected_component: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectCode {
    MissingWorkspace,
    MissingSelection,
    MissingBoardDocument,
    MissingComponentSelection,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolveResult {
    Accepted { transaction: CommandTransaction },
    Rejected { code: RejectCode, message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandTransaction {
    pub source_intent: Intent,
    pub commands: Vec<Command>,
    pub undo_policy: TxUndoPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxUndoPolicy {
    /// Push inverse commands into history if generated.
    Track,
    /// Do not push into history even if inverse commands exist.
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    RequestQuit,
    ApplyProposal { proposal_id: ProposalId },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    OpenKeybindings,
    SetCommandPaletteVisible(bool),

    SetPrimarySidebarVisible(bool),
    SetSecondarySidebarVisible(bool),
    SetSecondarySidebarTab(SecondarySidebarTabIntent),
    SetActivityView(ActivityViewIntent),
    SetBottomPanelVisible(bool),
    SetBottomTab(BottomTab),
    SetActivityBarVisible(bool),
    SetStatusBarVisible(bool),

    ActivateNextEditorTab,
    ActivatePreviousEditorTab,
    SetEditorSplitRight,
    SetEditorSplitDown,
    ResetLayout,
    EditorReopenClosed,
    EditorActivateDocument {
        id: DocumentId,
    },
    EditorCloseDocument {
        id: DocumentId,
    },
    EditorOpenSchLibComponent {
        source_path: PathBuf,
        source_spec_document: Option<DocumentId>,
        component_name: String,
    },

    FileClose,
    FileCloseAll,
    FileCloseOthers,

    WorkspaceOpen {
        root: Option<PathBuf>,
    },
    WorkspaceOpenProject {
        path: Option<PathBuf>,
    },
    WorkspaceReloadProject,
    WorkspaceSyncIr,
    WorkspaceClose,

    FileNewSpec,
    FileOpen {
        path: Option<PathBuf>,
    },
    FileImportAltium {
        path: Option<PathBuf>,
    },
    FileSave,
    FileSaveAll,
    FileRevert,

    JobsCancelActive,

    PcbSetViewMode(BoardViewMode),
    PcbZoomFit,

    SetSelection(SelectionKind),
    MoveComponent {
        designator: String,
        delta_x_mm: f32,
        delta_y_mm: f32,
    },

    RunStartLast,
    HelpAbout,
    SessionSaveNow,
    SessionRestoreLatest,
    ThemeOpenManagerTab,
    ThemeCycleNext,
    ThemeCyclePrevious,
    ThemeSetActive {
        id: ThemeId,
    },
    ThemeSetUiScale {
        scale: f32,
    },
    ToolSetActive {
        tool: ToolId,
    },
    ToolBeginMoveSelection {
        designator: String,
    },
    ToolPreviewMoveSelection {
        designator: String,
        delta_x_mm: f32,
        delta_y_mm: f32,
    },
    ToolCancelInteraction,
    AgentCreateSession,
    AgentSubmitPrompt {
        session_id: Option<AgentSessionId>,
        prompt: String,
    },
    ReviewSelectProposal {
        proposal_id: ProposalId,
    },
    ProposalApply {
        proposal_id: ProposalId,
    },
    ProposalReject {
        proposal_id: ProposalId,
    },

    EmitEffect(Effect),
}

pub trait TelemetrySink {
    fn intent_received(&self, intent: &Intent);
    fn intent_rejected(&self, intent: &Intent, code: &RejectCode, message: &str);
    fn commands_resolved(&self, tx: &CommandTransaction);
    fn command_executed(&self, command: &Command);
    fn undo_pushed(&self, count: usize);
    fn agent_session_started(&self, session_id: AgentSessionId);
    fn proposal_created(&self, proposal_id: ProposalId);
    fn proposal_applied(&self, proposal_id: ProposalId);
    fn proposal_rejected(&self, proposal_id: ProposalId);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TracingTelemetry;

impl TelemetrySink for TracingTelemetry {
    fn intent_received(&self, intent: &Intent) {
        info!(target: "autopcb_shell::intent", ?intent, "intent_received");
    }

    fn intent_rejected(&self, intent: &Intent, code: &RejectCode, message: &str) {
        warn!(target: "autopcb_shell::intent", ?intent, ?code, message, "intent_rejected");
    }

    fn commands_resolved(&self, tx: &CommandTransaction) {
        info!(target: "autopcb_shell::command", source_intent=?tx.source_intent, command_count=tx.commands.len(), "commands_resolved");
    }

    fn command_executed(&self, command: &Command) {
        info!(target: "autopcb_shell::command", ?command, "command_executed");
    }

    fn undo_pushed(&self, count: usize) {
        info!(target: "autopcb_shell::history", command_count=count, "undo_pushed");
    }

    fn agent_session_started(&self, session_id: AgentSessionId) {
        info!(target: "autopcb_shell::agent", session_id=session_id.0, "agent_session_started");
    }

    fn proposal_created(&self, proposal_id: ProposalId) {
        info!(target: "autopcb_shell::review", proposal_id=proposal_id.0, "proposal_created");
    }

    fn proposal_applied(&self, proposal_id: ProposalId) {
        info!(target: "autopcb_shell::review", proposal_id=proposal_id.0, "proposal_applied");
    }

    fn proposal_rejected(&self, proposal_id: ProposalId) {
        info!(target: "autopcb_shell::review", proposal_id=proposal_id.0, "proposal_rejected");
    }
}

pub fn intent_from_command_id(id: &str, arg: Option<String>) -> Result<Intent, IntentParseError> {
    match id {
        "app.quit" => Ok(Intent::App(AppIntent::Quit)),
        "app.open_keybindings" => Ok(Intent::App(AppIntent::OpenKeybindings)),

        "workspace.open" | "file.open_folder" => Ok(Intent::Workspace(WorkspaceIntent::Open {
            root: arg.map(PathBuf::from),
        })),
        "workspace.open_project" => Ok(Intent::Workspace(WorkspaceIntent::OpenProject {
            path: arg.map(PathBuf::from),
        })),
        "workspace.reload_project" => Ok(Intent::Workspace(WorkspaceIntent::ReloadProject)),
        "workspace.sync_ir" => Ok(Intent::Workspace(WorkspaceIntent::SyncIr)),
        "workspace.close" => Ok(Intent::Workspace(WorkspaceIntent::Close)),

        "file.new_spec" => Ok(Intent::File(FileIntent::NewSpec)),
        "file.open" => Ok(Intent::File(FileIntent::Open {
            path: arg.map(PathBuf::from),
        })),
        "file.import_altium" => Ok(Intent::File(FileIntent::ImportAltium {
            path: arg.map(PathBuf::from),
        })),
        "file.save" => Ok(Intent::File(FileIntent::Save)),
        "file.save_all" => Ok(Intent::File(FileIntent::SaveAll)),
        "file.revert" => Ok(Intent::File(FileIntent::Revert)),
        "file.close" => Ok(Intent::File(FileIntent::Close)),
        "file.close_all" => Ok(Intent::File(FileIntent::CloseAll)),
        "file.close_others" => Ok(Intent::File(FileIntent::CloseOthers)),

        "edit.undo" | "history.undo" => Ok(Intent::History(HistoryIntent::Undo)),
        "edit.redo" | "history.redo" => Ok(Intent::History(HistoryIntent::Redo)),

        "workbench.command_palette" => Ok(Intent::Navigate(NavigateIntent::CommandPalette)),
        "navigate.quick_open" => Ok(Intent::Navigate(NavigateIntent::QuickOpen)),
        "go.quick_open" => Ok(Intent::Navigate(NavigateIntent::GoQuickOpen)),

        "view.next_editor_tab" => Ok(Intent::View(ViewIntent::NextEditorTab)),
        "view.previous_editor_tab" => Ok(Intent::View(ViewIntent::PreviousEditorTab)),
        "view.split_editor_right" => Ok(Intent::View(ViewIntent::SplitEditorRight)),
        "view.split_editor_down" => Ok(Intent::View(ViewIntent::SplitEditorDown)),
        "view.toggle_primary_sidebar" => Ok(Intent::View(ViewIntent::TogglePrimarySidebar)),
        "view.toggle_activity_bar" => Ok(Intent::View(ViewIntent::ToggleActivityBar)),
        "view.toggle_status_bar" => Ok(Intent::View(ViewIntent::ToggleStatusBar)),
        "view.toggle_secondary_sidebar" => Ok(Intent::View(ViewIntent::ToggleSecondarySidebar)),
        "view.toggle_bottom_panel" => Ok(Intent::View(ViewIntent::ToggleBottomPanel)),
        "view.reset_layout" => Ok(Intent::View(ViewIntent::ResetLayout)),

        "panel.show.explorer" => Ok(Intent::Panel(PanelIntent::ShowExplorer)),
        "panel.show.search" => Ok(Intent::Panel(PanelIntent::ShowSearch)),
        "panel.show.source_control" => Ok(Intent::Panel(PanelIntent::ShowSourceControl)),
        "panel.show.run" => Ok(Intent::Panel(PanelIntent::ShowRun)),
        "panel.show.extensions" => Ok(Intent::Panel(PanelIntent::ShowExtensions)),
        "panel.show.inspector" => Ok(Intent::Panel(PanelIntent::ShowInspector)),
        "panel.show.problems" => Ok(Intent::Panel(PanelIntent::ShowProblems)),
        "panel.show.output" => Ok(Intent::Panel(PanelIntent::ShowOutput)),
        "panel.show.jobs" => Ok(Intent::Panel(PanelIntent::ShowJobs)),

        "run.start_last" => Ok(Intent::Run(RunIntent::StartLast)),
        "jobs.cancel_active" => Ok(Intent::Jobs(JobsIntent::CancelActive)),
        "terminal.toggle" => Ok(Intent::Terminal(TerminalIntent::Toggle)),
        "session.save_now" => Ok(Intent::Session(SessionIntent::SaveNow)),
        "session.restore_last" => Ok(Intent::Session(SessionIntent::RestoreLatest)),
        "agent.open_panel" => Ok(Intent::Agent(AgentIntent::OpenPanel)),
        "review.open_queue" => Ok(Intent::Review(ReviewIntent::OpenQueue)),
        "help.about" => Ok(Intent::Help(HelpIntent::About)),
        "theme.open_manager" => Ok(Intent::Theme(ThemeIntent::OpenManager)),
        "theme.next" => Ok(Intent::Theme(ThemeIntent::NextTheme)),
        "theme.previous" => Ok(Intent::Theme(ThemeIntent::PreviousTheme)),
        "tool.select" => Ok(Intent::Tool(ToolIntent::SetActive {
            tool: ToolId::Select,
        })),
        "tool.move" => Ok(Intent::Tool(ToolIntent::SetActive { tool: ToolId::Move })),
        "tool.route" => Ok(Intent::Tool(ToolIntent::SetActive {
            tool: ToolId::Route,
        })),
        "tool.pour" => Ok(Intent::Tool(ToolIntent::SetActive { tool: ToolId::Pour })),
        "tool.cancel" => Ok(Intent::Tool(ToolIntent::CancelInteraction)),

        "editor.reopen_closed" => Ok(Intent::Editor(EditorIntent::ReopenClosed)),
        "editor.activate_document" => {
            let raw = arg.ok_or_else(|| IntentParseError::InvalidArgument {
                id: id.to_owned(),
                message: "missing document id argument".to_owned(),
            })?;
            let id = raw.parse::<u64>().map(DocumentId).map_err(|_| {
                IntentParseError::InvalidArgument {
                    id: id.to_owned(),
                    message: format!("invalid document id: {raw}"),
                }
            })?;
            Ok(Intent::Editor(EditorIntent::ActivateDocument { id }))
        }
        "editor.close_document" => {
            let raw = arg.ok_or_else(|| IntentParseError::InvalidArgument {
                id: id.to_owned(),
                message: "missing document id argument".to_owned(),
            })?;
            let id = raw.parse::<u64>().map(DocumentId).map_err(|_| {
                IntentParseError::InvalidArgument {
                    id: id.to_owned(),
                    message: format!("invalid document id: {raw}"),
                }
            })?;
            Ok(Intent::Editor(EditorIntent::CloseDocument { id }))
        }

        "pcb.view.2d" => Ok(Intent::Pcb(PcbIntent::SetView2d)),
        "pcb.view.3d" => Ok(Intent::Pcb(PcbIntent::SetView3d)),
        "pcb.zoom.fit" => Ok(Intent::Pcb(PcbIntent::ZoomFit)),

        "selection.clear" => Ok(Intent::Selection(SelectionIntent::Clear)),

        "crossprobe.select_component" => {
            let designator = arg.ok_or_else(|| IntentParseError::InvalidArgument {
                id: id.to_owned(),
                message: "missing component designator argument".to_owned(),
            })?;
            if designator.is_empty() {
                return Err(IntentParseError::InvalidArgument {
                    id: id.to_owned(),
                    message: "component designator cannot be empty".to_owned(),
                });
            }
            Ok(Intent::Crossprobe(CrossprobeIntent::SelectComponent {
                designator,
            }))
        }
        "crossprobe.select_net" => {
            let net_name = arg.ok_or_else(|| IntentParseError::InvalidArgument {
                id: id.to_owned(),
                message: "missing net name argument".to_owned(),
            })?;
            if net_name.is_empty() {
                return Err(IntentParseError::InvalidArgument {
                    id: id.to_owned(),
                    message: "net name cannot be empty".to_owned(),
                });
            }
            Ok(Intent::Crossprobe(CrossprobeIntent::SelectNet { net_name }))
        }
        _ => Err(IntentParseError::UnknownCommandId { id: id.to_owned() }),
    }
}

pub fn resolve_intent(intent: Intent, ctx: ResolveContext) -> ResolveResult {
    use Command as C;

    if matches!(
        intent,
        Intent::Workspace(WorkspaceIntent::ReloadProject)
            | Intent::Workspace(WorkspaceIntent::SyncIr)
            | Intent::Workspace(WorkspaceIntent::Close)
            | Intent::File(FileIntent::Save)
            | Intent::File(FileIntent::SaveAll)
            | Intent::File(FileIntent::Revert)
            | Intent::Jobs(JobsIntent::CancelActive)
            | Intent::Pcb(_)
            | Intent::Editor(_)
            | Intent::File(FileIntent::Close)
            | Intent::File(FileIntent::CloseAll)
            | Intent::File(FileIntent::CloseOthers)
    ) && !ctx.workspace_open
    {
        return ResolveResult::Rejected {
            code: RejectCode::MissingWorkspace,
            message: "Command requires an open workspace".to_owned(),
        };
    }

    if matches!(intent, Intent::Selection(SelectionIntent::Clear)) && !ctx.selection_exists {
        return ResolveResult::Rejected {
            code: RejectCode::MissingSelection,
            message: "No selection to clear".to_owned(),
        };
    }

    if matches!(intent, Intent::Tool(_)) && !ctx.active_document_supports_tools {
        return ResolveResult::Rejected {
            code: RejectCode::MissingBoardDocument,
            message: "Tool commands require an active board or schematic document".to_owned(),
        };
    }

    if matches!(
        intent,
        Intent::Tool(ToolIntent::BeginMoveSelection)
            | Intent::Tool(ToolIntent::PreviewMoveSelection { .. })
            | Intent::Tool(ToolIntent::CommitMoveSelection { .. })
    ) && ctx.selected_component.is_none()
    {
        return ResolveResult::Rejected {
            code: RejectCode::MissingComponentSelection,
            message: "Move tool requires a selected component".to_owned(),
        };
    }

    let commands = match &intent {
        Intent::App(AppIntent::Quit) => vec![C::EmitEffect(Effect::RequestQuit)],
        Intent::App(AppIntent::OpenKeybindings) => vec![C::OpenKeybindings],

        Intent::Workspace(WorkspaceIntent::Open { root }) => {
            vec![C::WorkspaceOpen { root: root.clone() }]
        }
        Intent::Workspace(WorkspaceIntent::OpenProject { path }) => {
            vec![C::WorkspaceOpenProject { path: path.clone() }]
        }
        Intent::Workspace(WorkspaceIntent::ReloadProject) => vec![C::WorkspaceReloadProject],
        Intent::Workspace(WorkspaceIntent::SyncIr) => vec![C::WorkspaceSyncIr],
        Intent::Workspace(WorkspaceIntent::Close) => vec![C::WorkspaceClose],

        Intent::File(FileIntent::NewSpec) => vec![C::FileNewSpec],
        Intent::File(FileIntent::Open { path }) => vec![C::FileOpen { path: path.clone() }],
        Intent::File(FileIntent::ImportAltium { path }) => {
            vec![C::FileImportAltium { path: path.clone() }]
        }
        Intent::File(FileIntent::Save) => vec![C::FileSave],
        Intent::File(FileIntent::SaveAll) => vec![C::FileSaveAll],
        Intent::File(FileIntent::Revert) => vec![C::FileRevert],
        Intent::File(FileIntent::Close) => vec![C::FileClose],
        Intent::File(FileIntent::CloseAll) => vec![C::FileCloseAll],
        Intent::File(FileIntent::CloseOthers) => vec![C::FileCloseOthers],

        Intent::Navigate(NavigateIntent::CommandPalette)
        | Intent::Navigate(NavigateIntent::QuickOpen)
        | Intent::Navigate(NavigateIntent::GoQuickOpen) => vec![C::SetCommandPaletteVisible(true)],

        Intent::View(ViewIntent::NextEditorTab) => vec![C::ActivateNextEditorTab],
        Intent::View(ViewIntent::PreviousEditorTab) => vec![C::ActivatePreviousEditorTab],
        Intent::View(ViewIntent::SplitEditorRight) => vec![C::SetEditorSplitRight],
        Intent::View(ViewIntent::SplitEditorDown) => vec![C::SetEditorSplitDown],
        Intent::View(ViewIntent::TogglePrimarySidebar) => {
            vec![C::SetPrimarySidebarVisible(!ctx.show_primary_sidebar)]
        }
        Intent::View(ViewIntent::ToggleActivityBar) => {
            vec![C::SetActivityBarVisible(!ctx.show_activity_bar)]
        }
        Intent::View(ViewIntent::ToggleStatusBar) => {
            vec![C::SetStatusBarVisible(!ctx.show_status_bar)]
        }
        Intent::View(ViewIntent::ToggleSecondarySidebar) => {
            vec![C::SetSecondarySidebarVisible(!ctx.show_secondary_sidebar)]
        }
        Intent::View(ViewIntent::ToggleBottomPanel) => {
            vec![C::SetBottomPanelVisible(!ctx.show_bottom_panel)]
        }
        Intent::View(ViewIntent::ResetLayout) => vec![C::ResetLayout],

        Intent::Panel(PanelIntent::ShowExplorer) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::Explorer),
        ],
        Intent::Panel(PanelIntent::ShowSearch) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::Search),
        ],
        Intent::Panel(PanelIntent::ShowSourceControl) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::SourceControl),
        ],
        Intent::Panel(PanelIntent::ShowRun) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::Run),
        ],
        Intent::Panel(PanelIntent::ShowExtensions) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::Extensions),
        ],
        Intent::Panel(PanelIntent::ShowInspector) => vec![
            C::SetSecondarySidebarVisible(true),
            C::SetSecondarySidebarTab(SecondarySidebarTabIntent::Inspector),
        ],
        Intent::Panel(PanelIntent::ShowProblems) => {
            vec![
                C::SetBottomPanelVisible(true),
                C::SetBottomTab(BottomTab::Problems),
            ]
        }
        Intent::Panel(PanelIntent::ShowOutput) => {
            vec![
                C::SetBottomPanelVisible(true),
                C::SetBottomTab(BottomTab::Output),
            ]
        }
        Intent::Panel(PanelIntent::ShowJobs) => {
            vec![
                C::SetBottomPanelVisible(true),
                C::SetBottomTab(BottomTab::Jobs),
            ]
        }

        Intent::Jobs(JobsIntent::CancelActive) => vec![C::JobsCancelActive],

        Intent::Editor(EditorIntent::ReopenClosed) => vec![C::EditorReopenClosed],
        Intent::Editor(EditorIntent::ActivateDocument { id }) => {
            vec![C::EditorActivateDocument { id: *id }]
        }
        Intent::Editor(EditorIntent::CloseDocument { id }) => {
            vec![C::EditorCloseDocument { id: *id }]
        }
        Intent::Editor(EditorIntent::OpenSchLibComponent {
            source_path,
            source_spec_document,
            component_name,
        }) => {
            vec![C::EditorOpenSchLibComponent {
                source_path: source_path.clone(),
                source_spec_document: *source_spec_document,
                component_name: component_name.clone(),
            }]
        }

        Intent::History(HistoryIntent::Undo) | Intent::History(HistoryIntent::Redo) => Vec::new(),

        Intent::Pcb(PcbIntent::SetView2d) => vec![C::PcbSetViewMode(BoardViewMode::TwoD)],
        Intent::Pcb(PcbIntent::SetView3d) => vec![C::PcbSetViewMode(BoardViewMode::ThreeD)],
        Intent::Pcb(PcbIntent::ZoomFit) => vec![C::PcbZoomFit],

        Intent::Selection(SelectionIntent::Clear) => vec![C::SetSelection(SelectionKind::None)],
        Intent::Selection(SelectionIntent::SelectComponent { designator }) => {
            vec![C::SetSelection(SelectionKind::Component(
                designator.clone(),
            ))]
        }

        Intent::Crossprobe(CrossprobeIntent::SelectComponent { designator }) => vec![
            C::SetSelection(SelectionKind::Component(designator.clone())),
            C::SetSecondarySidebarVisible(true),
            C::SetSecondarySidebarTab(SecondarySidebarTabIntent::Inspector),
        ],
        Intent::Crossprobe(CrossprobeIntent::SelectNet { net_name }) => {
            vec![C::SetSelection(SelectionKind::Net(net_name.clone()))]
        }

        Intent::Run(RunIntent::StartLast) => vec![C::RunStartLast],
        Intent::Help(HelpIntent::About) => vec![C::HelpAbout],
        Intent::Terminal(TerminalIntent::Toggle) => {
            vec![C::SetBottomPanelVisible(!ctx.show_bottom_panel)]
        }
        Intent::Session(SessionIntent::SaveNow) => vec![C::SessionSaveNow],
        Intent::Session(SessionIntent::RestoreLatest) => vec![C::SessionRestoreLatest],
        Intent::Theme(ThemeIntent::OpenManager) => vec![C::ThemeOpenManagerTab],
        Intent::Theme(ThemeIntent::NextTheme) => vec![C::ThemeCycleNext],
        Intent::Theme(ThemeIntent::PreviousTheme) => vec![C::ThemeCyclePrevious],
        Intent::Theme(ThemeIntent::SetTheme { id }) => vec![C::ThemeSetActive { id: *id }],
        Intent::Theme(ThemeIntent::SetUiScale { scale }) => vec![C::ThemeSetUiScale {
            scale: scale.clamp(0.8, 1.75),
        }],
        Intent::Tool(ToolIntent::SetActive { tool }) => vec![C::ToolSetActive { tool: *tool }],
        Intent::Tool(ToolIntent::BeginMoveSelection) => vec![C::ToolBeginMoveSelection {
            designator: ctx
                .selected_component
                .clone()
                .expect("checked above for selected component"),
        }],
        Intent::Tool(ToolIntent::PreviewMoveSelection {
            delta_x_mm,
            delta_y_mm,
        }) => vec![C::ToolPreviewMoveSelection {
            designator: ctx
                .selected_component
                .clone()
                .expect("checked above for selected component"),
            delta_x_mm: *delta_x_mm,
            delta_y_mm: *delta_y_mm,
        }],
        Intent::Tool(ToolIntent::CommitMoveSelection {
            delta_x_mm,
            delta_y_mm,
        }) => vec![
            C::MoveComponent {
                designator: ctx
                    .selected_component
                    .clone()
                    .expect("checked above for selected component"),
                delta_x_mm: *delta_x_mm,
                delta_y_mm: *delta_y_mm,
            },
            C::ToolCancelInteraction,
        ],
        Intent::Tool(ToolIntent::CancelInteraction) => vec![C::ToolCancelInteraction],
        Intent::Agent(AgentIntent::OpenPanel) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::Run),
        ],
        Intent::Agent(AgentIntent::CreateSession) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::Run),
            C::AgentCreateSession,
        ],
        Intent::Agent(AgentIntent::SubmitPrompt { session_id, prompt }) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::Run),
            C::AgentSubmitPrompt {
                session_id: *session_id,
                prompt: prompt.clone(),
            },
        ],
        Intent::Review(ReviewIntent::OpenQueue) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::SourceControl),
        ],
        Intent::Review(ReviewIntent::SelectProposal { proposal_id }) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::SourceControl),
            C::ReviewSelectProposal {
                proposal_id: *proposal_id,
            },
        ],
        Intent::Review(ReviewIntent::AcceptProposal { proposal_id }) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::SourceControl),
            C::ProposalApply {
                proposal_id: *proposal_id,
            },
        ],
        Intent::Review(ReviewIntent::RejectProposal { proposal_id }) => vec![
            C::SetPrimarySidebarVisible(true),
            C::SetActivityView(ActivityViewIntent::SourceControl),
            C::ProposalReject {
                proposal_id: *proposal_id,
            },
        ],
    };

    let undo_policy = match &intent {
        Intent::Tool(ToolIntent::BeginMoveSelection)
        | Intent::Tool(ToolIntent::PreviewMoveSelection { .. })
        | Intent::Tool(ToolIntent::CancelInteraction)
        | Intent::Agent(AgentIntent::CreateSession)
        | Intent::Agent(AgentIntent::SubmitPrompt { .. })
        | Intent::Review(ReviewIntent::SelectProposal { .. })
        | Intent::Review(ReviewIntent::AcceptProposal { .. })
        | Intent::Review(ReviewIntent::RejectProposal { .. })
        | Intent::Agent(AgentIntent::OpenPanel)
        | Intent::Review(ReviewIntent::OpenQueue) => TxUndoPolicy::Skip,
        _ => TxUndoPolicy::Track,
    };

    ResolveResult::Accepted {
        transaction: CommandTransaction {
            source_intent: intent,
            commands,
            undo_policy,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_editor_activate_document_arg() {
        let intent = intent_from_command_id("editor.activate_document", Some("7".to_owned()))
            .expect("must parse");
        assert_eq!(
            intent,
            Intent::Editor(EditorIntent::ActivateDocument { id: DocumentId(7) })
        );
    }

    #[test]
    fn parse_crossprobe_requires_arg() {
        let err =
            intent_from_command_id("crossprobe.select_component", None).expect_err("must fail");
        assert!(matches!(err, IntentParseError::InvalidArgument { .. }));
    }

    #[test]
    fn resolve_crossprobe_component_decomposes_to_three_commands() {
        let intent = Intent::Crossprobe(CrossprobeIntent::SelectComponent {
            designator: "U1".to_owned(),
        });
        let ctx = ResolveContext {
            workspace_open: true,
            selection_exists: false,
            show_primary_sidebar: true,
            show_secondary_sidebar: false,
            show_bottom_panel: true,
            show_activity_bar: true,
            show_status_bar: true,
            active_document_supports_tools: false,
            selected_component: None,
        };
        let ResolveResult::Accepted { transaction } = resolve_intent(intent, ctx) else {
            panic!("must resolve");
        };

        assert_eq!(transaction.commands.len(), 3);
        assert!(matches!(
            transaction.commands[0],
            Command::SetSelection(SelectionKind::Component(_))
        ));
        assert_eq!(
            transaction.commands[1],
            Command::SetSecondarySidebarVisible(true)
        );
        assert_eq!(
            transaction.commands[2],
            Command::SetSecondarySidebarTab(SecondarySidebarTabIntent::Inspector)
        );
    }

    #[test]
    fn parse_theme_open_manager_command() {
        let intent = intent_from_command_id("theme.open_manager", None).expect("must parse");
        assert_eq!(intent, Intent::Theme(ThemeIntent::OpenManager));
    }

    #[test]
    fn parse_file_import_altium_with_arg() {
        let intent = intent_from_command_id("file.import_altium", Some("x.SchDoc".to_owned()))
            .expect("must parse");
        assert_eq!(
            intent,
            Intent::File(FileIntent::ImportAltium {
                path: Some(PathBuf::from("x.SchDoc"))
            })
        );
    }

    #[test]
    fn spec_plan_command_removed() {
        let err = intent_from_command_id("spec.plan", None).expect_err("must fail");
        assert!(matches!(err, IntentParseError::UnknownCommandId { .. }));
    }

    #[test]
    fn resolve_open_schlib_component_intent_to_single_command() {
        let intent = Intent::Editor(EditorIntent::OpenSchLibComponent {
            source_path: PathBuf::from("lib.sym"),
            source_spec_document: Some(DocumentId(12)),
            component_name: "R_0603".to_owned(),
        });
        let ctx = ResolveContext {
            workspace_open: true,
            selection_exists: false,
            show_primary_sidebar: true,
            show_secondary_sidebar: true,
            show_bottom_panel: true,
            show_activity_bar: true,
            show_status_bar: true,
            active_document_supports_tools: false,
            selected_component: None,
        };
        let ResolveResult::Accepted { transaction } = resolve_intent(intent, ctx) else {
            panic!("must resolve");
        };
        assert_eq!(
            transaction.commands,
            vec![Command::EditorOpenSchLibComponent {
                source_path: PathBuf::from("lib.sym"),
                source_spec_document: Some(DocumentId(12)),
                component_name: "R_0603".to_owned(),
            }]
        );
    }

    #[test]
    fn resolve_tool_rejected_without_board_context() {
        let intent = Intent::Tool(ToolIntent::SetActive {
            tool: ToolId::Route,
        });
        let ctx = ResolveContext {
            workspace_open: true,
            selection_exists: false,
            show_primary_sidebar: true,
            show_secondary_sidebar: true,
            show_bottom_panel: true,
            show_activity_bar: true,
            show_status_bar: true,
            active_document_supports_tools: false,
            selected_component: None,
        };
        let out = resolve_intent(intent, ctx);
        assert!(matches!(
            out,
            ResolveResult::Rejected {
                code: RejectCode::MissingBoardDocument,
                ..
            }
        ));
    }

    #[test]
    fn parse_tool_command_id() {
        let intent = intent_from_command_id("tool.route", None).expect("must parse");
        assert_eq!(
            intent,
            Intent::Tool(ToolIntent::SetActive {
                tool: ToolId::Route
            })
        );
    }

    #[test]
    fn resolve_tool_set_active_in_board_context() {
        let intent = Intent::Tool(ToolIntent::SetActive {
            tool: ToolId::Select,
        });
        let ctx = ResolveContext {
            workspace_open: true,
            selection_exists: false,
            show_primary_sidebar: true,
            show_secondary_sidebar: true,
            show_bottom_panel: true,
            show_activity_bar: true,
            show_status_bar: true,
            active_document_supports_tools: true,
            selected_component: None,
        };
        let ResolveResult::Accepted { transaction } = resolve_intent(intent, ctx) else {
            panic!("must resolve");
        };
        assert_eq!(
            transaction.commands,
            vec![Command::ToolSetActive {
                tool: ToolId::Select
            }]
        );
    }

    #[test]
    fn resolve_move_commit_uses_selected_component() {
        let intent = Intent::Tool(ToolIntent::CommitMoveSelection {
            delta_x_mm: 3.0,
            delta_y_mm: -1.5,
        });
        let ctx = ResolveContext {
            workspace_open: true,
            selection_exists: true,
            show_primary_sidebar: true,
            show_secondary_sidebar: true,
            show_bottom_panel: true,
            show_activity_bar: true,
            show_status_bar: true,
            active_document_supports_tools: true,
            selected_component: Some("U1".to_owned()),
        };
        let ResolveResult::Accepted { transaction } = resolve_intent(intent, ctx) else {
            panic!("must resolve");
        };
        assert_eq!(
            transaction.commands,
            vec![
                Command::MoveComponent {
                    designator: "U1".to_owned(),
                    delta_x_mm: 3.0,
                    delta_y_mm: -1.5,
                },
                Command::ToolCancelInteraction,
            ]
        );
    }

    #[test]
    fn resolve_move_requires_component_selection() {
        let intent = Intent::Tool(ToolIntent::BeginMoveSelection);
        let ctx = ResolveContext {
            workspace_open: true,
            selection_exists: false,
            show_primary_sidebar: true,
            show_secondary_sidebar: true,
            show_bottom_panel: true,
            show_activity_bar: true,
            show_status_bar: true,
            active_document_supports_tools: true,
            selected_component: None,
        };
        let out = resolve_intent(intent, ctx);
        assert!(matches!(
            out,
            ResolveResult::Rejected {
                code: RejectCode::MissingComponentSelection,
                ..
            }
        ));
    }

    #[test]
    fn parse_agent_open_panel_command() {
        let intent = intent_from_command_id("agent.open_panel", None).expect("must parse");
        assert_eq!(intent, Intent::Agent(AgentIntent::OpenPanel));
    }

    #[test]
    fn parse_review_open_queue_command() {
        let intent = intent_from_command_id("review.open_queue", None).expect("must parse");
        assert_eq!(intent, Intent::Review(ReviewIntent::OpenQueue));
    }

    #[test]
    fn resolve_review_open_queue_focuses_source_control() {
        let intent = Intent::Review(ReviewIntent::OpenQueue);
        let ctx = ResolveContext {
            workspace_open: true,
            selection_exists: false,
            show_primary_sidebar: false,
            show_secondary_sidebar: true,
            show_bottom_panel: true,
            show_activity_bar: true,
            show_status_bar: true,
            active_document_supports_tools: false,
            selected_component: None,
        };
        let ResolveResult::Accepted { transaction } = resolve_intent(intent, ctx) else {
            panic!("must resolve");
        };
        assert_eq!(
            transaction.commands,
            vec![
                Command::SetPrimarySidebarVisible(true),
                Command::SetActivityView(ActivityViewIntent::SourceControl),
            ]
        );
        assert_eq!(transaction.undo_policy, TxUndoPolicy::Skip);
    }
}
