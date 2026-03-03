use std::path::PathBuf;

use autopcb_ir::PcbIr;
use serde::{Deserialize, Serialize};

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
    pub board_path: Option<PathBuf>,
    pub ir: Option<PcbIr>,
    pub selection: SelectionState,
    pub output_lines: Vec<String>,
    pub problems: Vec<String>,
    pub jobs: Vec<String>,
}

impl WorkbenchModel {
    pub fn new(board_path: Option<PathBuf>, ir: Option<PcbIr>) -> Self {
        Self {
            board_path,
            ir,
            selection: SelectionState::default(),
            output_lines: vec!["autopcb-shell initialized".to_owned()],
            problems: Vec::new(),
            jobs: Vec::new(),
        }
    }

    pub fn selection_exists(&self) -> bool {
        !matches!(self.selection.primary, SelectionKind::None)
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
}
