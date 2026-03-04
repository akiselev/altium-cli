use efame::egui::{self, Color32};

use crate::ui::list::{FilteredListState, ListRow, filtered_list};
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
    pub active_index: Option<usize>,
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
    let width = (ctx.content_rect().width() * 0.52).clamp(560.0, 860.0);
    let max_list_h = (ctx.content_rect().height() * 0.58).max(240.0);
    let mut state = FilteredListState {
        filter: std::mem::take(filter),
        selected: *selected,
        focus_pending: *focus_pending,
    };
    let mut submitted_index: Option<usize> = None;
    let mut hovered_index: Option<usize> = None;
    let mut active_index: Option<usize> = None;
    let list_rows: Vec<ListRow<usize>> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| ListRow {
            value: idx,
            title: item.title.clone(),
            subtitle: Some(item.subtitle.clone()),
            icon: None,
            swatch: item.swatch,
        })
        .collect();

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
                        let result = filtered_list(
                            ui,
                            ctx,
                            input_id,
                            hint,
                            tokens,
                            &mut state,
                            &list_rows,
                            Some(max_list_h),
                        );
                        submitted_index = result.submitted;
                        hovered_index = result.hovered;
                        active_index = result.active;
                    });
                });
        });
    *filter = state.filter;
    *selected = state.selected;
    *focus_pending = state.focus_pending;

    PaletteResult {
        submitted_index,
        hovered_index,
        active_index,
    }
}
