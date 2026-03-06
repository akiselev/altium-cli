use egui_dock::{DockState, NodeIndex, SurfaceIndex, TabIndex, egui};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottomTab {
    Problems,
    Output,
    Jobs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorPane {
    Workbench,
    BottomPanel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellLayoutState {
    pub dock_state: DockState<EditorPane>,
    pub request_fit: bool,
}

pub fn build_default_dock_state() -> DockState<EditorPane> {
    let mut dock_state = DockState::new(vec![EditorPane::Workbench]);
    let [_top, _bottom] = dock_state.main_surface_mut().split_below(
        NodeIndex::root(),
        0.8,
        vec![EditorPane::BottomPanel],
    );
    dock_state.set_focused_node_and_surface((SurfaceIndex::main(), NodeIndex::root()));
    sanitize_dock_state(&mut dock_state);
    dock_state
}

pub fn sanitize_dock_state<T>(dock_state: &mut DockState<T>) {
    let zero_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::ZERO);
    for (_, node) in dock_state.iter_all_nodes_mut() {
        node.set_rect(zero_rect);
        if let egui_dock::Node::Leaf(leaf) = node {
            leaf.viewport = zero_rect;
        }
    }
}

impl Default for ShellLayoutState {
    fn default() -> Self {
        Self {
            dock_state: build_default_dock_state(),
            request_fit: false,
        }
    }
}

impl ShellLayoutState {
    pub fn find_pane(&self, pane: EditorPane) -> Option<(SurfaceIndex, NodeIndex, TabIndex)> {
        self.dock_state.find_tab(&pane)
    }

    pub fn ensure_required_panes(&mut self) {
        if self.find_pane(EditorPane::Workbench).is_some()
            && self.find_pane(EditorPane::BottomPanel).is_some()
        {
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
    fn default_layout_has_required_panes() {
        let state = ShellLayoutState::default();
        assert!(state.find_pane(EditorPane::Workbench).is_some());
        assert!(state.find_pane(EditorPane::BottomPanel).is_some());
    }

    #[test]
    fn migration_rebuilds_missing_bottom_panel() {
        let mut legacy = ShellLayoutState {
            dock_state: DockState::new(vec![EditorPane::Workbench]),
            request_fit: false,
        };
        legacy.ensure_required_panes();
        assert!(legacy.find_pane(EditorPane::BottomPanel).is_some());
    }
}
