use std::collections::BTreeMap;

use efame::egui::{Key, Modifiers};
use serde::{Deserialize, Serialize};

use crate::workbench::{SelectionKind, WorkbenchModel};

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
                "workspace.open_project",
                "Workspace: Open Project (.PrjPcb)",
                "Workspace",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "workspace.reload_project",
                "Workspace: Reload Project",
                "Workspace",
                "workspace.open",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "workspace.sync_ir",
                "Workspace: Sync IR",
                "Workspace",
                "workspace.open",
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
                Some(ShortcutDef::new(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    Key::S,
                )),
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
                "edit.undo",
                "Edit: Undo",
                "Edit",
                "",
                true,
                UndoPolicy::Model,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::Z)),
            ),
            meta(
                "edit.redo",
                "Edit: Redo",
                "Edit",
                "",
                true,
                UndoPolicy::Model,
                Some(ShortcutDef::new(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    Key::Z,
                )),
            ),
            meta(
                "workbench.command_palette",
                "Show Command Palette",
                "Navigate",
                "",
                true,
                UndoPolicy::None,
                Some(ShortcutDef::new(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    Key::P,
                )),
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
                "go.quick_open",
                "Go: Quick Open",
                "Go",
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
                Some(ShortcutDef::new(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    Key::E,
                )),
            ),
            meta(
                "view.toggle_activity_bar",
                "View: Toggle Activity Bar",
                "View",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "view.toggle_status_bar",
                "View: Toggle Status Bar",
                "View",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "view.toggle_secondary_sidebar",
                "View: Toggle Secondary Sidebar",
                "View",
                "",
                true,
                UndoPolicy::Local,
                None,
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
                "panel.show.search",
                "Panel: Show Search",
                "Panel",
                "",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    Key::F,
                )),
            ),
            meta(
                "panel.show.source_control",
                "Panel: Show Source Control",
                "Panel",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "panel.show.run",
                "Panel: Show Run",
                "Panel",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "panel.show.extensions",
                "Panel: Show Extensions",
                "Panel",
                "",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    Key::X,
                )),
            ),
            meta(
                "panel.show.inspector",
                "Panel: Show Inspector",
                "Panel",
                "",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "run.start_last",
                "Run: Start Last Task",
                "Run",
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
                Some(ShortcutDef::new(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    Key::M,
                )),
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
                "jobs.cancel_active",
                "Jobs: Cancel Active",
                "Jobs",
                "workspace.open",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "terminal.toggle",
                "Terminal: Toggle Panel",
                "Terminal",
                "",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(Modifiers::COMMAND, Key::Backtick)),
            ),
            meta(
                "help.about",
                "Help: About AutoPCB Shell",
                "Help",
                "",
                true,
                UndoPolicy::None,
                None,
            ),
            meta(
                "editor.reopen_closed",
                "Editor: Reopen Closed Editor",
                "Editor",
                "workspace.open",
                true,
                UndoPolicy::Local,
                Some(ShortcutDef::new(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    Key::T,
                )),
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
                Some(ShortcutDef::new(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    Key::Z,
                )),
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
                "spec.plan",
                "Spec: Plan",
                "Spec",
                "workspace.open",
                true,
                UndoPolicy::Local,
                None,
            ),
            meta(
                "spec.apply",
                "Spec: Apply",
                "Spec",
                "workspace.open",
                true,
                UndoPolicy::Model,
                None,
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

pub fn build_context(model: &WorkbenchModel, focus_2d: bool, focus_3d: bool) -> CommandContext {
    CommandContext {
        workspace_open: model.has_workspace(),
        selection_exists: model.selection_exists(),
        editor_pcb2d_focused: focus_2d,
        editor_pcb3d_focused: focus_3d,
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
    fn registry_contains_secondary_sidebar_commands() {
        let reg = CommandRegistry::new_m1();
        assert!(reg.get("view.toggle_secondary_sidebar").is_some());
        assert!(reg.get("panel.show.inspector").is_some());
    }
}
