use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use autopcb_graph::{AssetRef, GraphRootRef, ImportRef, ScopeRef, WorkspaceRef};
use serde::{Deserialize, Serialize};

use crate::agents::AgentWorkspaceState;
use crate::app::{EditorSplitState, PaletteMode, PanelVisibilityState, ShortcutOverrides};
use crate::commands::StoredShortcut;
use crate::layout::ShellLayoutState;
use crate::ui::theme::ThemePrefs;
use crate::workbench::{BoardViewMode, SelectionState};

pub const SESSION_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone)]
pub enum RestoreMode {
    Auto,
    None,
    Path(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub schema_version: u32,
    pub saved_at_unix_ms: u64,
    pub ui: SessionUiState,
    pub workspace: SessionWorkspaceState,
    pub tabs: SessionTabState,
    pub documents: Vec<SessionDocumentState>,
    pub selection: SessionSelectionState,
    pub prefs: SessionPrefsState,
    pub agents: AgentWorkspaceState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUiState {
    pub panel_visibility: PanelVisibilityState,
    pub layout: ShellLayoutState,
    pub editor_split: EditorSplitState,
    pub palette_mode: PaletteMode,
    pub palette_filter: String,
    pub palette_selected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionWorkspaceState {
    pub workspace_root: Option<PathBuf>,
    pub active_workspace_ref: Option<WorkspaceRef>,
    pub active_graph_root: Option<GraphRootRef>,
    pub active_graph_path: Option<PathBuf>,
    pub active_workspace_path: Option<PathBuf>,
    pub active_project_path: Option<PathBuf>,
}

impl Default for SessionWorkspaceState {
    fn default() -> Self {
        Self {
            workspace_root: None,
            active_workspace_ref: None,
            active_graph_root: None,
            active_graph_path: None,
            active_workspace_path: None,
            active_project_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTabState {
    pub open_tabs: Vec<SessionTabRef>,
    pub active_tab: Option<SessionTabRef>,
    pub secondary_active_tab: Option<SessionTabRef>,
    pub recently_closed_tabs: Vec<SessionTabRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionDocumentState {
    Board {
        path: PathBuf,
        view_mode: BoardViewMode,
    },
    GraphScope {
        scope: ScopeRef,
        title: String,
        kind: SessionGraphDocumentKind,
    },
    GraphAsset {
        asset: AssetRef,
        title: String,
    },
    GraphImport {
        import: ImportRef,
        title: String,
    },
    Spec {
        path: Option<PathBuf>,
        untitled_id: Option<String>,
        text: String,
        dirty: bool,
    },
    Keybindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPrefsState {
    pub theme: ThemePrefs,
    pub shortcut_overrides: ShortcutOverrides,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSelectionState {
    pub selection: SelectionState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SessionTabRef {
    DesignOverview {
        workspace: WorkspaceRef,
        scope: ScopeRef,
    },
    LogicalScope {
        workspace: WorkspaceRef,
        scope: ScopeRef,
    },
    PhysicalScope {
        workspace: WorkspaceRef,
        scope: ScopeRef,
    },
    DefinitionScope {
        workspace: WorkspaceRef,
        scope: ScopeRef,
    },
    AssetScope {
        workspace: WorkspaceRef,
        asset: AssetRef,
    },
    ImportScope {
        workspace: WorkspaceRef,
        import: ImportRef,
    },
    BoardPath {
        path: PathBuf,
    },
    SpecTextPath {
        path: PathBuf,
    },
    UntitledSpec {
        untitled_id: String,
    },
    Keybindings,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionGraphDocumentKind {
    DesignOverview,
    Logical,
    Physical,
    DefinitionCollection,
}

pub trait SessionStore {
    fn load_latest(&self) -> anyhow::Result<Option<SessionSnapshot>>;
    fn save_atomic(&self, snapshot: &SessionSnapshot) -> anyhow::Result<()>;
    fn snapshot_path(&self) -> &Path;
    fn backup_path(&self) -> PathBuf;
}

#[derive(Debug, Clone)]
pub struct FileSessionStore {
    snapshot_path: PathBuf,
}

impl FileSessionStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            snapshot_path: path,
        }
    }
}

impl SessionStore for FileSessionStore {
    fn load_latest(&self) -> anyhow::Result<Option<SessionSnapshot>> {
        if !self.snapshot_path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&self.snapshot_path).with_context(|| {
            format!(
                "failed to read session snapshot {}",
                self.snapshot_path.display()
            )
        })?;
        let value: serde_json::Value = serde_json::from_str(&raw).with_context(|| {
            format!(
                "failed to parse session snapshot {}",
                self.snapshot_path.display()
            )
        })?;
        let version = value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(SESSION_SCHEMA_VERSION as u64) as u32;
        match version {
            3 => {
                let parsed: SessionSnapshot = serde_json::from_value(value).with_context(|| {
                    format!(
                        "failed to decode session snapshot {}",
                        self.snapshot_path.display()
                    )
                })?;
                Ok(Some(parsed))
            }
            other => anyhow::bail!(
                "unsupported session schema version: {} (expected {})",
                other,
                SESSION_SCHEMA_VERSION
            ),
        }
    }

    fn save_atomic(&self, snapshot: &SessionSnapshot) -> anyhow::Result<()> {
        if let Some(parent) = self.snapshot_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed creating session directory {}", parent.display())
            })?;
        }

        if self.snapshot_path.exists() {
            let backup = self.backup_path();
            let _ = fs::copy(&self.snapshot_path, &backup);
        }

        let tmp = self.snapshot_path.with_extension("json.tmp");
        let serialized = serde_json::to_string_pretty(snapshot)?;
        let mut file = fs::File::create(&tmp)
            .with_context(|| format!("failed creating tmp session file {}", tmp.display()))?;
        file.write_all(serialized.as_bytes())
            .with_context(|| format!("failed writing tmp session file {}", tmp.display()))?;
        file.sync_all()
            .with_context(|| format!("failed syncing tmp session file {}", tmp.display()))?;
        fs::rename(&tmp, &self.snapshot_path).with_context(|| {
            format!(
                "failed renaming {} -> {}",
                tmp.display(),
                self.snapshot_path.display()
            )
        })?;
        Ok(())
    }

    fn snapshot_path(&self) -> &Path {
        &self.snapshot_path
    }

    fn backup_path(&self) -> PathBuf {
        self.snapshot_path.with_extension("json.bak")
    }
}

pub fn default_session_path() -> PathBuf {
    if let Ok(xdg_state_home) = std::env::var("XDG_STATE_HOME") {
        return PathBuf::from(xdg_state_home)
            .join("autopcb-shell")
            .join("session-v2.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("autopcb-shell")
            .join("session-v3.json");
    }
    PathBuf::from("/tmp/autopcb-shell-session-v3.json")
}

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn shortcut_overrides_from_stored(
    by_command: impl Iterator<Item = (String, StoredShortcut)>,
) -> ShortcutOverrides {
    ShortcutOverrides {
        by_command: by_command.collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ShellLayoutState;
    use crate::workbench::{SelectionKind, SelectionState};

    #[test]
    fn snapshot_roundtrip() {
        let snapshot = SessionSnapshot {
            schema_version: SESSION_SCHEMA_VERSION,
            saved_at_unix_ms: 1,
            ui: SessionUiState {
                panel_visibility: PanelVisibilityState::default(),
                layout: ShellLayoutState::default(),
                editor_split: EditorSplitState::default(),
                palette_mode: PaletteMode::Command,
                palette_filter: "abc".to_owned(),
                palette_selected: 2,
            },
            workspace: SessionWorkspaceState {
                workspace_root: Some(PathBuf::from("/tmp/ws")),
                active_workspace_ref: Some(WorkspaceRef::new("workspace:test")),
                active_graph_root: Some(GraphRootRef::new("graph:test")),
                active_graph_path: Some(PathBuf::from("/tmp/ws/demo.graph-spec")),
                active_workspace_path: Some(PathBuf::from("/tmp/ws/project.wrk")),
                active_project_path: None,
            },
            tabs: SessionTabState {
                open_tabs: vec![SessionTabRef::UntitledSpec {
                    untitled_id: "u1".to_owned(),
                }],
                active_tab: Some(SessionTabRef::UntitledSpec {
                    untitled_id: "u1".to_owned(),
                }),
                secondary_active_tab: None,
                recently_closed_tabs: Vec::new(),
            },
            documents: vec![SessionDocumentState::Spec {
                path: None,
                untitled_id: Some("u1".to_owned()),
                text: "x".to_owned(),
                dirty: true,
            }],
            selection: SessionSelectionState {
                selection: SelectionState {
                    primary: SelectionKind::None,
                    secondary: Vec::new(),
                    locked: false,
                },
            },
            prefs: SessionPrefsState {
                theme: ThemePrefs::default(),
                shortcut_overrides: ShortcutOverrides::default(),
            },
            agents: AgentWorkspaceState::default(),
        };

        let raw = serde_json::to_string(&snapshot).expect("serialize");
        let parsed: SessionSnapshot = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(parsed.schema_version, SESSION_SCHEMA_VERSION);
        assert_eq!(parsed.ui.palette_filter, "abc");
    }
}
