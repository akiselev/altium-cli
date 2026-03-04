use efame::egui::{self, Color32};

use crate::ui::theme::ThemeTokens;

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub title: String,
    pub subtitle: String,
    pub swatch: Option<Color32>,
}

#[derive(Debug, Clone, Copy)]
pub struct PaletteResult {
    pub submitted_index: Option<usize>,
    pub hovered_index: Option<usize>,
}

pub fn show_palette_overlay(
    ctx: &egui::Context,
    overlay_id: &str,
    input_id: &str,
    title: &str,
    hint: &str,
    tokens: &ThemeTokens,
    filter: &mut String,
    selected: &mut usize,
    focus_pending: &mut bool,
    items: &[PaletteItem],
) -> PaletteResult {
    if !items.is_empty() {
        *selected = (*selected).min(items.len() - 1);
    } else {
        *selected = 0;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !items.is_empty() {
        *selected = (*selected + 1) % items.len();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !items.is_empty() {
        *selected = if *selected == 0 {
            items.len() - 1
        } else {
            *selected - 1
        };
    }

    let width = (ctx.content_rect().width() * 0.52).clamp(560.0, 860.0);
    let max_list_h = (ctx.content_rect().height() * 0.58).max(240.0);

    let mut submitted_index = None;
    let mut hovered_index = None;

    egui::Area::new(egui::Id::new(overlay_id))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 42.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(tokens.sidebar_bg)
                .stroke(egui::Stroke::new(1.0, tokens.border_focus))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_min_width(width);
                    ui.set_max_width(width);

                    ui.label(
                        egui::RichText::new(title)
                            .small()
                            .strong()
                            .color(tokens.text_muted),
                    );
                    ui.add_space(4.0);

                    let input_frame = egui::Frame::new()
                        .fill(tokens.window_bg)
                        .stroke(egui::Stroke::new(1.0, tokens.border_default))
                        .corner_radius(egui::CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(8, 6));
                    input_frame.show(ui, |ui| {
                        let input_id = ui.make_persistent_id(input_id);
                        if *focus_pending {
                            ui.memory_mut(|m| m.request_focus(input_id));
                            *focus_pending = false;
                        }
                        let edit = egui::TextEdit::singleline(filter)
                            .hint_text(hint)
                            .id(input_id);
                        let resp = ui.add(edit);
                        if resp.changed() {
                            *selected = 0;
                        }
                    });

                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .max_height(max_list_h)
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            if items.is_empty() {
                                ui.colored_label(tokens.text_muted, "No matching items");
                                return;
                            }

                            for (idx, item) in items.iter().enumerate() {
                                let is_selected = idx == *selected;
                                let row_frame = egui::Frame::new()
                                    .fill(if is_selected {
                                        tokens.accent_blue.gamma_multiply(0.32)
                                    } else {
                                        tokens.window_bg
                                    })
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        if is_selected {
                                            tokens.border_focus
                                        } else {
                                            tokens.border_default.gamma_multiply(0.55)
                                        },
                                    ))
                                    .corner_radius(egui::CornerRadius::same(4))
                                    .inner_margin(egui::Margin::symmetric(8, 6));

                                let row = row_frame.show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        if let Some(swatch) = item.swatch {
                                            let (r, _) = ui.allocate_exact_size(
                                                egui::vec2(14.0, 14.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(
                                                r,
                                                egui::CornerRadius::same(2),
                                                swatch,
                                            );
                                            ui.add_space(6.0);
                                        }
                                        ui.vertical(|ui| {
                                            let title = if is_selected {
                                                egui::RichText::new(&item.title)
                                                    .strong()
                                                    .color(egui::Color32::WHITE)
                                            } else {
                                                egui::RichText::new(&item.title)
                                                    .strong()
                                                    .color(tokens.text_primary)
                                            };
                                            ui.label(title);
                                            let subtitle = if is_selected {
                                                egui::RichText::new(&item.subtitle)
                                                    .small()
                                                    .color(egui::Color32::from_gray(220))
                                            } else {
                                                egui::RichText::new(&item.subtitle)
                                                    .small()
                                                    .color(tokens.text_muted)
                                            };
                                            ui.label(subtitle);
                                        });
                                    });
                                });

                                let resp = row.response.interact(egui::Sense::click());
                                if resp.clicked() {
                                    submitted_index = Some(idx);
                                }
                                if resp.hovered() {
                                    *selected = idx;
                                    hovered_index = Some(idx);
                                }
                                ui.add_space(4.0);
                            }
                        });
                });
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !items.is_empty() {
        submitted_index = Some(*selected);
    }

    PaletteResult {
        submitted_index,
        hovered_index,
    }
}
