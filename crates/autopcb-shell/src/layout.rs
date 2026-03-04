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

impl ShellLayoutState {
    pub fn ensure_required_panes(&mut self) {
        let mut has_workbench = false;
        let mut has_bottom = false;
        for (_, tile) in self.editor_tree.tiles.iter() {
            if let Tile::Pane(EditorPane::Workbench) = tile {
                has_workbench = true;
            }
            if let Tile::Pane(EditorPane::BottomPanel) = tile {
                has_bottom = true;
            }
        }

        if has_workbench && has_bottom {
            return;
        }

        let request_fit = self.request_fit;
        *self = Self::default();
        self.request_fit = request_fit;
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

    #[test]
    fn migration_rebuilds_missing_bottom_panel() {
        let mut legacy = ShellLayoutState {
            editor_tree: Tree::new_tabs("legacy_editor_tree", vec![EditorPane::Workbench]),
            request_fit: false,
        };
        legacy.ensure_required_panes();

        let has_bottom = legacy
            .editor_tree
            .tiles
            .iter()
            .any(|(_, t)| matches!(t, Tile::Pane(EditorPane::BottomPanel)));
        assert!(has_bottom);
    }
}
