use efame::egui::{self, Align, Layout, RichText};

use crate::ui::icons::{IconId, icon, icon_button};
use crate::ui::theme::ThemeTokens;
use crate::workbench::{DocumentId, DocumentKind, WorkbenchModel};

pub enum TabAction {
    Activate(DocumentId),
    Close(DocumentId),
}

pub fn render_tabstrip(
    ui: &mut egui::Ui,
    model: &WorkbenchModel,
    tokens: &ThemeTokens,
    active_tab: Option<DocumentId>,
) -> Vec<TabAction> {
    let mut actions = Vec::new();
    let tabs: Vec<_> = model
        .documents_in_tab_order()
        .map(|d| (d.id, d.title.clone(), d.dirty, d.kind_id(), &d.kind))
        .collect();

    ui.horizontal_wrapped(|ui| {
        for (id, title, dirty, _kind_id, kind) in tabs {
            let selected = active_tab == Some(id);
            let bg = if selected {
                tokens.tab_active_bg
            } else {
                tokens.tab_inactive_bg
            };
            egui::Frame::new()
                .fill(bg)
                .stroke(egui::Stroke::new(1.0, tokens.border_default))
                .show(ui, |ui| {
                    ui.set_height(28.0);
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        let icon_id = match kind {
                            DocumentKind::Board(_) => IconId::PcbDoc,
                            DocumentKind::Spec(_) => IconId::Spec,
                            DocumentKind::Keybindings => IconId::Gear,
                        };
                        icon(ui, icon_id, tokens.text_muted, 14.0);
                        if dirty {
                            ui.label(RichText::new("●").color(tokens.accent_blue));
                        }
                        let resp =
                            ui.selectable_label(selected, RichText::new(title).color(tokens.text_primary));
                        if resp.clicked() {
                            actions.push(TabAction::Activate(id));
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let close = icon_button(ui, IconId::Close, false, tokens.text_muted, 16.0);
                            if close.clicked() {
                                actions.push(TabAction::Close(id));
                            }
                        });
                    });
                });
        }
    });
    ui.separator();
    actions
}
