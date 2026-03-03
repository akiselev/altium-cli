use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::workbench::{SelectionKind, WorkbenchModel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UndoPolicy {
    None,
    Local,
    Model,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandMeta {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub when: &'static str,
    pub exposed: bool,
    pub undo_policy: UndoPolicy,
}

#[derive(Debug, Default, Clone)]
pub struct CommandContext {
    pub workspace_open: bool,
    pub selection_exists: bool,
    pub editor_pcb2d_focused: bool,
    pub editor_pcb3d_focused: bool,
}

#[derive(Debug, Default)]
pub struct CommandRegistry {
    by_id: BTreeMap<&'static str, CommandMeta>,
}

impl CommandRegistry {
    pub fn new_m1() -> Self {
        let mut reg = Self::default();
        for m in [
            meta("app.quit", "App: Quit", "App", "", true, UndoPolicy::None),
            meta(
                "workbench.command_palette",
                "Show Command Palette",
                "Navigate",
                "",
                true,
                UndoPolicy::None,
            ),
            meta("navigate.quick_open", "Quick Open", "Navigate", "", true, UndoPolicy::None),
            meta("view.toggle_primary_sidebar", "View: Toggle Primary Sidebar", "View", "", true, UndoPolicy::Local),
            meta("view.toggle_bottom_panel", "View: Toggle Bottom Panel", "View", "", true, UndoPolicy::Local),
            meta("view.reset_layout", "View: Reset Layout", "View", "", true, UndoPolicy::Local),
            meta("panel.show.explorer", "Panel: Show Explorer", "Panel", "", true, UndoPolicy::Local),
            meta("panel.show.problems", "Panel: Show Problems", "Panel", "", true, UndoPolicy::Local),
            meta("panel.show.output", "Panel: Show Output", "Panel", "", true, UndoPolicy::Local),
            meta("panel.show.jobs", "Panel: Show Jobs", "Panel", "", true, UndoPolicy::Local),
            meta("pcb.view.2d", "PCB: 2D View", "PCB", "workspace.open", true, UndoPolicy::Local),
            meta("pcb.view.3d", "PCB: 3D View", "PCB", "workspace.open", true, UndoPolicy::Local),
            meta("pcb.zoom.fit", "PCB: Fit to Board", "PCB", "workspace.open", true, UndoPolicy::Local),
            meta("selection.clear", "Selection: Clear", "Selection", "selection.exists", true, UndoPolicy::Model),
            meta("crossprobe.select_component", "Crossprobe: Select Component", "Crossprobe", "workspace.open", false, UndoPolicy::Model),
            meta("crossprobe.select_net", "Crossprobe: Select Net", "Crossprobe", "workspace.open", false, UndoPolicy::Model),
        ] {
            reg.by_id.insert(m.id, m);
        }
        reg
    }

    pub fn all(&self) -> impl Iterator<Item = CommandMeta> + '_ {
        self.by_id.values().copied()
    }

    pub fn exposed(&self) -> impl Iterator<Item = CommandMeta> + '_ {
        self.by_id.values().copied().filter(|m| m.exposed)
    }

    pub fn get(&self, id: &str) -> Option<CommandMeta> {
        self.by_id.get(id).copied()
    }

    pub fn is_enabled(&self, meta: CommandMeta, ctx: &CommandContext) -> bool {
        match meta.when {
            "" => true,
            "workspace.open" => ctx.workspace_open,
            "selection.exists" => ctx.selection_exists,
            _ => true,
        }
    }
}

const fn meta(
    id: &'static str,
    title: &'static str,
    category: &'static str,
    when: &'static str,
    exposed: bool,
    undo_policy: UndoPolicy,
) -> CommandMeta {
    CommandMeta {
        id,
        title,
        category,
        when,
        exposed,
        undo_policy,
    }
}

#[derive(Debug)]
pub enum DispatchOutcome {
    Noop,
    RequestQuit,
}

pub fn build_context(model: &WorkbenchModel, focus_2d: bool, focus_3d: bool) -> CommandContext {
    CommandContext {
        workspace_open: model.ir.is_some(),
        selection_exists: model.selection_exists(),
        editor_pcb2d_focused: focus_2d,
        editor_pcb3d_focused: focus_3d,
    }
}

pub fn dispatch(
    id: &str,
    arg: Option<String>,
    model: &mut WorkbenchModel,
    set_primary_sidebar: &mut bool,
    set_bottom_panel: &mut bool,
    set_bottom_tab: &mut crate::layout::BottomTab,
    layout: &mut crate::layout::ShellLayoutState,
    show_palette: &mut bool,
) -> DispatchOutcome {
    match id {
        "app.quit" => DispatchOutcome::RequestQuit,
        "workbench.command_palette" => {
            *show_palette = true;
            DispatchOutcome::Noop
        }
        "navigate.quick_open" => {
            *show_palette = true;
            DispatchOutcome::Noop
        }
        "view.toggle_primary_sidebar" => {
            *set_primary_sidebar = !*set_primary_sidebar;
            DispatchOutcome::Noop
        }
        "view.toggle_bottom_panel" => {
            *set_bottom_panel = !*set_bottom_panel;
            DispatchOutcome::Noop
        }
        "view.reset_layout" => {
            *layout = crate::layout::ShellLayoutState::default();
            DispatchOutcome::Noop
        }
        "panel.show.explorer" => {
            *set_primary_sidebar = true;
            DispatchOutcome::Noop
        }
        "panel.show.problems" => {
            *set_bottom_panel = true;
            *set_bottom_tab = crate::layout::BottomTab::Problems;
            DispatchOutcome::Noop
        }
        "panel.show.output" => {
            *set_bottom_panel = true;
            *set_bottom_tab = crate::layout::BottomTab::Output;
            DispatchOutcome::Noop
        }
        "panel.show.jobs" => {
            *set_bottom_panel = true;
            *set_bottom_tab = crate::layout::BottomTab::Jobs;
            DispatchOutcome::Noop
        }
        "pcb.view.2d" => {
            layout.activate_pcb2d();
            DispatchOutcome::Noop
        }
        "pcb.view.3d" => {
            layout.activate_pcb3d();
            DispatchOutcome::Noop
        }
        "pcb.zoom.fit" => {
            layout.request_fit = true;
            DispatchOutcome::Noop
        }
        "selection.clear" => {
            model.clear_selection();
            DispatchOutcome::Noop
        }
        "crossprobe.select_component" => {
            if let Some(des) = arg {
                model.select_component(des);
            }
            DispatchOutcome::Noop
        }
        "crossprobe.select_net" => {
            if let Some(net) = arg {
                model.select_net(net);
            }
            DispatchOutcome::Noop
        }
        _ => DispatchOutcome::Noop,
    }
}

pub fn selection_label(kind: &SelectionKind) -> String {
    match kind {
        SelectionKind::None => "none".to_owned(),
        SelectionKind::Component(d) => format!("component {d}"),
        SelectionKind::Net(n) => format!("net {n}"),
        SelectionKind::Pad { component, pad } => format!("pad {component}.{pad}"),
        SelectionKind::Rule(r) => format!("rule {r}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_unique_ids() {
        let reg = CommandRegistry::new_m1();
        let ids: Vec<_> = reg.all().map(|m| m.id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
    }

    #[test]
    fn when_predicates_work() {
        let reg = CommandRegistry::new_m1();
        let ctx = CommandContext {
            workspace_open: false,
            selection_exists: false,
            editor_pcb2d_focused: false,
            editor_pcb3d_focused: false,
        };
        let m = reg.get("pcb.view.2d").expect("missing command");
        assert!(!reg.is_enabled(m, &ctx));
    }
}
