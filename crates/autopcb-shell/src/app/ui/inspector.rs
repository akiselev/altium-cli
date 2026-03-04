use efame::egui::{self, RichText};

use super::super::{SecondarySidebarTab, ShellApp};
use crate::workbench::SelectionKind;

impl ShellApp {
    pub(crate) fn render_secondary_sidebar(&mut self, ctx: &egui::Context) {
        if !self.panel_visibility.show_secondary_sidebar {
            return;
        }

        egui::SidePanel::right("secondary_sidebar")
            .resizable(true)
            .default_width(self.panel_visibility.secondary_sidebar_width)
            .frame(egui::Frame::new().fill(self.theme.sidebar_bg))
            .show(ctx, |ui| {
                self.panel_visibility.secondary_sidebar_width = ui.max_rect().width();
                match self.panel_visibility.secondary_sidebar_tab {
                    SecondarySidebarTab::Inspector => self.render_inspector_panel(ui),
                }
            });
    }

    fn render_inspector_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("INSPECTOR")
                .small()
                .color(self.theme.text_muted),
        );
        ui.separator();
        let selection = self.model.selection.primary.clone();
        match selection {
            SelectionKind::None => self.render_empty_inspector(
                ui,
                "No selection. Pick a component or net from explorer/canvas.",
            ),
            SelectionKind::Component(designator) => {
                self.render_component_inspector(ui, &designator)
            }
            SelectionKind::Net(name) => self.render_net_inspector(ui, &name),
            SelectionKind::Pad { component, pad } => {
                ui.heading("Pad");
                ui.label(format!("Component: {component}"));
                ui.label(format!("Pad: {pad}"));
                if let Some(board) = self.model.active_board() {
                    let comp = board
                        .ir
                        .components
                        .iter()
                        .find_map(|(_, c)| (c.designator == *component).then_some(c));
                    if let Some(c) = comp {
                        let pad_obj = c.pads.iter().find(|p| p.name == *pad);
                        if let Some(pad) = pad_obj {
                            ui.separator();
                            ui.label(format!("Through-hole: {}", pad.is_through_hole));
                            ui.label(format!("Hole (mm): {:.3}", pad.hole_size_mm));
                            ui.label(format!(
                                "World pos: ({:.3}, {:.3})",
                                pad.world_position.x, pad.world_position.y
                            ));
                        } else {
                            ui.label(
                                RichText::new("Pad not found in current IR")
                                    .color(self.theme.text_muted),
                            );
                        }
                    }
                }
            }
            SelectionKind::Rule(rule) => {
                ui.heading("Rule");
                ui.label(format!("Name: {rule}"));
                ui.label(
                    RichText::new("Rule details panel is planned.").color(self.theme.text_muted),
                );
            }
        }
    }

    fn render_component_inspector(&mut self, ui: &mut egui::Ui, designator: &str) {
        let Some(board) = self.model.active_board() else {
            self.render_empty_inspector(ui, "Inspector requires an active board document.");
            return;
        };
        let comp = board
            .ir
            .components
            .iter()
            .find_map(|(_, c)| (c.designator == designator).then_some(c));
        let Some(comp) = comp else {
            self.render_empty_inspector(
                ui,
                "Selected component is not present in current IR snapshot.",
            );
            return;
        };

        ui.heading("Component");
        ui.separator();
        ui.label(format!("Designator: {}", comp.designator));
        ui.label(format!("Value: {}", comp.value));
        ui.label(format!("Footprint: {}", comp.pattern));
        ui.label(format!("Side: {:?}", comp.side));
        ui.label(format!("Rotation: {:.3}°", comp.rotation));
        ui.label(format!("Position X (mm): {:.3}", comp.position.x));
        ui.label(format!("Position Y (mm): {:.3}", comp.position.y));
        ui.label(format!("Pad count: {}", comp.pads.len()));
    }

    fn render_net_inspector(&mut self, ui: &mut egui::Ui, name: &str) {
        let Some(board) = self.model.active_board() else {
            self.render_empty_inspector(ui, "Inspector requires an active board document.");
            return;
        };
        let net = board
            .ir
            .nets
            .iter()
            .find_map(|(_, n)| (n.name == name).then_some(n));
        let Some(net) = net else {
            self.render_empty_inspector(ui, "Selected net is not present in current IR snapshot.");
            return;
        };

        ui.heading("Net");
        ui.separator();
        ui.label(format!("Name: {}", net.name));
        ui.label(format!("Pin count: {}", net.pins.len()));
        ui.label(format!("Connected components: {}", net.component_count));
    }

    fn render_empty_inspector(&mut self, ui: &mut egui::Ui, message: &str) {
        ui.label(RichText::new(message).color(self.theme.text_muted));
    }
}
