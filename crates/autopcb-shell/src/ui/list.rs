use efame::egui::{self, Color32, RichText};

use crate::ui::icons::{IconId, icon};
use crate::ui::theme::ThemeTokens;
use crate::ui::theme_primitives::list_tokens;

#[derive(Debug, Clone)]
pub struct ListRow<T> {
    pub value: T,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<IconId>,
    pub swatch: Option<Color32>,
}

#[derive(Debug, Clone)]
pub struct FilteredListState {
    pub filter: String,
    pub selected: usize,
    pub focus_pending: bool,
}

impl Default for FilteredListState {
    fn default() -> Self {
        Self {
            filter: String::new(),
            selected: 0,
            focus_pending: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FilteredListResult<T> {
    pub submitted: Option<T>,
    pub active: Option<T>,
    pub hovered: Option<T>,
}

pub fn filtered_list<T: Copy>(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    input_id: &str,
    hint: &str,
    theme: &ThemeTokens,
    state: &mut FilteredListState,
    rows: &[ListRow<T>],
    max_height: Option<f32>,
) -> FilteredListResult<T> {
    if !rows.is_empty() {
        state.selected = state.selected.min(rows.len() - 1);
    } else {
        state.selected = 0;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !rows.is_empty() {
        state.selected = (state.selected + 1) % rows.len();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !rows.is_empty() {
        state.selected = if state.selected == 0 {
            rows.len() - 1
        } else {
            state.selected - 1
        };
    }

    let t = list_tokens(theme);
    let mut submitted = None;
    let mut hovered = None;

    let input_id = ui.make_persistent_id(input_id);
    let resp = ui.add(
        egui::TextEdit::singleline(&mut state.filter)
            .hint_text(hint)
            .id(input_id),
    );
    if state.focus_pending {
        resp.request_focus();
        if resp.has_focus() {
            state.focus_pending = false;
        }
    }
    if resp.changed() {
        state.selected = 0;
    }

    ui.add_space(6.0);
    let scroll = egui::ScrollArea::vertical().auto_shrink([false; 2]);
    let scroll = if let Some(h) = max_height {
        scroll.max_height(h)
    } else {
        scroll
    };
    scroll.show(ui, |ui| {
        if rows.is_empty() {
            ui.colored_label(t.muted, "No matching items");
            return;
        }

        for (idx, row) in rows.iter().enumerate() {
            let is_selected = idx == state.selected;
            let frame = egui::Frame::new()
                .fill(if is_selected {
                    t.row_selected_bg
                } else {
                    t.row_bg
                })
                .stroke(egui::Stroke::new(
                    1.0,
                    if is_selected {
                        t.row_selected_border
                    } else {
                        t.row_border
                    },
                ))
                .corner_radius(egui::CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(8, 6));
            let inner = frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if let Some(icon_id) = row.icon {
                        icon(ui, icon_id, t.muted, 14.0);
                        ui.add_space(6.0);
                    }
                    if let Some(swatch) = row.swatch {
                        let (r, _) =
                            ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(r, egui::CornerRadius::same(2), swatch);
                        ui.add_space(6.0);
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&row.title).strong().color(if is_selected {
                            Color32::WHITE
                        } else {
                            t.text
                        }));
                        if let Some(subtitle) = &row.subtitle {
                            ui.label(RichText::new(subtitle).small().color(if is_selected {
                                Color32::from_gray(220)
                            } else {
                                t.muted
                            }));
                        }
                    });
                });
            });
            let row_resp = inner.response.interact(egui::Sense::click());
            if row_resp.clicked() {
                submitted = Some(row.value);
            }
            if row_resp.hovered() {
                state.selected = idx;
                hovered = Some(row.value);
            }
            ui.add_space(4.0);
        }
    });

    if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !rows.is_empty() {
        submitted = rows.get(state.selected).map(|r| r.value);
    }

    FilteredListResult {
        submitted,
        active: rows.get(state.selected).map(|r| r.value),
        hovered,
    }
}
