use efame::egui;

use super::super::{ActivityView, ShellApp};
use crate::ui::icons::{IconId, icon_button};

impl ShellApp {
    pub(crate) fn render_activity_bar(&mut self, ctx: &egui::Context) {
        if !self.panel_visibility.show_activity_bar {
            return;
        }
        egui::SidePanel::left("activity_bar")
            .exact_width(42.0)
            .frame(egui::Frame::new().fill(self.theme.activitybar_bg))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    self.activity_button(ui, IconId::Explorer, ActivityView::Explorer);
                    self.activity_button(ui, IconId::Search, ActivityView::Search);
                    self.activity_button(ui, IconId::SourceControl, ActivityView::SourceControl);
                    self.activity_button(ui, IconId::Run, ActivityView::Run);
                    self.activity_button(ui, IconId::Extensions, ActivityView::Extensions);
                });
            });
    }

    fn activity_button(&mut self, ui: &mut egui::Ui, icon_id: IconId, view: ActivityView) {
        let selected = self.panel_visibility.activity_view == view;
        let resp = icon_button(ui, icon_id, selected, self.theme.text_primary, 28.0);
        if resp.clicked() {
            if selected {
                self.panel_visibility.show_primary_sidebar =
                    !self.panel_visibility.show_primary_sidebar;
            } else {
                self.panel_visibility.activity_view = view;
                self.panel_visibility.show_primary_sidebar = true;
            }
        }
    }
}
