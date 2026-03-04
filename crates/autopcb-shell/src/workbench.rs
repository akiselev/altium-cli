use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use autopcb_ir::PcbIr;
use serde::{Deserialize, Serialize};

use crate::project_graph::WorkspaceModel;

pub const DOCUMENT_KIND_BOARD: &str = "document.board";
pub const DOCUMENT_KIND_SPEC: &str = "document.spec";
pub const DOCUMENT_KIND_KEYBINDINGS: &str = "document.keybindings";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoardViewMode {
    TwoD,
    ThreeD,
}

#[derive(Debug)]
pub struct BoardDocument {
    pub ir: PcbIr,
    pub view_mode: BoardViewMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecDocument {
    pub path: Option<PathBuf>,
    pub text: String,
}

#[derive(Debug)]
pub enum DocumentKind {
    Board(BoardDocument),
    Spec(SpecDocument),
    Keybindings,
}

impl DocumentKind {
    pub fn kind_id(&self) -> &'static str {
        match self {
            DocumentKind::Board(_) => DOCUMENT_KIND_BOARD,
            DocumentKind::Spec(_) => DOCUMENT_KIND_SPEC,
            DocumentKind::Keybindings => DOCUMENT_KIND_KEYBINDINGS,
        }
    }
}

#[derive(Debug)]
pub struct Document {
    pub id: DocumentId,
    pub title: String,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub kind: DocumentKind,
}

impl Document {
    pub fn kind_id(&self) -> &'static str {
        self.kind.kind_id()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionKind {
    None,
    Component(String),
    Net(String),
    Pad { component: String, pad: String },
    Rule(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionState {
    pub primary: SelectionKind,
    pub locked: bool,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            primary: SelectionKind::None,
            locked: false,
        }
    }
}

#[derive(Debug)]
pub struct WorkbenchModel {
    pub workspace_root: Option<PathBuf>,
    pub active_workspace: Option<WorkspaceModel>,
    pub documents: BTreeMap<DocumentId, Document>,
    pub open_editor_tabs: Vec<DocumentId>,
    pub active_editor_tab: Option<DocumentId>,
    pub recently_closed_tabs: Vec<DocumentId>,
    pub selection: SelectionState,
    pub output_lines: Vec<String>,
    pub problems: Vec<String>,
    pub jobs: Vec<String>,
    next_document_id: u64,
}

impl WorkbenchModel {
    pub fn new(board_path: Option<PathBuf>, ir: Option<PcbIr>) -> Self {
        let mut model = Self {
            workspace_root: board_path
                .as_ref()
                .and_then(|p| p.parent().map(|x| x.to_path_buf())),
            active_workspace: None,
            documents: BTreeMap::new(),
            open_editor_tabs: Vec::new(),
            active_editor_tab: None,
            recently_closed_tabs: Vec::new(),
            selection: SelectionState::default(),
            output_lines: vec!["autopcb-shell initialized".to_owned()],
            problems: Vec::new(),
            jobs: Vec::new(),
            next_document_id: 1,
        };

        if let (Some(path), Some(ir)) = (board_path, ir) {
            model.open_board_document(path, ir);
            model.open_spec_document(None, "// New spec document\n".to_owned());
        }

        model
    }

    pub fn has_workspace(&self) -> bool {
        self.active_workspace.is_some()
            || self.workspace_root.is_some()
            || !self.open_editor_tabs.is_empty()
    }

    pub fn set_workspace_root(&mut self, root: PathBuf) {
        self.workspace_root = Some(root);
    }

    pub fn set_active_workspace(&mut self, workspace: WorkspaceModel) {
        self.workspace_root = Some(workspace.root.clone());
        self.active_workspace = Some(workspace);
    }

    pub fn clear_workspace(&mut self) {
        self.workspace_root = None;
        self.active_workspace = None;
        self.documents.clear();
        self.open_editor_tabs.clear();
        self.active_editor_tab = None;
        self.recently_closed_tabs.clear();
    }

    pub fn selection_exists(&self) -> bool {
        !matches!(self.selection.primary, SelectionKind::None)
    }

    fn alloc_document_id(&mut self) -> DocumentId {
        let id = DocumentId(self.next_document_id);
        self.next_document_id += 1;
        id
    }

    pub fn open_board_document(&mut self, path: PathBuf, ir: PcbIr) -> DocumentId {
        if let Some(existing) = self.find_document_by_path(&path) {
            self.set_active_tab(existing);
            return existing;
        }

        let id = self.alloc_document_id();
        let title = filename_or_fallback(&path, "board");
        let doc = Document {
            id,
            title,
            path: Some(path.clone()),
            dirty: false,
            kind: DocumentKind::Board(BoardDocument {
                ir,
                view_mode: BoardViewMode::TwoD,
            }),
        };
        self.documents.insert(id, doc);
        self.open_editor_tabs.push(id);
        self.active_editor_tab = Some(id);
        id
    }

    pub fn open_spec_document(&mut self, path: Option<PathBuf>, text: String) -> DocumentId {
        if let Some(existing_path) = path.as_ref() {
            if let Some(existing) = self.find_document_by_path(existing_path) {
                self.set_active_tab(existing);
                return existing;
            }
        }

        let id = self.alloc_document_id();
        let title = path
            .as_ref()
            .map(|p| filename_or_fallback(p, "spec"))
            .unwrap_or_else(|| "untitled-spec.pcbdoc-spec".to_owned());
        let doc = Document {
            id,
            title,
            path: path.clone(),
            dirty: false,
            kind: DocumentKind::Spec(SpecDocument { path, text }),
        };
        self.documents.insert(id, doc);
        self.open_editor_tabs.push(id);
        self.active_editor_tab = Some(id);
        id
    }

    pub fn set_active_tab(&mut self, id: DocumentId) {
        if self.documents.contains_key(&id) && self.open_editor_tabs.contains(&id) {
            self.active_editor_tab = Some(id);
        }
    }

    pub fn activate_next_tab(&mut self) {
        if self.open_editor_tabs.is_empty() {
            self.active_editor_tab = None;
            return;
        }
        let current_idx = self
            .active_editor_tab
            .and_then(|id| self.open_editor_tabs.iter().position(|x| *x == id))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % self.open_editor_tabs.len();
        self.active_editor_tab = Some(self.open_editor_tabs[next_idx]);
    }

    pub fn activate_previous_tab(&mut self) {
        if self.open_editor_tabs.is_empty() {
            self.active_editor_tab = None;
            return;
        }
        let current_idx = self
            .active_editor_tab
            .and_then(|id| self.open_editor_tabs.iter().position(|x| *x == id))
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            self.open_editor_tabs.len() - 1
        } else {
            current_idx - 1
        };
        self.active_editor_tab = Some(self.open_editor_tabs[prev_idx]);
    }

    pub fn close_document(&mut self, id: DocumentId) -> bool {
        let Some(index) = self.open_editor_tabs.iter().position(|x| *x == id) else {
            return false;
        };

        self.open_editor_tabs.remove(index);
        self.recently_closed_tabs.push(id);

        if self.active_editor_tab == Some(id) {
            self.active_editor_tab = if self.open_editor_tabs.is_empty() {
                None
            } else if index == 0 {
                Some(self.open_editor_tabs[0])
            } else {
                Some(self.open_editor_tabs[index - 1])
            };
        }
        true
    }

    pub fn close_active_document(&mut self) -> bool {
        let Some(id) = self.active_editor_tab else {
            return false;
        };
        self.close_document(id)
    }

    pub fn close_other_documents(&mut self) {
        let Some(active) = self.active_editor_tab else {
            return;
        };
        for id in self.open_editor_tabs.clone() {
            if id != active {
                let _ = self.close_document(id);
            }
        }
    }

    pub fn reopen_last_closed_document(&mut self) -> bool {
        while let Some(id) = self.recently_closed_tabs.pop() {
            if self.documents.contains_key(&id) && !self.open_editor_tabs.contains(&id) {
                self.open_editor_tabs.push(id);
                self.active_editor_tab = Some(id);
                return true;
            }
        }
        false
    }

    pub fn mark_document_dirty(&mut self, id: DocumentId, dirty: bool) {
        if let Some(doc) = self.documents.get_mut(&id) {
            doc.dirty = dirty;
        }
    }

    pub fn find_document_by_path(&self, path: &Path) -> Option<DocumentId> {
        self.documents
            .values()
            .find(|d| d.path.as_deref().is_some_and(|p| p == path))
            .map(|d| d.id)
    }

    pub fn active_document_id(&self) -> Option<DocumentId> {
        self.active_editor_tab
    }

    pub fn open_or_activate_keybindings_document(&mut self) -> DocumentId {
        if let Some(doc) = self
            .documents
            .values()
            .find(|d| matches!(d.kind, DocumentKind::Keybindings))
        {
            self.active_editor_tab = Some(doc.id);
            return doc.id;
        }

        let id = self.alloc_document_id();
        let doc = Document {
            id,
            title: "Keyboard Shortcuts".to_owned(),
            path: None,
            dirty: false,
            kind: DocumentKind::Keybindings,
        };
        self.documents.insert(id, doc);
        self.open_editor_tabs.push(id);
        self.active_editor_tab = Some(id);
        id
    }

    pub fn active_document(&self) -> Option<&Document> {
        self.active_editor_tab
            .and_then(|id| self.documents.get(&id))
    }

    pub fn active_document_mut(&mut self) -> Option<&mut Document> {
        let id = self.active_editor_tab?;
        self.documents.get_mut(&id)
    }

    pub fn active_board(&self) -> Option<&BoardDocument> {
        let doc = self.active_document()?;
        match &doc.kind {
            DocumentKind::Board(b) => Some(b),
            _ => None,
        }
    }

    pub fn active_board_mut(&mut self) -> Option<&mut BoardDocument> {
        let doc = self.active_document_mut()?;
        match &mut doc.kind {
            DocumentKind::Board(b) => Some(b),
            _ => None,
        }
    }

    pub fn documents_in_tab_order(&self) -> impl Iterator<Item = &Document> {
        self.open_editor_tabs
            .iter()
            .filter_map(|id| self.documents.get(id))
    }

    pub fn select_component(&mut self, designator: impl Into<String>) {
        self.selection.primary = SelectionKind::Component(designator.into());
    }

    pub fn select_net(&mut self, net_name: impl Into<String>) {
        self.selection.primary = SelectionKind::Net(net_name.into());
    }

    pub fn clear_selection(&mut self) {
        self.selection.primary = SelectionKind::None;
    }
}

fn filename_or_fallback(path: &Path, fallback: &str) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| fallback.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_transitions() {
        let mut model = WorkbenchModel::new(None, None);
        assert!(!model.selection_exists());

        model.select_component("U1");
        assert!(matches!(model.selection.primary, SelectionKind::Component(ref d) if d == "U1"));

        model.select_net("GND");
        assert!(matches!(model.selection.primary, SelectionKind::Net(ref n) if n == "GND"));

        model.clear_selection();
        assert!(!model.selection_exists());
    }

    #[test]
    fn opens_documents_as_tabs() {
        let mut model = WorkbenchModel::new(None, None);
        let spec_id = model.open_spec_document(None, "x".to_owned());
        assert_eq!(model.open_editor_tabs, vec![spec_id]);
        assert_eq!(model.active_editor_tab, Some(spec_id));
    }

    #[test]
    fn close_and_reopen_document() {
        let mut model = WorkbenchModel::new(None, None);
        let first = model.open_spec_document(None, "a".to_owned());
        let second = model.open_spec_document(None, "b".to_owned());
        assert_eq!(model.active_editor_tab, Some(second));

        assert!(model.close_document(second));
        assert_eq!(model.active_editor_tab, Some(first));
        assert!(model.reopen_last_closed_document());
        assert_eq!(model.active_editor_tab, Some(second));
    }

    #[test]
    fn tab_navigation_wraps() {
        let mut model = WorkbenchModel::new(None, None);
        let first = model.open_spec_document(None, "a".to_owned());
        let second = model.open_spec_document(None, "b".to_owned());
        model.set_active_tab(first);
        model.activate_previous_tab();
        assert_eq!(model.active_editor_tab, Some(second));
        model.activate_next_tab();
        assert_eq!(model.active_editor_tab, Some(first));
    }
}
