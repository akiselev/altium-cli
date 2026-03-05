use efame::egui::{self, Align, Layout, RichText, Stroke, Vec2};

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

    egui::Frame::new()
        .fill(tokens.tab_inactive_bg)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.horizontal(|ui| {
                for (id, title, dirty, _kind_id, kind) in tabs {
                    let selected = active_tab == Some(id);
                    let tab_bg = if selected {
                        tokens.tab_active_bg
                    } else if ui.rect_contains_pointer(ui.max_rect()) {
                        tokens.tab_hover_bg
                    } else {
                        tokens.tab_inactive_bg
                    };

                    let frame = egui::Frame::new()
                        .fill(tab_bg)
                        .stroke(Stroke::new(1.0, tokens.border_default))
                        .inner_margin(egui::Margin::symmetric(8, 0));
                    let inner = frame.show(ui, |ui| {
                        ui.set_height(32.0);
                        ui.set_min_width(150.0);
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            let icon_id = match kind {
                                DocumentKind::Board(_) => IconId::PcbDoc,
                                DocumentKind::Spec(_) => IconId::Spec,
                                DocumentKind::SchDocPreview(_)
                                | DocumentKind::SchLibGallery(_)
                                | DocumentKind::SchLibComponent(_)
                                | DocumentKind::Logical(_)
                                | DocumentKind::DefinitionCollection(_)
                                | DocumentKind::Import(_)
                                | DocumentKind::DesignOverview(_) => IconId::Spec,
                                DocumentKind::Physical(_) => IconId::PcbDoc,
                                DocumentKind::Asset(_) => IconId::Gear,
                                DocumentKind::Keybindings => IconId::Gear,
                            };
                            icon(ui, icon_id, tokens.text_muted, 14.0);
                            if dirty {
                                ui.label(RichText::new("●").color(tokens.accent_blue));
                            }
                            let text = if selected {
                                RichText::new(title.clone()).color(tokens.text_primary)
                            } else {
                                RichText::new(title.clone()).color(tokens.text_muted)
                            };
                            ui.label(text);
                            ui.add_space(8.0);
                            if selected || ui.rect_contains_pointer(ui.max_rect()) {
                                let close =
                                    icon_button(ui, IconId::Close, false, tokens.text_muted, 14.0);
                                if close.clicked() {
                                    actions.push(TabAction::Close(id));
                                }
                            } else {
                                ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
                            }
                        });
                    });

                    if selected {
                        let rect = inner.response.rect;
                        ui.painter().line_segment(
                            [
                                rect.left_top() + egui::vec2(0.0, 1.0),
                                rect.right_top() + egui::vec2(0.0, 1.0),
                            ],
                            Stroke::new(2.0, tokens.accent_blue),
                        );
                    }

                    if inner.response.clicked() {
                        actions.push(TabAction::Activate(id));
                    }
                }
                ui.add_space(2.0);
            });
        });
    ui.painter().line_segment(
        [
            ui.min_rect().left_bottom() + egui::vec2(0.0, -1.0),
            ui.min_rect().right_bottom() + egui::vec2(0.0, -1.0),
        ],
        Stroke::new(1.0, tokens.border_default),
    );
    ui.add_space(2.0);
    actions
}
