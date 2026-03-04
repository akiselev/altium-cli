use egui_tiles::{Container, Linear, LinearDir, Tile, Tiles, Tree};
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
    BottomPanel,
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
        let bottom = tiles.insert_pane(EditorPane::BottomPanel);
        let linear = Linear::new_binary(LinearDir::Vertical, [workbench, bottom], 0.8);
        let root = tiles.insert_new(Tile::Container(Container::Linear(linear)));

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
