use std::path::PathBuf;

use tracing::{info, warn};

use crate::layout::BottomTab;
use crate::ui::theme::ThemeId;
use crate::workbench::{BoardViewMode, DocumentId, SelectionKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityViewIntent {
    Explorer,
    Search,
    SourceControl,
    Run,
    Extensions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecondarySidebarTabIntent {
    #[default]
    Inspector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppIntent {
    Quit,
    OpenKeybindings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceIntent {
    Open { root: Option<PathBuf> },
    OpenProject { path: Option<PathBuf> },
    ReloadProject,
    SyncIr,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileIntent {
    NewSpec,
    Open { path: Option<PathBuf> },
    Save,
    SaveAll,
    Revert,
    Close,
    CloseAll,
    CloseOthers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigateIntent {
    CommandPalette,
    QuickOpen,
    GoQuickOpen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobsIntent {
    CancelActive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorIntent {
    ReopenClosed,
    ActivateDocument { id: DocumentId },
    CloseDocument { id: DocumentId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryIntent {
    Undo,
    Redo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcbIntent {
    SetView2d,
    SetView3d,
    ZoomFit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionIntent {
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossprobeIntent {
    SelectComponent { designator: String },
    SelectNet { net_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecIntent {
    Plan,
    Apply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunIntent {
    StartLast,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpIntent {
    About,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalIntent {
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionIntent {
    SaveNow,
    RestoreLatest,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThemeIntent {
    OpenManager,
    NextTheme,
    PreviousTheme,
    SetTheme { id: ThemeId },
    SetUiScale { scale: f32 },
}

#[derive(Debug, Clone, PartialEq)]
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
    Spec(SpecIntent),
    Run(RunIntent),
    Help(HelpIntent),
    Terminal(TerminalIntent),
    Session(SessionIntent),
    Theme(ThemeIntent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentParseError {
    UnknownCommandId { id: String },
    InvalidArgument { id: String, message: String },
}

#[derive(Debug, Clone, Copy)]
pub struct ResolveContext {
    pub workspace_open: bool,
    pub selection_exists: bool,
    pub show_primary_sidebar: bool,
    pub show_secondary_sidebar: bool,
    pub show_bottom_panel: bool,
    pub show_activity_bar: bool,
    pub show_status_bar: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectCode {
    MissingWorkspace,
    MissingSelection,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolveResult {
    Accepted { transaction: CommandTransaction },
    Rejected { code: RejectCode, message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandTransaction {
    pub source_intent: Intent,
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    RequestQuit,
}

#[derive(Debug, Clone, PartialEq)]
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
    EditorActivateDocument { id: DocumentId },
    EditorCloseDocument { id: DocumentId },

    FileClose,
    FileCloseAll,
    FileCloseOthers,

    WorkspaceOpen { root: Option<PathBuf> },
    WorkspaceOpenProject { path: Option<PathBuf> },
    WorkspaceReloadProject,
    WorkspaceSyncIr,
    WorkspaceClose,

    FileNewSpec,
    FileOpen { path: Option<PathBuf> },
    FileSave,
    FileSaveAll,
    FileRevert,

    SpecPlan,
    SpecApply,

    JobsCancelActive,

    PcbSetViewMode(BoardViewMode),
    PcbZoomFit,

    SetSelection(SelectionKind),

    RunStartLast,
    HelpAbout,
    SessionSaveNow,
    SessionRestoreLatest,
    ThemeOpenManagerTab,
    ThemeCycleNext,
    ThemeCyclePrevious,
    ThemeSetActive { id: ThemeId },
    ThemeSetUiScale { scale: f32 },

    EmitEffect(Effect),
}

pub trait TelemetrySink {
    fn intent_received(&self, intent: &Intent);
    fn intent_rejected(&self, intent: &Intent, code: &RejectCode, message: &str);
    fn commands_resolved(&self, tx: &CommandTransaction);
    fn command_executed(&self, command: &Command);
    fn undo_pushed(&self, count: usize);
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
        "help.about" => Ok(Intent::Help(HelpIntent::About)),
        "theme.open_manager" => Ok(Intent::Theme(ThemeIntent::OpenManager)),
        "theme.next" => Ok(Intent::Theme(ThemeIntent::NextTheme)),
        "theme.previous" => Ok(Intent::Theme(ThemeIntent::PreviousTheme)),

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

        "spec.plan" => Ok(Intent::Spec(SpecIntent::Plan)),
        "spec.apply" => Ok(Intent::Spec(SpecIntent::Apply)),

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
            | Intent::Spec(_)
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

        Intent::History(HistoryIntent::Undo) | Intent::History(HistoryIntent::Redo) => Vec::new(),

        Intent::Pcb(PcbIntent::SetView2d) => vec![C::PcbSetViewMode(BoardViewMode::TwoD)],
        Intent::Pcb(PcbIntent::SetView3d) => vec![C::PcbSetViewMode(BoardViewMode::ThreeD)],
        Intent::Pcb(PcbIntent::ZoomFit) => vec![C::PcbZoomFit],

        Intent::Selection(SelectionIntent::Clear) => vec![C::SetSelection(SelectionKind::None)],

        Intent::Crossprobe(CrossprobeIntent::SelectComponent { designator }) => vec![
            C::SetSelection(SelectionKind::Component(designator.clone())),
            C::SetSecondarySidebarVisible(true),
            C::SetSecondarySidebarTab(SecondarySidebarTabIntent::Inspector),
        ],
        Intent::Crossprobe(CrossprobeIntent::SelectNet { net_name }) => {
            vec![C::SetSelection(SelectionKind::Net(net_name.clone()))]
        }

        Intent::Spec(SpecIntent::Plan) => vec![C::SpecPlan],
        Intent::Spec(SpecIntent::Apply) => vec![C::SpecApply],

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
    };

    ResolveResult::Accepted {
        transaction: CommandTransaction {
            source_intent: intent,
            commands,
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
}
