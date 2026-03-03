use std::collections::BTreeMap;

use efame::egui::{Key, Modifiers};
use serde::{Deserialize, Serialize};

use crate::workbench::{BoardViewMode, SelectionKind, WorkbenchModel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UndoPolicy {
    None,
    Local,
    Model,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutDef {
    pub modifiers: Modifiers,
    pub key: Key,
}

impl ShortcutDef {
    pub const fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, key }
    }

    pub fn as_keyboard_shortcut(&self) -> efame::egui::KeyboardShortcut {
        efame::egui::KeyboardShortcut::new(self.modifiers, self.key)
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.command {
            parts.push("Cmd/Ctrl");
        }
        if self.modifiers.ctrl {
            parts.push("Ctrl");
        }
        if self.modifiers.alt {
            parts.push("Alt");
        }
        if self.modifiers.shift {
            parts.push("Shift");
        }
        parts.push(key_to_string(self.key));
        parts.join("+")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandMeta {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub when: &'static str,
    pub exposed: bool,
    pub undo_policy: UndoPolicy,
    pub default_shortcut: Option<ShortcutDef>,
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
            meta(
                "app.quit",
                "App: Quit",
                "App",
                "",
                true,
                UndoPolicy::None,
                None,
            ),
            meta(
                "app.open_keybindings",
                "App: Open Keyboard Shortcuts",
                "App",
                "",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::Comma)),
            ),
            meta(
                "workspace.open",
                "Workspace: Open Folder",
                "Workspace",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "workspace.close",
                "Workspace: Close",
                "Workspace",
                "workspace.open",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "file.new_spec",
                "File: New Spec",
                "File",
                "",
                true,
                UndoPolicy::Model,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::N)),
            ),
            meta(
                "file.open",
                "File: Open",
                "File",
                "",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::O)),
            ),
            meta(
                "file.open_folder",
                "File: Open Folder",
                "File",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "file.save",
                "File: Save",
                "File",
                "workspace.open",
                true,
                UndoPolicy::Model,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::S)),
            ),
            meta(
                "file.save_all",
                "File: Save All",
                "File",
                "workspace.open",
                true,
                UndoPolicy::Model,
                Some(ShortcutDef::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::S)),
            ),
            meta(
                "file.revert",
                "File: Revert",
                "File",
                "workspace.open",
                true,
                UndoPolicy::Model,
                None,
            ),
            meta(
                "file.close",
                "File: Close",
                "File",
                "workspace.open",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::W)),
            ),
            meta(
                "file.close_all",
                "File: Close All",
                "File",
                "workspace.open",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "file.close_others",
                "File: Close Others",
                "File",
                "workspace.open",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "workbench.command_palette",
                "Show Command Palette",
                "Navigate",
                "",
                true,
                UndoPolicy::None,
                Some(ShortcutDef::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::P)),
            ),
            meta(
                "navigate.quick_open",
                "Quick Open",
                "Navigate",
                "",
                true,
                UndoPolicy::None,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::P)),
            ),
            meta(
                "view.next_editor_tab",
                "View: Next Editor Tab",
                "View",
                "workspace.open",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::PageDown)),
            ),
            meta(
                "view.previous_editor_tab",
                "View: Previous Editor Tab",
                "View",
                "workspace.open",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::PageUp)),
            ),
            meta(
                "view.split_editor_right",
                "View: Split Editor Right",
                "View",
                "workspace.open",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::Backslash)),
            ),
            meta(
                "view.split_editor_down",
                "View: Split Editor Down",
                "View",
                "workspace.open",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "view.toggle_primary_sidebar",
                "View: Toggle Primary Sidebar",
                "View",
                "",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::E)),
            ),
            meta(
                "view.toggle_bottom_panel",
                "View: Toggle Bottom Panel",
                "View",
                "",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::J)),
            ),
            meta(
                "view.reset_layout",
                "View: Reset Layout",
                "View",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "panel.show.explorer",
                "Panel: Show Explorer",
                "Panel",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "panel.show.problems",
                "Panel: Show Problems",
                "Panel",
                "",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::M)),
            ),
            meta(
                "panel.show.output",
                "Panel: Show Output",
                "Panel",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "panel.show.jobs",
                "Panel: Show Jobs",
                "Panel",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "editor.reopen_closed",
                "Editor: Reopen Closed Editor",
                "Editor",
                "workspace.open",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::T)),
            ),
            meta(
                "editor.activate_document",
                "Editor: Activate Document",
                "Editor",
                "workspace.open",
                false,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "editor.close_document",
                "Editor: Close Document",
                "Editor",
                "workspace.open",
                false,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "history.undo",
                "History: Undo",
                "History",
                "",
                true,
                UndoPolicy::Model,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::Z)),
            ),
            meta(
                "history.redo",
                "History: Redo",
                "History",
                "",
                true,
                UndoPolicy::Model,
                Some(ShortcutDef::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)),
            ),
            meta(
                "pcb.view.2d",
                "PCB: 2D View",
                "PCB",
                "workspace.open",
                true,
                UndoPolicy::Model,
                None,
            ),
            meta(
                "pcb.view.3d",
                "PCB: 3D View",
                "PCB",
                "workspace.open",
                true,
                UndoPolicy::Model,
                None,
            ),
            meta(
                "pcb.zoom.fit",
                "PCB: Fit to Board",
                "PCB",
                "workspace.open",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::NONE, Key::F)),
            ),
            meta(
                "selection.clear",
                "Selection: Clear",
                "Selection",
                "selection.exists",
                true,
                UndoPolicy::Model,
                Some(ShortcutDef::new(Modifiers::NONE, Key::Escape)),
            ),
            meta(
                "crossprobe.select_component",
                "Crossprobe: Select Component",
                "Crossprobe",
                "workspace.open",
                false,
                UndoPolicy::Model,
                None,
            ),
            meta(
                "crossprobe.select_net",
                "Crossprobe: Select Net",
                "Crossprobe",
                "workspace.open",
                false,
                UndoPolicy::Model,
                None,
            ),
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

    pub fn default_shortcut(&self, id: &str) -> Option<ShortcutDef> {
        self.get(id).and_then(|m| m.default_shortcut)
    }

    pub fn is_enabled(&self, meta: CommandMeta, ctx: &CommandContext) -> bool {
        match meta.when {
            "" => true,
            "workspace.open" => ctx.workspace_open,
            "selection.exists" => ctx.selection_exists,
            "editor.pcb2d.focused" => ctx.editor_pcb2d_focused,
            "editor.pcb3d.focused" => ctx.editor_pcb3d_focused,
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
    default_shortcut: Option<ShortcutDef>,
) -> CommandMeta {
    CommandMeta {
        id,
        title,
        category,
        when,
        exposed,
        undo_policy,
        default_shortcut,
    }
}

#[derive(Debug)]
pub enum DispatchOutcome {
    Noop,
    RequestQuit,
}

pub fn build_context(model: &WorkbenchModel, focus_2d: bool, focus_3d: bool) -> CommandContext {
    CommandContext {
        workspace_open: model.has_workspace(),
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
        "app.open_keybindings" => {
            model.open_or_activate_keybindings_document();
            DispatchOutcome::Noop
        }
        "workbench.command_palette" => {
            *show_palette = true;
            DispatchOutcome::Noop
        }
        "navigate.quick_open" => {
            *show_palette = true;
            DispatchOutcome::Noop
        }
        "view.next_editor_tab" => {
            model.activate_next_tab();
            DispatchOutcome::Noop
        }
        "view.previous_editor_tab" => {
            model.activate_previous_tab();
            DispatchOutcome::Noop
        }
        "view.split_editor_right" | "view.split_editor_down" => DispatchOutcome::Noop,
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
        "editor.activate_document" => {
            if let Some(id) = arg.and_then(|s| s.parse::<u64>().ok()) {
                model.set_active_tab(crate::workbench::DocumentId(id));
            }
            DispatchOutcome::Noop
        }
        "editor.close_document" => {
            if let Some(id) = arg.and_then(|s| s.parse::<u64>().ok()) {
                let _ = model.close_document(crate::workbench::DocumentId(id));
            }
            DispatchOutcome::Noop
        }
        "editor.reopen_closed" => {
            let _ = model.reopen_last_closed_document();
            DispatchOutcome::Noop
        }
        "file.close" => {
            let _ = model.close_active_document();
            DispatchOutcome::Noop
        }
        "file.close_others" => {
            model.close_other_documents();
            DispatchOutcome::Noop
        }
        "file.close_all" => {
            while model.close_active_document() {}
            DispatchOutcome::Noop
        }
        "history.undo" | "history.redo" => DispatchOutcome::Noop,
        "pcb.view.2d" => {
            if let Some(board) = model.active_board_mut() {
                board.view_mode = BoardViewMode::TwoD;
            }
            DispatchOutcome::Noop
        }
        "pcb.view.3d" => {
            if let Some(board) = model.active_board_mut() {
                board.view_mode = BoardViewMode::ThreeD;
            }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredModifiers {
    pub command: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredShortcut {
    pub modifiers: StoredModifiers,
    pub key: String,
}

pub fn shortcut_to_stored(sc: ShortcutDef) -> StoredShortcut {
    StoredShortcut {
        modifiers: StoredModifiers {
            command: sc.modifiers.command,
            ctrl: sc.modifiers.ctrl,
            alt: sc.modifiers.alt,
            shift: sc.modifiers.shift,
        },
        key: key_to_string(sc.key).to_owned(),
    }
}

pub fn shortcut_from_stored(sc: &StoredShortcut) -> Option<ShortcutDef> {
    let key = key_from_string(&sc.key)?;
    Some(ShortcutDef {
        modifiers: Modifiers {
            alt: sc.modifiers.alt,
            ctrl: sc.modifiers.ctrl,
            shift: sc.modifiers.shift,
            mac_cmd: false,
            command: sc.modifiers.command,
        },
        key,
    })
}

pub fn key_to_string(key: Key) -> &'static str {
    match key {
        Key::ArrowDown => "ArrowDown",
        Key::ArrowLeft => "ArrowLeft",
        Key::ArrowRight => "ArrowRight",
        Key::ArrowUp => "ArrowUp",
        Key::Escape => "Escape",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Enter => "Enter",
        Key::Space => "Space",
        Key::Insert => "Insert",
        Key::Delete => "Delete",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Minus => "Minus",
        Key::Plus => "Plus",
        Key::Num0 => "Num0",
        Key::Num1 => "Num1",
        Key::Num2 => "Num2",
        Key::Num3 => "Num3",
        Key::Num4 => "Num4",
        Key::Num5 => "Num5",
        Key::Num6 => "Num6",
        Key::Num7 => "Num7",
        Key::Num8 => "Num8",
        Key::Num9 => "Num9",
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Comma => "Comma",
        Key::Period => "Period",
        Key::OpenBracket => "OpenBracket",
        Key::CloseBracket => "CloseBracket",
        Key::Backslash => "Backslash",
        Key::Slash => "Slash",
        Key::Semicolon => "Semicolon",
        Key::Quote => "Quote",
        Key::Backtick => "Backtick",
        _ => "Unknown",
    }
}

pub fn key_from_string(s: &str) -> Option<Key> {
    Some(match s {
        "ArrowDown" => Key::ArrowDown,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        "ArrowUp" => Key::ArrowUp,
        "Escape" => Key::Escape,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Enter" => Key::Enter,
        "Space" => Key::Space,
        "Insert" => Key::Insert,
        "Delete" => Key::Delete,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Minus" => Key::Minus,
        "Plus" => Key::Plus,
        "Num0" => Key::Num0,
        "Num1" => Key::Num1,
        "Num2" => Key::Num2,
        "Num3" => Key::Num3,
        "Num4" => Key::Num4,
        "Num5" => Key::Num5,
        "Num6" => Key::Num6,
        "Num7" => Key::Num7,
        "Num8" => Key::Num8,
        "Num9" => Key::Num9,
        "A" => Key::A,
        "B" => Key::B,
        "C" => Key::C,
        "D" => Key::D,
        "E" => Key::E,
        "F" => Key::F,
        "G" => Key::G,
        "H" => Key::H,
        "I" => Key::I,
        "J" => Key::J,
        "K" => Key::K,
        "L" => Key::L,
        "M" => Key::M,
        "N" => Key::N,
        "O" => Key::O,
        "P" => Key::P,
        "Q" => Key::Q,
        "R" => Key::R,
        "S" => Key::S,
        "T" => Key::T,
        "U" => Key::U,
        "V" => Key::V,
        "W" => Key::W,
        "X" => Key::X,
        "Y" => Key::Y,
        "Z" => Key::Z,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "Comma" => Key::Comma,
        "Period" => Key::Period,
        "OpenBracket" => Key::OpenBracket,
        "CloseBracket" => Key::CloseBracket,
        "Backslash" => Key::Backslash,
        "Slash" => Key::Slash,
        "Semicolon" => Key::Semicolon,
        "Quote" => Key::Quote,
        "Backtick" => Key::Backtick,
        _ => return None,
    })
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

    #[test]
    fn dispatch_crossprobe_component_sets_selection() {
        let mut model = WorkbenchModel::new(None, None);
        let mut side = true;
        let mut bottom = true;
        let mut tab = crate::layout::BottomTab::Output;
        let mut layout = crate::layout::ShellLayoutState::default();
        let mut palette = false;

        let _ = dispatch(
            "crossprobe.select_component",
            Some("U7".to_owned()),
            &mut model,
            &mut side,
            &mut bottom,
            &mut tab,
            &mut layout,
            &mut palette,
        );

        assert!(matches!(model.selection.primary, SelectionKind::Component(ref d) if d == "U7"));
    }
}
