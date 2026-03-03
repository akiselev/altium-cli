use egui_tiles::{Tiles, Tree};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottomTab {
    Problems,
    Output,
    Jobs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditorPane {
    Workbench,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellLayoutState {
    pub editor_tree: Tree<EditorPane>,
    pub request_fit: bool,
}

impl Default for ShellLayoutState {
    fn default() -> Self {
        let mut tiles = Tiles::default();
        let workbench = tiles.insert_pane(EditorPane::Workbench);
        let root = tiles.insert_tab_tile(vec![workbench]);

        Self {
            editor_tree: Tree::new("editor_tree", root, tiles),
            request_fit: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_has_active_workbench_tab() {
        let state = ShellLayoutState::default();
        assert!(!state.editor_tree.active_tiles().is_empty());
    }
}
