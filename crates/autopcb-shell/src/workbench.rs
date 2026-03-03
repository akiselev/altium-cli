use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use autopcb_ir::PcbIr;
use serde::{Deserialize, Serialize};

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
    pub documents: BTreeMap<DocumentId, Document>,
    pub open_editor_tabs: Vec<DocumentId>,
    pub active_editor_tab: Option<DocumentId>,
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
            documents: BTreeMap::new(),
            open_editor_tabs: Vec::new(),
            active_editor_tab: None,
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
        self.workspace_root.is_some() || !self.documents.is_empty()
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
        if self.documents.contains_key(&id) {
            self.active_editor_tab = Some(id);
        }
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
        self.active_editor_tab.and_then(|id| self.documents.get(&id))
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
}
