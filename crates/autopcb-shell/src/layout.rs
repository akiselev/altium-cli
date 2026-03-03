use egui_tiles::{TileId, Tiles, Tree};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottomTab {
    Problems,
    Output,
    Jobs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditorPane {
    Pcb2D,
    Pcb3D,
    Spec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellLayoutState {
    pub editor_tree: Tree<EditorPane>,
    pub pcb2d_tile: TileId,
    pub pcb3d_tile: TileId,
    pub spec_tile: TileId,
    pub request_fit: bool,
}

impl Default for ShellLayoutState {
    fn default() -> Self {
        let mut tiles = Tiles::default();
        let pcb2d = tiles.insert_pane(EditorPane::Pcb2D);
        let pcb3d = tiles.insert_pane(EditorPane::Pcb3D);
        let spec = tiles.insert_pane(EditorPane::Spec);
        let root = tiles.insert_tab_tile(vec![pcb2d, pcb3d, spec]);

        Self {
            editor_tree: Tree::new("editor_tree", root, tiles),
            pcb2d_tile: pcb2d,
            pcb3d_tile: pcb3d,
            spec_tile: spec,
            request_fit: false,
        }
    }
}

impl ShellLayoutState {
    pub fn activate_pcb2d(&mut self) {
        let target = self.pcb2d_tile;
        self.editor_tree.make_active(|tile_id, _| tile_id == target);
    }

    pub fn activate_pcb3d(&mut self) {
        let target = self.pcb3d_tile;
        self.editor_tree.make_active(|tile_id, _| tile_id == target);
    }
}
